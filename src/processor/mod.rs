mod error;
mod r2r;

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use color_eyre::eyre::{eyre, Context as _};
use color_eyre::{Report, Result};
use thiserror::Error;

use crate::events_common::{Context, Time};
use crate::model::{
    Callback, CallbackInstance, Client, Node, PublicationMessage, Publisher, Service, Subscriber,
    SubscriptionMessage, Timer,
};
use crate::{processed_events, raw_events};

use super::model::CallbackCaller;

pub enum MaybeProcessed<P, R> {
    Processed(P),
    Raw(R),
}

impl MaybeProcessed<processed_events::Event, raw_events::Event> {
    pub fn into_full_event(
        self,
        context: Context,
        time: Time,
    ) -> MaybeProcessed<processed_events::FullEvent, raw_events::FullEvent> {
        match self {
            Self::Processed(event) => MaybeProcessed::Processed(processed_events::FullEvent {
                context,
                time,
                event,
            }),
            Self::Raw(event) => MaybeProcessed::Raw(raw_events::FullEvent {
                context,
                time,
                event,
            }),
        }
    }
}

impl<P, R> From<Result<P, R>> for MaybeProcessed<processed_events::Event, raw_events::Event>
where
    P: Into<processed_events::Event>,
    R: Into<raw_events::Event>,
{
    fn from(result: Result<P, R>) -> Self {
        match result {
            Ok(processed) => Self::Processed(processed.into()),
            Err(raw) => Self::Raw(raw.into()),
        }
    }
}

#[derive(Debug, Error)]
pub enum UnsupportedOrError<EVENT: Debug> {
    #[error("Unsupported event {_0:?}.")]
    Unsupported(EVENT),

