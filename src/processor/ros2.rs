use std::borrow::ToOwned;
use std::collections::hash_map::Entry;
use std::sync::{Arc, Mutex};

use crate::events_common::Context;
use crate::model::{
    Callback, CallbackCaller, CallbackInstance, Client, Node, PublicationMessage, Publisher,
    Service, Subscriber, SubscriptionMessage, Time, Timer,
};
use crate::utils::Known;
use crate::{processed_events, raw_events};

use super::{error, ContextId, IntoId, MapGetAsResult, Processor};

use color_eyre::eyre::{eyre, Context as _};
use color_eyre::Result;

// Event processing methods
impl Processor {
    pub(super) fn process_rcl_node_init(
        &mut self,
        event: &raw_events::ros2::RclNodeInit,
        _time: Time,
        context_id: ContextId,
        _context: &Context,
    ) -> processed_events::ros2::RclNodeInit {
        let node_arc = match self
            .nodes_by_rcl
            .entry(event.node_handle.into_id(context_id))
        {
            Entry::Occupied(entry) => {
                let mut node = entry.get().lock().unwrap();
                node.rcl_init(event.rmw_handle, &event.node_name, &event.namespace);
                entry.get().clone()
            }
            Entry::Vacant(entry) => {
                let mut node = Node::new(event.rmw_handle);
                node.rcl_init(event.rmw_handle, &event.node_name, &event.namespace);
                let node_arc = Arc::new(Mutex::new(node));
                entry.insert(node_arc.clone());
                node_arc
            }
        };

        processed_events::ros2::RclNodeInit { node: node_arc }
    }

    pub(super) fn process_rmw_publisher_init(
        &mut self,
        event: &raw_events::ros2::RmwPublisherInit,
        _time: Time,
        context_id: ContextId,
        _context: &Context,
    ) -> processed_events::ros2::RmwPublisherInit {
        let publisher_arc = self
            .publishers_by_rmw
            .entry(event.rmw_publisher_handle.into_id(context_id))
            .or_default();

        let init_result = publisher_arc
            .lock()
            .unwrap()
            .rmw_init(event.rmw_publisher_handle, event.gid);

        if let Err(_e) = init_result {
            log::warn!(
                target: "rmw_publisher_init",
                "Repeated initialization for handle {}. Assuming old publisher was deleted. Creating new.",
                event.rmw_publisher_handle
            );
            let mut publisher = publisher_arc.lock().unwrap();
            publisher.mark_removed();
            drop(publisher);

            let mut publisher = Publisher::default();
            publisher
                .rmw_init(event.rmw_publisher_handle, event.gid)
                .expect("New Publisher should not be initialized yet");
            *publisher_arc = Arc::new(Mutex::new(publisher));
        }

        processed_events::ros2::RmwPublisherInit {
            publisher: publisher_arc.clone(),
        }
    }

    pub(super) fn process_rcl_publisher_init(
        &mut self,
        event: &raw_events::ros2::RclPublisherInit,
        _time: Time,
        context_id: ContextId,
        _context: &Context,
    ) -> processed_events::ros2::RclPublisherInit {
        let publisher_by_rmw_arc = self
            .publishers_by_rmw
            .entry(event.rmw_publisher_handle.into_id(context_id))
            .or_insert_with_key(|key| {
                let mut publisher = Publisher::default();
                publisher
                    .set_rmw_handle(key.id)
                    .expect("New Publisher should not be initialized yet");
                Arc::new(Mutex::new(publisher))
            });

        let node_arc = self
            .nodes_by_rcl
            .entry(event.node_handle.into_id(context_id))
            .or_insert_with(|| {
                // Only /rosout publishers are allowed to be created before node.
                // This happens in ROS2 Iron and lower versions.
                assert_eq!(
                    event.topic_name, "/rosout",
                    "Node not found for publisher: {event:?}"
                );
                let node = Node::new(event.node_handle);
                Arc::new(Mutex::new(node))
            });

        let init_result = publisher_by_rmw_arc.lock().unwrap().rcl_init(
            event.publisher_handle,
            event.topic_name.clone(),
            event.queue_depth,
            Arc::downgrade(node_arc),
        );

        let publisher_arc = if let Err(_e) = init_result {
            log::warn!(
                target: "rcl_publisher_init",
                "Repeated initialization for handle {}. Assuming old publisher was deleted. Creating new.",
                event.publisher_handle
            );
            let mut old_publisher = publisher_by_rmw_arc.lock().unwrap();
            old_publisher.mark_removed();
            drop(old_publisher);

            let mut publisher = Publisher::default();
            publisher
                .rcl_init(
                    event.publisher_handle,
                    event.topic_name.clone(),
                    event.queue_depth,
                    Arc::downgrade(node_arc),
                )
                .expect("New Publisher should not be initialized yet");
            let publisher_arc = Arc::new(Mutex::new(publisher));
            *publisher_by_rmw_arc = publisher_arc.clone();

            publisher_arc
        } else {
            publisher_by_rmw_arc.clone()
        };

        self.publishers_by_rcl
            .insert(event.publisher_handle.into_id(context_id), publisher_arc.clone()).inspect(|old| {
                log::warn!(
                    target: "rcl_publisher_init",
                    "Found different Publisher with same address. Assuming old Publisher was deleted: old={old:?}"
                );
                old.lock().unwrap().mark_removed();
            });

        node_arc
            .lock()
            .unwrap()
            .add_publisher(publisher_arc.clone());

        processed_events::ros2::RclPublisherInit {
            publisher: publisher_arc.clone(),
        }
    }

    pub(super) fn process_rclcpp_publish(
        &mut self,
        event: &raw_events::ros2::RclcppPublish,
        time: Time,
        context_id: ContextId,
        _context: &Context,
    ) -> processed_events::ros2::RclcppPublish {
        let mut message = PublicationMessage::new(event.message);
        message.rclcpp_publish(time);
        let message_arc = Arc::new(Mutex::new(message));
        self.published_messages_by_rclcpp
            .insert(event.message.into_id(context_id), message_arc.clone());

        processed_events::ros2::RclcppPublish {
            message: message_arc,
        }
    }

    pub(super) fn process_rcl_publish(
        &mut self,
        event: &raw_events::ros2::RclPublish,
        time: Time,
        context_id: ContextId,
        _context: &Context,
    ) -> processed_events::ros2::RclPublish {
        let id = event.message.into_id(context_id);
        let message_arc = self
            .published_messages_by_rclcpp
            .remove(&id)
            .unwrap_or_else(|| {
                let message = PublicationMessage::new(event.message);
                Arc::new(Mutex::new(message))
            });
        self.published_messages_by_rcl
            .insert(id, message_arc.clone());

        let publisher = self
            .publishers_by_rcl
            .entry(event.publisher_handle.into_id(context_id))
            .or_insert_with_key(|key| {
                // TODO: rosout publisher can change handle address
                // For now we just create a new publisher
                let mut publisher = Publisher::default();
                publisher
                    .set_rcl_handle(key.id)
                    .expect("New Publisher should not be initialized yet");

                Arc::new(Mutex::new(publisher))
            })
            .clone();

        {
            let mut message = message_arc.lock().unwrap();
            message.set_publisher(publisher);
            message.rcl_publish(time);
        }

        processed_events::ros2::RclPublish {
            message: message_arc,
        }
    }