    #[error(transparent)]
    Error(#[from] Report),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContextId {
    vpid: u32,
    host_id: u32,
}

impl ContextId {
    pub fn new(vpid: u32, host_id: u32) -> Self {
        Self { vpid, host_id }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id<T> {
    id: T,
    context: ContextId,
}

trait IntoId<T> {
    fn into_id(self, context: ContextId) -> Id<T>;
}

impl<T> IntoId<T> for T {
    fn into_id(self, context: ContextId) -> Id<T> {
        Id { id: self, context }
    }
}

#[derive(Debug, Default)]
pub struct Processor {
    hostname_to_host_id: HashMap<String, u32>,

    nodes_by_rcl: HashMap<Id<u64>, Arc<Mutex<Node>>>,

    subscribers_by_rmw: HashMap<Id<u64>, Arc<Mutex<Subscriber>>>,
    subscribers_by_rcl: HashMap<Id<u64>, Arc<Mutex<Subscriber>>>,
    subscribers_by_rclcpp: HashMap<Id<u64>, Arc<Mutex<Subscriber>>>,

    publishers_by_rmw: HashMap<Id<u64>, Arc<Mutex<Publisher>>>,
    publishers_by_rcl: HashMap<Id<u64>, Arc<Mutex<Publisher>>>,

    services_by_rcl: HashMap<Id<u64>, Arc<Mutex<Service>>>,

    clients_by_rcl: HashMap<Id<u64>, Arc<Mutex<Client>>>,

    timers_by_rcl: HashMap<Id<u64>, Arc<Mutex<Timer>>>,

    callbacks_by_id: HashMap<Id<u64>, Arc<Mutex<Callback>>>,

    /// Id by publication timestamp
    published_messages: HashMap<i64, Arc<Mutex<PublicationMessage>>>,
    /// Id by message ptr
    received_messages: HashMap<Id<u64>, Arc<Mutex<SubscriptionMessage>>>,

    // temporary
    /// Id by message ptr
    published_messages_by_rclcpp: HashMap<Id<u64>, Arc<Mutex<PublicationMessage>>>,
    /// Id by message ptr
    published_messages_by_rcl: HashMap<Id<u64>, Arc<Mutex<PublicationMessage>>>,
}

impl Processor {
    pub fn new() -> Self {
        Self::default()
    }

    fn host_to_host_id(&mut self, hostname: &str) -> u32 {
        let next_id = self
            .hostname_to_host_id
            .len()
            .try_into()
            .expect("Host ID overflow");
        *self
            .hostname_to_host_id
            .entry(hostname.to_owned())
            .or_insert(next_id)
    }

    pub fn get_all_nodes(&self) -> Vec<Arc<Mutex<Node>>> {
        self.nodes_by_rcl.values().cloned().collect()
    }

    pub fn print_objects(&self) {
        println!("Nodes:");
        for (i, node) in self.nodes_by_rcl.values().enumerate() {
            let node = node.lock().unwrap();
            print!(" [{i}] ");
            node.print_node_info();
        }
    }

    fn get_node_by_rcl_handle(&self, id: Id<u64>) -> Result<&Arc<Mutex<Node>>, error::NotFound> {
        self.nodes_by_rcl
            .get(&id)
            .ok_or(error::NotFound::node(id.id))
    }

    fn get_subscriber_by_rmw_handle(
        &self,
        id: Id<u64>,
    ) -> Result<&Arc<Mutex<Subscriber>>, error::NotFound> {
        self.subscribers_by_rmw
            .get(&id)
            .ok_or(error::NotFound::subscriber(id.id, "rmw_handle"))
    }

    fn get_subscriber_by_rcl_handle(
        &self,
        id: Id<u64>,
    ) -> Result<&Arc<Mutex<Subscriber>>, error::NotFound> {
        self.subscribers_by_rcl
            .get(&id)
            .ok_or(error::NotFound::subscriber(id.id, "rcl_handle"))
    }

    fn get_subscriber_by_rclcpp_handle(
        &self,
        id: Id<u64>,
    ) -> Result<&Arc<Mutex<Subscriber>>, error::NotFound> {
        self.subscribers_by_rclcpp
            .get(&id)
            .ok_or(error::NotFound::subscriber(id.id, "rclcpp_handle"))
    }

    fn get_publisher_by_rmw_handle(
        &self,
        id: Id<u64>,
    ) -> Result<&Arc<Mutex<Publisher>>, error::NotFound> {
        self.publishers_by_rmw
            .get(&id)
            .ok_or(error::NotFound::publisher(id.id, "rmw_handle"))
    }

    fn get_publisher_by_rcl_handle(
        &self,
        id: Id<u64>,
    ) -> Result<&Arc<Mutex<Publisher>>, error::NotFound> {
        self.publishers_by_rcl
            .get(&id)
            .ok_or(error::NotFound::publisher(id.id, "rcl_handle"))
    }

    fn get_service_by_rcl_handle(
        &self,
        id: Id<u64>,
    ) -> Result<&Arc<Mutex<Service>>, error::NotFound> {
        self.services_by_rcl
            .get(&id)
            .ok_or(error::NotFound::service(id.id, "rcl_handle"))
    }

    fn get_client_by_rcl_handle(
        &self,
        id: Id<u64>,
    ) -> Result<&Arc<Mutex<Client>>, error::NotFound> {
        self.clients_by_rcl
            .get(&id)
            .ok_or(error::NotFound::client(id.id, "rcl_handle"))
    }

    fn get_timer_by_rcl_handle(&self, id: Id<u64>) -> Result<&Arc<Mutex<Timer>>, error::NotFound> {
        self.timers_by_rcl
            .get(&id)
            .ok_or(error::NotFound::timer(id.id))
    }

    fn get_callback_by_id(&self, id: Id<u64>) -> Result<&Arc<Mutex<Callback>>, error::NotFound> {
        self.callbacks_by_id
            .get(&id)
            .ok_or(error::NotFound::callback(id.id))
    }

    pub fn process_raw_event(
        &mut self,
        full_event: raw_events::FullEvent,
    ) -> Result<MaybeProcessed<processed_events::FullEvent, raw_events::FullEvent>> {
        Ok(match full_event.event {
            raw_events::Event::Ros2(event) => {
                match self.process_raw_ros2_event(event, &full_event.context, full_event.time) {
                    Ok(processed) => MaybeProcessed::Processed(processed.into()),
                    Err(UnsupportedOrError::Unsupported(raw_event)) => {
                        MaybeProcessed::Raw(raw_event.into())
                    }
                    Err(UnsupportedOrError::Error(error)) => {
                        return Err(error);
                    }
                }
            }
            raw_events::Event::R2r(event) => {
                match self.process_raw_r2r_event(event, &full_event.context, full_event.time) {
                    Ok(processed) => MaybeProcessed::Processed(processed.into()),
                    Err(UnsupportedOrError::Unsupported(raw_event)) => {
                        MaybeProcessed::Raw(raw_event.into())
                    }
                    Err(UnsupportedOrError::Error(error)) => {
                        return Err(error);
                    }
                }
            }
        }
        .into_full_event(full_event.context, full_event.time))
    }

    pub fn process_raw_ros2_event(
        &mut self,
        event: raw_events::ros2::Event,
        context: &Context,
        time: Time,
    ) -> Result<processed_events::ros2::Event, UnsupportedOrError<raw_events::ros2::Event>> {
        let context_id = ContextId::new(context.vpid(), self.host_to_host_id(context.hostname()));

        Ok(match event {
            raw_events::ros2::Event::RclNodeInit(event) => self
                .process_rcl_node_init(event, time, context_id, context)
                .into(),
            raw_events::ros2::Event::RmwPublisherInit(event) => self
                .process_rmw_publisher_init(event, time, context_id, context)
                .into(),
            raw_events::ros2::Event::RclPublisherInit(event) => self
                .process_rcl_publisher_init(event, time, context_id, context)?
                .into(),
            raw_events::ros2::Event::RclcppPublish(event) => self
                .process_rclcpp_publish(event, time, context_id, context)
                .into(),
            raw_events::ros2::Event::RclcppIntraPublish(event) => self
                .process_rclcpp_intra_publish(event, time, context_id, context)?
                .into(),
            raw_events::ros2::Event::RclPublish(event) => self
                .process_rcl_publish(event, time, context_id, context)
                .into(),
            raw_events::ros2::Event::RmwPublish(event) => self
                .process_rmw_publish(event, time, context_id, context)
                .into(),
            raw_events::ros2::Event::RmwSubscriptionInit(event) => self
                .process_rmw_subscription_init(event, time, context_id, context)
                .into(),
            raw_events::ros2::Event::RclSubscriptionInit(event) => self
                .process_rcl_subscription_init(event, time, context_id, context)?
                .into(),
            raw_events::ros2::Event::RclcppSubscriptionInit(event) => self
                .process_rclcpp_subscription_init(event, time, context_id, context)?
                .into(),
            raw_events::ros2::Event::RclcppSubscriptionCallbackAdded(event) => self
                .process_rclcpp_subscription_callback_added(event, time, context_id, context)?
                .into(),
            raw_events::ros2::Event::RmwTake(event) => self
                .process_rmw_take(event, time, context_id, context)?
                .into(),
            raw_events::ros2::Event::RclTake(event) => self
                .process_rcl_take(event, time, context_id, context)
                .into(),
            raw_events::ros2::Event::RclcppTake(event) => self
                .process_rclcpp_take(event, time, context_id, context)
                .into(),
            raw_events::ros2::Event::RclServiceInit(event) => self
                .process_rcl_service_init(event, time, context_id, context)?
                .into(),
            raw_events::ros2::Event::RclcppServiceCallbackAdded(event) => self
                .process_rclcpp_service_callback_added(event, time, context_id, context)?
                .into(),
            raw_events::ros2::Event::RclClientInit(event) => self
                .process_rcl_client_init(event, time, context_id, context)?
                .into(),
            raw_events::ros2::Event::RclTimerInit(event) => self
                .process_rcl_timer_init(event, time, context_id, context)
                .into(),
            raw_events::ros2::Event::RclcppTimerCallbackAdded(event) => self
                .process_rclcpp_timer_callback_added(event, context_id, context, time)?
                .into(),
            raw_events::ros2::Event::RclcppTimerLinkNode(event) => self
                .process_rclcpp_timer_link_node(event, time, context_id, context)?
                .into(),
            raw_events::ros2::Event::RclcppCallbackRegister(event) => self
                .process_rclcpp_callback_register(event, time, context_id, context)?
                .into(),
            raw_events::ros2::Event::CallbackStart(event) => self
                .process_callback_start(event, time, context_id, context)?
                .into(),
            raw_events::ros2::Event::CallbackEnd(event) => self
                .process_callback_end(event, time, context_id, context)?
                .into(),

            _ => {
                return Err(UnsupportedOrError::Unsupported(event));
            }
        })
    }
}

// Event processing methods
impl Processor {
    fn process_rcl_node_init(
        &mut self,
        event: raw_events::ros2::RclNodeInit,
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

    fn process_rmw_publisher_init(
        &mut self,
        event: raw_events::ros2::RmwPublisherInit,
        _time: Time,
        context_id: ContextId,
        _context: &Context,
    ) -> processed_events::ros2::RmwPublisherInit {
        let publisher_arc = match self
            .publishers_by_rmw
            .entry(event.rmw_publisher_handle.into_id(context_id))
        {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => {
                let publisher = Publisher::default();
                let publisher_arc = Arc::new(Mutex::new(publisher));
                entry.insert(publisher_arc.clone());
                publisher_arc
            }
        };

        publisher_arc
            .lock()
            .unwrap()
            .rmw_init(event.rmw_publisher_handle, event.gid);

        processed_events::ros2::RmwPublisherInit {
            publisher: publisher_arc,
        }
    }

    fn process_rcl_publisher_init(
        &mut self,
        event: raw_events::ros2::RclPublisherInit,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::ros2::RclPublisherInit> {
        let publisher_arc = self
            .publishers_by_rmw
            .entry(event.rmw_publisher_handle.into_id(context_id))
            .or_insert_with_key(|key| {
                let mut publisher = Publisher::default();
                publisher.set_rmw_handle(key.id);
                Arc::new(Mutex::new(publisher))
            });
        self.publishers_by_rcl
            .insert(
                event.publisher_handle.into_id(context_id),
                publisher_arc.clone(),
            )
            .map_or(Ok(()), |old| {
                Err(error::AlreadyExists::by_rcl_handle(
                    event.publisher_handle,
                    publisher_arc.clone(),
                    old,
                )
                .with_ros2_event(&event, time, context))
            })?;

        let node_arc = self
            .nodes_by_rcl
            .entry(event.node_handle.into_id(context_id))
            .or_insert_with(|| {
                // Only /rosout publishers are allowed to be created before node.
                assert_eq!(
                    event.topic_name, "/rosout",
                    "Node not found for publisher: {event:?}"
                );
                let node = Node::new(event.node_handle);
                Arc::new(Mutex::new(node))
            });

        node_arc
            .lock()
            .unwrap()
            .add_publisher(publisher_arc.clone());

        publisher_arc.lock().unwrap().rcl_init(
            event.publisher_handle,
            event.topic_name,
            event.queue_depth,
            Arc::downgrade(node_arc),
        );

        Ok(processed_events::ros2::RclPublisherInit {
            publisher: publisher_arc.clone(),
        })
    }

    fn process_rclcpp_publish(
        &mut self,
        event: raw_events::ros2::RclcppPublish,
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

    fn process_rcl_publish(
        &mut self,
        event: raw_events::ros2::RclPublish,
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
                // FIXME: rosout publisher can change handle address
                // For now we just create a new publisher
                let mut publisher = Publisher::default();
                publisher.set_rcl_handle(key.id);

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

    fn process_rmw_publish(
        &mut self,
        event: raw_events::ros2::RmwPublish,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> processed_events::ros2::RmwPublish {
        let message_arc = self
            .published_messages_by_rcl
            .remove(&event.message.into_id(context_id))
            .unwrap_or_else(|| {
                let mut message = PublicationMessage::new(event.message);
                if let Some(rmw_handle) = event.rmw_publisher_handle {
                    if let Some(publisher) =
                        self.publishers_by_rmw.get(&rmw_handle.into_id(context_id))
                    {
                        message.set_publisher(publisher.clone());
                    }
                }
                Arc::new(Mutex::new(message))
            });
        event.timestamp.map_or_else(
            || {
                eprintln!("Missing timestamp for RMW publish event. Subscription messages will not match it: [{time}] {event:?} {context:?}");

                let mut message = message_arc.lock().unwrap();
                message.rmw_publish_old(time);
            },
            |timestamp| {
                let mut message = message_arc.lock().unwrap();
                message.rmw_publish(time, timestamp);

                self.published_messages
                    .insert(timestamp, message_arc.clone());
            },
        );

        processed_events::ros2::RmwPublish {
            message: message_arc,
        }
    }

    fn process_rclcpp_intra_publish(
        &mut self,
        event: raw_events::ros2::RclcppIntraPublish,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::ros2::RclcppIntraPublish> {
        let message_arc = self
            .published_messages_by_rclcpp
            .get(&event.message.into_id(context_id))
            .ok_or(error::NotFound::published_message(event.message))
            .map_err(|e| e.with_ros2_event(&event, time, context))?
            .clone();

        // TODO: check if event has rcl handle, rclcpp handle or other.
        let publisher_arc = self
            .get_publisher_by_rcl_handle(event.publisher_handle.into_id(context_id))
            .map_err(|e| e.with_ros2_event(&event, time, context))?
            .clone();

        message_arc.lock().unwrap().set_publisher(publisher_arc);

        Ok(processed_events::ros2::RclcppIntraPublish {
            message: message_arc,
        })
    }

    fn process_rmw_subscription_init(
        &mut self,
        event: raw_events::ros2::RmwSubscriptionInit,
        _time: Time,
        context_id: ContextId,
        _context: &Context,
    ) -> processed_events::ros2::RmwSubscriptionInit {
        let subscriber_arc = self
            .subscribers_by_rmw
            .entry(event.rmw_subscription_handle.into_id(context_id))
            .or_default()
            .clone();

        subscriber_arc
            .lock()
            .unwrap()
            .rmw_init(event.rmw_subscription_handle, event.gid);

        processed_events::ros2::RmwSubscriptionInit {
            subscription: subscriber_arc,
        }
    }

    fn process_rcl_subscription_init(
        &mut self,
        event: raw_events::ros2::RclSubscriptionInit,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::ros2::RclSubscriptionInit> {
        let subscriber_arc = self
            .subscribers_by_rmw
            .entry(event.rmw_subscription_handle.into_id(context_id))
            .or_insert_with_key(|key| {
                let mut subscriber = Subscriber::default();
                subscriber.set_rmw_handle(key.id);
                Arc::new(Mutex::new(subscriber))
            })
            .clone();

        self.subscribers_by_rcl
            .insert(
                event.subscription_handle.into_id(context_id),
                subscriber_arc.clone(),
            )
            .map_or(Ok(()), |old| {
                Err(error::AlreadyExists::new(
                    event.subscription_handle,
                    "rcl_handle",
                    subscriber_arc.clone(),
                    old,
                )
                .with_ros2_event(&event, time, context))
            })?;

        let node_arc = self
            .get_node_by_rcl_handle(event.node_handle.into_id(context_id))
            .map_err(|e| e.dependent_object(&subscriber_arc))
            .map_err(|e| e.with_ros2_event(&event, time, context))?;

        node_arc
            .lock()
            .unwrap()
            .add_subscriber(subscriber_arc.clone());

        subscriber_arc.lock().unwrap().rcl_init(
            event.subscription_handle,
            event.topic_name,
            event.queue_depth,
            Arc::downgrade(node_arc),
        );

        Ok(processed_events::ros2::RclSubscriptionInit {
            subscription: subscriber_arc.clone(),
        })
    }

    fn process_rclcpp_subscription_init(
        &mut self,
        event: raw_events::ros2::RclcppSubscriptionInit,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::ros2::RclcppSubscriptionInit> {
        let subscriber_arc = self
            .get_subscriber_by_rcl_handle(event.subscription_handle.into_id(context_id))
            .map_err(|e| e.with_ros2_event(&event, time, context))?
            .clone();

        subscriber_arc
            .lock()
            .unwrap()
            .rclcpp_init(event.subscription);

        self.subscribers_by_rclcpp
            .insert(
                event.subscription.into_id(context_id),
                subscriber_arc.clone(),
            )
            .and_then(|old_arc| {
                if Arc::ptr_eq(&old_arc, &subscriber_arc) {
                    unreachable!(
                        "The subscriber was already added to the map but without initialization."
                    );
                }
                let mut old = old_arc.lock().unwrap();
                let new = subscriber_arc.lock().unwrap();
                if old.get_topic() == new.get_topic() {
                    // Subscriber with same topic on the same address means it was destroyed and created again.
                    old.mark_removed();
                    None
                } else {
                    drop(old);
                    Some(old_arc)
                }
            })
            .map_or(Ok(()), |old| {
                Err(error::AlreadyExists::by_rclcpp_handle(
                    event.subscription,
                    subscriber_arc.clone(),
                    old,
                )
                .with_ros2_event(&event, time, context))
            })?;

        Ok(processed_events::ros2::RclcppSubscriptionInit {
            subscription: subscriber_arc.clone(),
        })
    }

    fn process_rclcpp_subscription_callback_added(
        &mut self,
        event: raw_events::ros2::RclcppSubscriptionCallbackAdded,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::ros2::RclcppSubscriptionCallbackAdded> {
        let subscription_arc = self
            .get_subscriber_by_rclcpp_handle(event.subscription.into_id(context_id))
            .map_err(|e| e.with_ros2_event(&event, time, context))?
            .clone();

        let callback_arc = Callback::new_subscription(event.callback, &subscription_arc);

        self.callbacks_by_id
            .insert(event.callback.into_id(context_id), callback_arc.clone())
            .and_then(|old_arc| {
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
            })
            .map_or(Ok(()), |old: Arc<Mutex<Callback>>| {
                Err(
                    error::AlreadyExists::with_id(event.callback, &callback_arc, old)
                        .with_ros2_event(&event, time, context),
                )
            })?;

        subscription_arc
            .lock()
            .unwrap()
            .set_callback(callback_arc.clone());

        Ok(processed_events::ros2::RclcppSubscriptionCallbackAdded {
            callback: callback_arc,
        })
    }

    fn process_rmw_take(
        &mut self,
        event: raw_events::ros2::RmwTake,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::ros2::RmwTake> {
        let subscriber = self
            .get_subscriber_by_rmw_handle(event.rmw_subscription_handle.into_id(context_id))
            .map_err(|e| e.with_ros2_event(&event, time, context))
            .wrap_err("Taken message missing subscriber.")?;

        let mut message = SubscriptionMessage::new(event.message);

        if let Some(published_message) = (event.source_timestamp != 0)
            .then_some(())
            .and_then(|()| self.published_messages.get(&event.source_timestamp))
        {
            message.rmw_take_matched(subscriber.clone(), published_message.clone(), time);
        } else {
            if event.source_timestamp == 0 {
                eprintln!("rmw_take: Missing source timestamp. [{time}] {event:?} {context:?}");
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

    fn process_rcl_take(
        &mut self,
        event: raw_events::ros2::RclTake,
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

            eprintln!(
                "rcl_take: Message was not taken before. Creating new message. [{time}] {event:?} {context:?}"
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

    fn process_rclcpp_take(
        &mut self,
        event: raw_events::ros2::RclcppTake,
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

            eprintln!("rclcpp_take: Message was not taken before. Creating new message. [{time}] {event:?} {context:?}");

            self.received_messages
                .insert(event.message.into_id(context_id), message_arc.clone());

            message_arc
        };

        processed_events::ros2::RclCppTake {
            message: message_arc.clone(),
            is_new,
        }
    }

    fn process_rcl_service_init(
        &mut self,
        event: raw_events::ros2::RclServiceInit,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::ros2::RclServiceInit> {
        let service_arc = self
            .services_by_rcl
            .entry(event.service_handle.into_id(context_id))
            .or_insert_with_key(|key| {
                let service = Service::new(key.id);
                Arc::new(Mutex::new(service))
            })
            .clone();

        let node_arc = self
            .get_node_by_rcl_handle(event.node_handle.into_id(context_id))
            .map_err(|e| e.dependent_object(&service_arc))?;

        service_arc
            .lock()
            .unwrap()
            .rcl_init(
                event.rmw_service_handle,
                event.service_name.clone(),
                node_arc,
            )
            .map_err(|e| {
                eyre!("Service was already initialized by rcl_service_init event: {e}").wrap_err(
                    format!(
                        "Error while processing event: [{time}] {:?} {context:?}",
                        &event
                    ),
                )
            })?;

        node_arc.lock().unwrap().add_service(service_arc.clone());

        Ok(processed_events::ros2::RclServiceInit {
            service: service_arc,
        })
    }

    fn process_rclcpp_service_callback_added(
        &mut self,
        event: raw_events::ros2::RclcppServiceCallbackAdded,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::ros2::RclCppServiceCallbackAdded> {
        let service_arc = self
            .services_by_rcl
            .entry(event.service_handle.into_id(context_id))
            .or_insert_with_key(|key| {
                eprintln!("Service not found for callback. Creating new (possibly duplicate) service. Event: [{time}] {event:?} {context:?}");
                let service = Service::new(key.id);
                Arc::new(Mutex::new(service))
            });

        let callback_arc = Callback::new_service(event.callback, service_arc);

        self.callbacks_by_id
            .insert(event.callback.into_id(context_id), callback_arc.clone())
            .map_or(Ok(()), |old| {
                Err(
                    error::AlreadyExists::with_id(event.callback, &callback_arc, old)
                        .with_ros2_event(&event, time, context),
                )
            })?;

        service_arc
            .lock()
            .unwrap()
            .set_callback(callback_arc.clone());

        Ok(processed_events::ros2::RclCppServiceCallbackAdded {
            callback: callback_arc,
        })
    }

    fn process_rcl_client_init(
        &mut self,
        event: raw_events::ros2::RclClientInit,
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
            })
            .clone();

        let node_arc = self
            .get_node_by_rcl_handle(event.node_handle.into_id(context_id))
            .map_err(|e| e.dependent_object(&client_arc))
            .map_err(|e| e.with_ros2_event(&event, time, context))?;

        {
            let mut client = client_arc.lock().unwrap();
            client.rcl_init(event.rmw_client_handle, event.service_name, node_arc);
        }

        node_arc.lock().unwrap().add_client(client_arc.clone());

        Ok(processed_events::ros2::RclClientInit { client: client_arc })
    }

    fn process_rcl_timer_init(
        &mut self,
        event: raw_events::ros2::RclTimerInit,
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
            })
            .clone();

        {
            let mut timer = timer_arc.lock().unwrap();
            timer.rcl_init(event.period);
        }

        processed_events::ros2::RclTimerInit { timer: timer_arc }
    }

    fn process_rclcpp_timer_callback_added(
        &mut self,
        event: raw_events::ros2::RclcppTimerCallbackAdded,
        context_id: ContextId,
        context: &Context,
        time: Time,
    ) -> Result<processed_events::ros2::RclcppTimerCallbackAdded> {
        let timer_arc = self
            .get_timer_by_rcl_handle(event.timer_handle.into_id(context_id))
            .map_err(|e| e.with_ros2_event(&event, time, context))?
            .clone();

        let callback_arc = Callback::new_timer(event.callback, &timer_arc);

        self.callbacks_by_id
            .insert(event.callback.into_id(context_id), callback_arc.clone())
            .map_or(Ok(()), |old| {
                Err(
                    error::AlreadyExists::with_id(event.callback, &callback_arc, old)
                        .with_ros2_event(&event, time, context),
                )
            })?;

        timer_arc.lock().unwrap().set_callback(callback_arc.clone());

        Ok(processed_events::ros2::RclcppTimerCallbackAdded {
            callback: callback_arc,
        })
    }

    fn process_rclcpp_timer_link_node(
        &mut self,
        event: raw_events::ros2::RclcppTimerLinkNode,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::ros2::RclcppTimerLinkNode> {
        let timer_arc = self
            .get_timer_by_rcl_handle(event.timer_handle.into_id(context_id))
            .map_err(|e| e.with_ros2_event(&event, time, context))
            .wrap_err("Timer not found. Missing rcl_timer_init event")?;

        let node_arc = self
            .get_node_by_rcl_handle(event.node_handle.into_id(context_id))
            .map_err(|e| e.dependent_object(timer_arc))
            .map_err(|e| e.with_ros2_event(&event, time, context))
            .wrap_err("Node not found. Missing rcl_node_init event")?;

        {
            let mut timer = timer_arc.lock().unwrap();
            timer.link_node(node_arc);
        }
        {
            let mut node = node_arc.lock().unwrap();
            node.add_timer(timer_arc.clone());
        }

        Ok(processed_events::ros2::RclcppTimerLinkNode {
            timer: timer_arc.clone(),
        })
    }

    fn process_rclcpp_callback_register(
        &mut self,
        event: raw_events::ros2::RclcppCallbackRegister,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::ros2::RclcppCallbackRegister> {
        let callback_arc = self
            .get_callback_by_id(event.callback.into_id(context_id))
            .map_err(|e| e.with_ros2_event(&event, time, context))
            .wrap_err("Callback not found. Missing rclcpp_*_callback_added event?")?;

        {
            let mut callback = callback_arc.lock().unwrap();
            callback.set_name(event.symbol);
        }

        Ok(processed_events::ros2::RclcppCallbackRegister {
            callback: callback_arc.clone(),
        })
    }

    fn process_callback_start(
        &mut self,
        event: raw_events::ros2::CallbackStart,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::ros2::CallbackStart> {
        let callback_arc = self
            .get_callback_by_id(event.callback.into_id(context_id))
            .map_err(|e| e.with_ros2_event(&event, time, context))
            .wrap_err("Callback not found. Missing rclcpp_*_callback_added event?")?;

        let callback_instance = CallbackInstance::new(callback_arc.clone(), time);

        Ok(processed_events::ros2::CallbackStart {
            callback: callback_instance,
            is_intra_process: event.is_intra_process,
        })
    }

    fn process_callback_end(
        &mut self,
        event: raw_events::ros2::CallbackEnd,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::ros2::CallbackEnd> {
        let callback_arc = self
            .get_callback_by_id(event.callback.into_id(context_id))
            .map_err(|e| e.with_ros2_event(&event, time, context))
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