    pub(super) fn process_rmw_publish(
        &mut self,
        event: &raw_events::ros2::RmwPublish,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> processed_events::ros2::RmwPublish {
        let message_arc = self
            .published_messages_by_rcl
            .remove(&event.message.into_id(context_id))
            .unwrap_or_else(|| Arc::new(Mutex::new(PublicationMessage::new(event.message))));
        let mut message = message_arc.lock().unwrap();
        let publisher_id = event.rmw_publisher_handle.map(|h| h.into_id(context_id));
        let publisher_arc = publisher_id.and_then(|id| self.publishers_by_rmw.get(&id));
        let topic: Known<String> = match (publisher_arc, message.get_publisher()) {
            (Some(publisher_arc), Some(message_publisher_arc)) => {
                let message_publisher = message_publisher_arc.lock().unwrap();
                if Arc::ptr_eq(publisher_arc, &message_publisher_arc) {
                    message_publisher.get_topic().map(ToOwned::to_owned)
                } else if message_publisher.is_stub() {
                    message.replace_publisher(publisher_arc.clone());
                    let mut publisher = publisher_arc.lock().unwrap();
                    self.publishers_by_rcl.insert(
                        message_publisher
                            .get_rcl_handle()
                            .unwrap()
                            .into_id(context_id),
                        publisher_arc.clone(),
                    );
                    publisher.change_rcl_handle(message_publisher.get_rcl_handle().unwrap());
                    publisher.get_topic().map(ToOwned::to_owned)
                } else {
                    log::warn!(target: "rmw_publish",
                        "Publisher mismatch for message. [{time}] {event:?} {context:?}"
                    );
                    Known::Unknown
                }
            }
            (Some(publisher_arc), None) => {
                message.set_publisher(publisher_arc.clone());
                publisher_arc
                    .lock()
                    .unwrap()
                    .get_topic()
                    .map(ToOwned::to_owned)
            }
            (None, Some(message_publisher_arc)) => message_publisher_arc
                .lock()
                .unwrap()
                .get_topic()
                .map(ToOwned::to_owned),
            (None, None) => Known::Unknown,
        };

        if let Some(timestamp) = event.timestamp {
            message.rmw_publish(time, timestamp);

            self.published_messages
                        .insert((timestamp, topic), message_arc.clone())
                        .inspect(|old| {
                            log::warn!(
                                target: "rmw_publish",
                                "Replacing different PublicationMessage with same sender timestamp. old_message={old:?}"
                            );
                        });
        } else {
            log::warn!(target: "rmw_publish",
                        "Missing timestamp for RMW publish event. Subscription messages will not match it: [{time}] {event:?} {context:?}");

            message.rmw_publish_old(time);
        }
        drop(message);

        processed_events::ros2::RmwPublish {
            message: message_arc,
        }
    }

    pub(super) fn process_rclcpp_intra_publish(
        &mut self,
        event: &raw_events::ros2::RclcppIntraPublish,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::ros2::RclcppIntraPublish> {
        let message_arc = self
            .published_messages_by_rclcpp
            .get(&event.message.into_id(context_id))
            .ok_or(error::NotFound::published_message(event.message))
            .map_err(|e| e.with_ros2_event(event, time, context))?
            .clone();

        // TODO: check if event has rcl handle, rclcpp handle or other.
        let publisher_arc = self
            .publishers_by_rcl
            .get_or_err(event.publisher_handle.into_id(context_id), "rcl_handle")
            .map_err(|e| e.with_ros2_event(event, time, context))?
            .clone();

        message_arc.lock().unwrap().set_publisher(publisher_arc);

        Ok(processed_events::ros2::RclcppIntraPublish {
            message: message_arc,
        })
    }

    pub(super) fn process_rmw_subscription_init(
        &mut self,
        event: &raw_events::ros2::RmwSubscriptionInit,
        _time: Time,
        context_id: ContextId,
        _context: &Context,
    ) -> processed_events::ros2::RmwSubscriptionInit {
        let subscriber_arc = self
            .subscribers_by_rmw
            .entry(event.rmw_subscription_handle.into_id(context_id))
            .or_default();

        let init_result = subscriber_arc
            .lock()
            .unwrap()
            .rmw_init(event.rmw_subscription_handle, event.gid);

        if let Err(_e) = init_result {
            log::warn!(
                target: "rmw_subscription_init",
                "Repeated initialization for handle {}. Assuming old subscriber was deleted. Creating new.",
                event.rmw_subscription_handle
            );
            let mut old_subscriber = subscriber_arc.lock().unwrap();
            old_subscriber.mark_removed();
            drop(old_subscriber);

            let mut subscriber = Subscriber::default();
            subscriber
                .rmw_init(event.rmw_subscription_handle, event.gid)
                .expect("New Subscriber should not be initialized yet");
            *subscriber_arc = Arc::new(Mutex::new(subscriber));
        }

        processed_events::ros2::RmwSubscriptionInit {
            subscription: subscriber_arc.clone(),
        }
    }

    pub(super) fn process_rcl_subscription_init(
        &mut self,
        event: &raw_events::ros2::RclSubscriptionInit,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::ros2::RclSubscriptionInit> {
        let subscriber_arc = self
            .subscribers_by_rmw
            .entry(event.rmw_subscription_handle.into_id(context_id))
            .or_insert_with_key(|key| {
                let mut subscriber = Subscriber::default();
                subscriber
                    .set_rmw_handle(key.id)
                    .expect("New Subscriber should not have rwm handle yet");
                Arc::new(Mutex::new(subscriber))
            });

        let node_arc = self
            .nodes_by_rcl
            .get_or_err(event.node_handle.into_id(context_id), "rcl_handle")
            .map_err(|e| e.dependent_object(&*subscriber_arc))
            .map_err(|e| e.with_ros2_event(event, time, context))?
            .clone();

        let init_result = subscriber_arc.lock().unwrap().rcl_init(
            event.subscription_handle,
            event.topic_name.clone(),
            event.queue_depth,
            Arc::downgrade(&node_arc),
        );

        if let Err(_e) = init_result {
            log::warn!(
                target: "rcl_subscription_init",
                "Repeated initialization for handle {}. Assuming old subscriber was deleted. Creating new.",
                event.subscription_handle
            );
            let mut old_subscriber = subscriber_arc.lock().unwrap();
            old_subscriber.mark_removed();
            drop(old_subscriber);

            let mut subscriber = Subscriber::default();
            subscriber
                .set_rmw_handle(event.rmw_subscription_handle)
                .expect("New Subscriber should not have rwm handle yet");
            subscriber
                .rcl_init(
                    event.subscription_handle,
                    event.topic_name.clone(),
                    event.queue_depth,
                    Arc::downgrade(&node_arc),
                )
                .expect("New Subscriber should not be initialized yet");
            *subscriber_arc = Arc::new(Mutex::new(subscriber));
        }

        self.subscribers_by_rcl
            .insert(event.subscription_handle.into_id(context_id), subscriber_arc.clone())
            .inspect(|old| {
                log::warn!(
                    target: "rcl_subscription_init",
                    "Found different Subscriber with same address. Assuming old Subscriber was deleted: old={old:?}"
                );
                old.lock().unwrap().mark_removed();
            });

        node_arc
            .lock()
            .unwrap()
            .add_subscriber(subscriber_arc.clone());

        Ok(processed_events::ros2::RclSubscriptionInit {
            subscription: subscriber_arc.clone(),
        })
    }

    pub(super) fn process_rclcpp_subscription_init(
        &mut self,
        event: &raw_events::ros2::RclcppSubscriptionInit,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::ros2::RclcppSubscriptionInit> {
        let subscriber_arc = self
            .subscribers_by_rcl
            .get_or_err(event.subscription_handle.into_id(context_id), "rcl_handle")
            .map_err(|e| e.with_ros2_event(event, time, context))?;

        subscriber_arc
            .lock()
            .unwrap()
            .rclcpp_init(event.subscription)
            .map_err(|e| error::Causes::AlreadyInitialized(e, subscriber_arc.clone().into()))?;

        self.subscribers_by_rclcpp
            .insert(
                event.subscription.into_id(context_id),
                subscriber_arc.clone(),
            )
            .inspect(|old| {
                log::warn!(
                    target: "rclcpp_subscription_init",
                    "Found different Subscriber with same address. Assuming old Subscriber was deleted: old={old:?}"
                );
                old.lock().unwrap().mark_removed();
            });

        Ok(processed_events::ros2::RclcppSubscriptionInit {
            subscription: subscriber_arc.clone(),
        })
    }

    pub(super) fn process_rclcpp_subscription_callback_added(
        &mut self,
        event: &raw_events::ros2::RclcppSubscriptionCallbackAdded,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::ros2::RclcppSubscriptionCallbackAdded> {
        let subscription_arc = self
            .subscribers_by_rclcpp
            .get_or_err(event.subscription.into_id(context_id), "rclcpp_handle")
            .map_err(|e| e.with_ros2_event(event, time, context))?
            .clone();

        let callback_arc = Callback::new_subscription(
            event.callback,
            &subscription_arc,
            context.hostname().to_owned(),
        );

        self.callbacks_by_id
            .insert(event.callback.into_id(context_id), callback_arc.clone())
            .and_then(filter_out_removed_callers)
            .map_or(Ok(()), |old: Arc<Mutex<Callback>>| {
                Err(
                    error::AlreadyExists::with_id(event.callback, &callback_arc, old)
                        .with_ros2_event(event, time, context),
                )
            })?;

        subscription_arc
            .lock()
            .unwrap()
            .set_callback(callback_arc.clone())
            .map_err(|e| {
                eyre!("Subscription was already initialized by rclcpp_subscription_init event: {e}")
            })?;

        Ok(processed_events::ros2::RclcppSubscriptionCallbackAdded {
            callback: callback_arc,
        })
    }

    pub(super) fn process_rmw_take(
        &mut self,
        event: &raw_events::ros2::RmwTake,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::ros2::RmwTake> {
        let subscriber = self
            .subscribers_by_rmw
            .get_or_err(
                event.rmw_subscription_handle.into_id(context_id),
                "rmw_handle",
            )
            .map_err(|e| e.with_ros2_event(event, time, context))
            .wrap_err("Taken message missing subscriber.")?;
        let topic = subscriber
            .lock()
            .unwrap()
            .get_topic()
            .map(ToOwned::to_owned);

        let mut message = SubscriptionMessage::new(event.message);

        if let Some(published_message) =
            (event.source_timestamp != 0).then_some(()).and_then(|()| {
                self.published_messages
                    .get(&(event.source_timestamp, topic))
                    .or_else(|| {
                        self.published_messages
                            .get(&(event.source_timestamp, Known::Unknown))
                    })
            })
        {
            message.rmw_take_matched(subscriber.clone(), published_message.clone(), time);
        } else {
            if event.source_timestamp == 0 {
                log::info!(target:"rmw_take", "Missing source timestamp. [{time}] {event:?} {context:?}");
            }
            message.rmw_take_unmatched(subscriber.clone(), event.source_timestamp, time);
        }

        let message_arc = Arc::new(Mutex::new(message));

        if event.taken {
            let mut subscriber = subscriber.lock().unwrap();
            if let Some(_old) = subscriber.replace_taken_message(message_arc.clone()) {
                // TODO: Save message to dropped messages
            }
            drop(subscriber);

            // Override the old message with the new one
            // TODO: Save old message to processed messages if needed
            self.received_messages
                .insert(event.message.into_id(context_id), message_arc.clone());
        }

        Ok(processed_events::ros2::RmwTake {
            message: message_arc,
            taken: event.taken,
        })
    }

    pub(super) fn process_rcl_take(
        &mut self,
        event: &raw_events::ros2::RclTake,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> processed_events::ros2::RclTake {
        let message_arc = self
            .received_messages
            .get(&event.message.into_id(context_id))
            .and_then(|message_arc| {
                let mut message = message_arc.lock().unwrap();
                message.rcl_take(time).ok().map(|()| message_arc)
            });

        let is_new = message_arc.is_none();

        let message_arc = if let Some(message_arc) = message_arc {
            message_arc.clone()
        } else {
            let mut message = SubscriptionMessage::new(event.message);
            message
                .rcl_take(time)
                .expect("The message was just created, rcl_take was not called before.");
            let message_arc = Arc::new(Mutex::new(message));

            log::warn!(target:"rcl_take",
                "Message was not taken before. Creating new message. [{time}] {event:?} {context:?}"
            );

            self.received_messages
                .insert(event.message.into_id(context_id), message_arc.clone());

            message_arc
        };

        processed_events::ros2::RclTake {
            message: message_arc,
            is_new,
        }
    }

    pub(super) fn process_rclcpp_take(
        &mut self,
        event: &raw_events::ros2::RclcppTake,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> processed_events::ros2::RclCppTake {
        let message_arc = self
            .received_messages
            .get(&event.message.into_id(context_id))
            .and_then(|message_arc| {
                let mut message = message_arc.lock().unwrap();
                message.rclcpp_take(time).ok().map(|()| message_arc)
            });

        let is_new = message_arc.is_none();

        let message_arc = if let Some(message_arc) = message_arc {
            message_arc.clone()
        } else {
            let mut message = SubscriptionMessage::new(event.message);
            message
                .rclcpp_take(time)
                .expect("The message was just created, rclcpp_take was not called before.");
            let message_arc = Arc::new(Mutex::new(message));

            log::warn!("rclcpp_take: Message was not taken before. Creating new message. [{time}] {event:?} {context:?}");

            self.received_messages
                .insert(event.message.into_id(context_id), message_arc.clone());

            message_arc
        };

        processed_events::ros2::RclCppTake {
            message: message_arc.clone(),
            is_new,
        }
    }

    pub(super) fn process_rcl_service_init(
        &mut self,
        event: &raw_events::ros2::RclServiceInit,
        _time: Time,
        context_id: ContextId,
        _context: &Context,
    ) -> Result<processed_events::ros2::RclServiceInit> {
        let service_arc = self
            .services_by_rcl
            .entry(event.service_handle.into_id(context_id))
            .or_insert_with_key(|key| {
                let service = Service::new(key.id);
                Arc::new(Mutex::new(service))
            });

        let node_arc = self
            .nodes_by_rcl
            .get_or_err(event.node_handle.into_id(context_id), "rcl_handle")
            .map_err(|e| e.dependent_object(&*service_arc))?;

        let init_result = service_arc.lock().unwrap().rcl_init(
            event.rmw_service_handle,
            event.service_name.clone(),
            node_arc,
        );
        if let Err(_e) = init_result {
            log::warn!(
                target: "rcl_service_init",
                "Repeated initialization for handle {}. Assuming old service was deleted. Creating new.",
                event.service_handle
            );
            let mut service = service_arc.lock().unwrap();
            service.mark_removed();
            drop(service);

            let mut service = Service::new(event.service_handle.into_id(context_id).id);
            service
                .rcl_init(
                    event.rmw_service_handle,
                    event.service_name.clone(),
                    node_arc,
                )
                .expect("New Service should not be initialized yet");
            *service_arc = Arc::new(Mutex::new(service));
        }

        node_arc.lock().unwrap().add_service(service_arc.clone());

        Ok(processed_events::ros2::RclServiceInit {
            service: service_arc.clone(),
        })
    }

    pub(super) fn process_rclcpp_service_callback_added(
        &mut self,
        event: &raw_events::ros2::RclcppServiceCallbackAdded,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::ros2::RclCppServiceCallbackAdded> {
        let service_arc = self
            .services_by_rcl
            .entry(event.service_handle.into_id(context_id))
            .or_insert_with_key(|key| {
                log::warn!("Service not found for callback. Creating new (possibly duplicate) service. Event: [{time}] {event:?} {context:?}");
                let service = Service::new(key.id);
                Arc::new(Mutex::new(service))
            });

        let callback_arc =
            Callback::new_service(event.callback, service_arc, context.hostname().to_owned());

        self.callbacks_by_id
            .insert(event.callback.into_id(context_id), callback_arc.clone())
            .and_then(filter_out_removed_callers)
            .map_or(Ok(()), |old: Arc<Mutex<Callback>>| {
                Err(
                    error::AlreadyExists::with_id(event.callback, &callback_arc, old)
                        .with_ros2_event(event, time, context),
                )
            })?;

        service_arc
            .lock()
            .unwrap()
            .set_callback(callback_arc.clone())
            .map_err(|e| {
                eyre!("Service was already initialized by rclcpp_service_init event: {e}")
            })?;

        Ok(processed_events::ros2::RclCppServiceCallbackAdded {
            callback: callback_arc,
        })
    }

    pub(super) fn process_rcl_client_init(
        &mut self,
        event: &raw_events::ros2::RclClientInit,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::ros2::RclClientInit> {
        let client_arc = self
            .clients_by_rcl
            .entry(event.client_handle.into_id(context_id))
            .or_insert_with_key(|key| {
                let client = Client::new(key.id);
                Arc::new(Mutex::new(client))
            });

        let node_arc = self
            .nodes_by_rcl
            .get_or_err(event.node_handle.into_id(context_id), "rcl_handle")
            .map_err(|e| e.dependent_object(&*client_arc))
            .map_err(|e| e.with_ros2_event(event, time, context))?;

        let init_result = client_arc.lock().unwrap().rcl_init(
            event.rmw_client_handle,
            event.service_name.clone(),
            node_arc,
        );
        if let Err(_e) = init_result {
            log::warn!(
                target: "rcl_client_init",
                "Repeated initialization for handle {}. Assuming old client was deleted. Creating new.",
                event.client_handle
            );
            let mut client = client_arc.lock().unwrap();
            client.mark_removed();
            drop(client);

            let mut client = Client::new(event.client_handle.into_id(context_id).id);
            client
                .rcl_init(
                    event.rmw_client_handle,
                    event.service_name.clone(),
                    node_arc,
                )
                .expect("New Client should not be initialized yet");
            *client_arc = Arc::new(Mutex::new(client));
        }

        node_arc.lock().unwrap().add_client(client_arc.clone());

        Ok(processed_events::ros2::RclClientInit {
            client: client_arc.clone(),
        })
    }

    pub(super) fn process_rcl_timer_init(
        &mut self,
        event: &raw_events::ros2::RclTimerInit,
        _time: Time,
        context_id: ContextId,
        _context: &Context,
    ) -> processed_events::ros2::RclTimerInit {
        let timer_arc = self
            .timers_by_rcl
            .entry(event.timer_handle.into_id(context_id))
            .or_insert_with_key(|key| {
                let timer = Timer::new(key.id);
                Arc::new(Mutex::new(timer))
            });

        let init_result = timer_arc.lock().unwrap().rcl_init(event.period);
        if let Err(_e) = init_result {
            log::warn!(
                target: "rcl_timer_init",
                "Repeated initialization for handle {}. Assuming old timer was deleted. Creating new.",
                event.timer_handle
            );
            let mut timer = timer_arc.lock().unwrap();
            timer.mark_removed();
            drop(timer);

            let mut timer = Timer::new(event.timer_handle.into_id(context_id).id);
            timer
                .rcl_init(event.period)
                .expect("New Timer should not be initialized yet");
            *timer_arc = Arc::new(Mutex::new(timer));
        }

        processed_events::ros2::RclTimerInit {
            timer: timer_arc.clone(),
        }
    }

    pub(super) fn process_rclcpp_timer_callback_added(
        &mut self,
        event: &raw_events::ros2::RclcppTimerCallbackAdded,
        context_id: ContextId,
        context: &Context,
        time: Time,
    ) -> Result<processed_events::ros2::RclcppTimerCallbackAdded> {
        let timer_arc = self
            .get_timer_by_rcl_handle(event.timer_handle.into_id(context_id))
            .map_err(|e| e.with_ros2_event(event, time, context))?
            .clone();

        let callback_arc =
            Callback::new_timer(event.callback, &timer_arc, context.hostname().to_owned());

        self.callbacks_by_id
            .insert(event.callback.into_id(context_id), callback_arc.clone())
            .and_then(filter_out_removed_callers)
            .map_or(Ok(()), |old: Arc<Mutex<Callback>>| {
                Err(
                    error::AlreadyExists::with_id(event.callback, &callback_arc, old)
                        .with_ros2_event(event, time, context),
                )
            })?;

        timer_arc
            .lock()
            .unwrap()
            .set_callback(callback_arc.clone())
            .map_err(|e| eyre!("Timer was already has a callback: {e}"))?;

        Ok(processed_events::ros2::RclcppTimerCallbackAdded {
            callback: callback_arc,
        })
    }

    pub(super) fn process_rclcpp_timer_link_node(
        &mut self,
        event: &raw_events::ros2::RclcppTimerLinkNode,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::ros2::RclcppTimerLinkNode> {
        let timer_arc = self
            .get_timer_by_rcl_handle(event.timer_handle.into_id(context_id))
            .map_err(|e| e.with_ros2_event(event, time, context))
            .wrap_err("Timer not found. Missing rcl_timer_init event")?;

        let node_arc = self
            .nodes_by_rcl
            .get_or_err(event.node_handle.into_id(context_id), "rcl_handle")
            .map_err(|e| e.dependent_object(timer_arc))
            .map_err(|e| e.with_ros2_event(event, time, context))
            .wrap_err("Node not found. Missing rcl_node_init event")?;

        timer_arc
            .lock()
            .unwrap()
            .link_node(node_arc)
            .map_err(|e| eyre!("Timer already linked to a node: {e}"))?;

        node_arc.lock().unwrap().add_timer(timer_arc.clone());

        Ok(processed_events::ros2::RclcppTimerLinkNode {
            timer: timer_arc.clone(),
        })
    }

    pub(super) fn process_rclcpp_callback_register(
        &mut self,
        event: &raw_events::ros2::RclcppCallbackRegister,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::ros2::RclcppCallbackRegister> {
        let callback_arc = self
            .get_callback_by_id(event.callback.into_id(context_id))
            .map_err(|e| e.with_ros2_event(event, time, context))
            .wrap_err("Callback not found. Missing rclcpp_*_callback_added event?")?;

        callback_arc
            .lock()
            .unwrap()
            .set_name(event.symbol.clone())
            .map_err(|e| eyre!("Callback already registered with a name: {e}"))?;

        Ok(processed_events::ros2::RclcppCallbackRegister {
            callback: callback_arc.clone(),
        })
    }

    pub(super) fn process_callback_start(
        &mut self,
        event: &raw_events::ros2::CallbackStart,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::ros2::CallbackStart> {
        let callback_arc = self
            .get_callback_by_id(event.callback.into_id(context_id))
            .map_err(|e| e.with_ros2_event(event, time, context))
            .wrap_err("Callback not found. Missing rclcpp_*_callback_added event?")?;

        let callback_instance = CallbackInstance::new(callback_arc.clone(), time);

        Ok(processed_events::ros2::CallbackStart {
            callback: callback_instance,
            is_intra_process: event.is_intra_process,
        })
    }

    pub(super) fn process_callback_end(
        &mut self,
        event: &raw_events::ros2::CallbackEnd,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::ros2::CallbackEnd> {
        let callback_arc = self
            .get_callback_by_id(event.callback.into_id(context_id))
            .map_err(|e| e.with_ros2_event(event, time, context))
            .wrap_err("Callback not found. Missing rclcpp_*_callback_added event?")?;

        let callback_instance = {
            let mut callback = callback_arc.lock().unwrap();
            callback
                .take_running_instance()
                .expect("No running instance found")
        };

        {
            let mut callback_instance = callback_instance.lock().unwrap();
            callback_instance.end(time);
        }

        Ok(processed_events::ros2::CallbackEnd {
            callback: callback_instance,
        })
    }
}

fn filter_out_removed_callers(old_arc: Arc<Mutex<Callback>>) -> Option<Arc<Mutex<Callback>>> {
    let old = old_arc.lock().unwrap();
    if old
        .get_caller()
        .and_then(CallbackCaller::is_removed)
        .unwrap_or(true)
    {
        None
    } else {
        drop(old);
        Some(old_arc)
    }
}
