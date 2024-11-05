mod r2r;

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::events_common::{Context, Time};
use crate::model::{
    Callback, CallbackInstance, Client, Node, PublicationMessage, Publisher, Service, Subscriber,
    SubscriptionMessage, Timer,
};
use crate::{processed_events, raw_events};

pub enum MaybeProccessed<P, R> {
    Processed(P),
    Raw(R),
}

impl MaybeProccessed<processed_events::Event, raw_events::Event> {
    pub fn into_full_event(
        self,
        context: Context,
        time: Time,
    ) -> MaybeProccessed<processed_events::FullEvent, raw_events::FullEvent> {
        match self {
            MaybeProccessed::Processed(event) => {
                MaybeProccessed::Processed(processed_events::FullEvent {
                    context,
                    time,
                    event,
                })
            }
            MaybeProccessed::Raw(event) => MaybeProccessed::Raw(raw_events::FullEvent {
                context,
                time,
                event,
            }),
        }
    }
}

impl<P, R> From<Result<P, R>> for MaybeProccessed<processed_events::Event, raw_events::Event>
where
    P: Into<processed_events::Event>,
    R: Into<raw_events::Event>,
{
    fn from(result: Result<P, R>) -> Self {
        match result {
            Ok(processed) => MaybeProccessed::Processed(processed.into()),
            Err(raw) => MaybeProccessed::Raw(raw.into()),
        }
    }
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

    pub fn process_raw_event(
        &mut self,
        full_event: raw_events::FullEvent,
    ) -> MaybeProccessed<processed_events::FullEvent, raw_events::FullEvent> {
        match full_event.event {
            raw_events::Event::Ros2(event) => MaybeProccessed::from(self.process_raw_ros2_event(
                event,
                &full_event.context,
                full_event.time,
            )),
            raw_events::Event::R2r(event) => MaybeProccessed::from(self.process_raw_r2r_event(
                event,
                &full_event.context,
                full_event.time,
            )),
        }
        .into_full_event(full_event.context, full_event.time)
    }

    pub fn process_raw_ros2_event(
        &mut self,
        event: raw_events::ros2::Event,
        context: &Context,
        time: Time,
    ) -> Result<processed_events::ros2::Event, raw_events::ros2::Event> {
        let context_id = ContextId::new(context.vpid(), self.host_to_host_id(context.hostname()));

        Ok(match event {
            raw_events::ros2::Event::RclNodeInit(event) => {
                self.process_rcl_node_init(event, context_id).into()
            }
            raw_events::ros2::Event::RmwPublisherInit(event) => {
                self.process_rmw_publisher_init(event, context_id).into()
            }
            raw_events::ros2::Event::RclPublisherInit(event) => {
                self.process_rcl_publisher_init(event, context_id).into()
            }
            raw_events::ros2::Event::RclcppPublish(event) => {
                self.process_rclcpp_publish(event, context_id, time).into()
            }
            raw_events::ros2::Event::RclcppIntraPublish(event) => {
                self.process_rclcpp_intra_publish(event, context_id).into()
            }
            raw_events::ros2::Event::RclPublish(event) => {
                self.process_rcl_publish(event, context_id, time).into()
            }
            raw_events::ros2::Event::RmwPublish(event) => {
                self.process_rmw_publish(event, context_id, time).into()
            }
            raw_events::ros2::Event::RmwSubscriptionInit(event) => {
                self.process_rmw_subscription_init(event, context_id).into()
            }
            raw_events::ros2::Event::RclSubscriptionInit(event) => {
                self.process_rcl_subscription_init(event, context_id).into()
            }
            raw_events::ros2::Event::RclcppSubscriptionInit(event) => self
                .process_rclcpp_subscription_init(event, context_id)
                .into(),
            raw_events::ros2::Event::RclcppSubscriptionCallbackAdded(event) => self
                .process_rclcpp_subscription_callback_added(event, context_id)
                .into(),
            raw_events::ros2::Event::RmwTake(event) => {
                self.process_rmw_take(event, context_id, time).into()
            }
            raw_events::ros2::Event::RclTake(event) => {
                self.process_rcl_take(event, context_id, time).into()
            }
            raw_events::ros2::Event::RclcppTake(event) => {
                self.process_rclcpp_take(event, context_id, time).into()
            }
            raw_events::ros2::Event::RclServiceInit(event) => {
                self.process_rcl_service_init(event, context_id).into()
            }
            raw_events::ros2::Event::RclcppServiceCallbackAdded(event) => self
                .process_rclcpp_service_callback_added(event, context_id)
                .into(),
            raw_events::ros2::Event::RclClientInit(event) => {
                self.process_rcl_client_init(event, context_id).into()
            }
            raw_events::ros2::Event::RclTimerInit(event) => {
                self.process_rcl_timer_init(event, context_id).into()
            }
            raw_events::ros2::Event::RclcppTimerCallbackAdded(event) => self
                .process_rclcpp_timer_callback_added(event, context_id)
                .into(),
            raw_events::ros2::Event::RclcppTimerLinkNode(event) => self
                .process_rclcpp_timer_link_node(event, context_id)
                .into(),
            raw_events::ros2::Event::RclcppCallbackRegister(event) => self
                .process_rclcpp_callback_register(event, context_id)
                .into(),
            raw_events::ros2::Event::CallbackStart(event) => {
                self.process_callback_start(event, time, context_id).into()
            }
            raw_events::ros2::Event::CallbackEnd(event) => {
                self.process_callback_end(event, time, context_id).into()
            }

            _ => {
                return Err(event);
            }
        })
    }
}

// Event processing methods
impl Processor {
    fn process_rcl_node_init(
        &mut self,
        event: raw_events::ros2::RclNodeInit,
        context_id: ContextId,
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
        context_id: ContextId,
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

        {
            let mut publisher = publisher_arc.lock().unwrap();
            publisher.rmw_init(event.rmw_publisher_handle, event.gid);
        }

        processed_events::ros2::RmwPublisherInit {
            publisher: publisher_arc,
        }
    }

    fn process_rcl_publisher_init(
        &mut self,
        event: raw_events::ros2::RclPublisherInit,
        context_id: ContextId,
    ) -> processed_events::ros2::RclPublisherInit {
        let publisher_arc = self
            .publishers_by_rmw
            .entry(event.rmw_publisher_handle.into_id(context_id))
            .or_insert_with_key(|key| {
                let mut publisher = Publisher::default();
                publisher.set_rmw_handle(key.id);
                Arc::new(Mutex::new(publisher))
            });
        if self
            .publishers_by_rcl
            .insert(
                event.publisher_handle.into_id(context_id),
                publisher_arc.clone(),
            )
            .is_some()
        {
            panic!("Publishers by rcl already contains key");
        }

        let node_arc = self
            .nodes_by_rcl
            .entry(event.node_handle.into_id(context_id))
            .or_insert_with(|| {
                assert!(
                    event.topic_name == "/rosout",
                    "Node not found for publisher: {event:?}"
                );
                let node = Node::new(event.node_handle);
                Arc::new(Mutex::new(node))
            });

        {
            let mut node = node_arc.lock().unwrap();
            node.add_publisher(publisher_arc.clone());
        }
        {
            let mut publisher = publisher_arc.lock().unwrap();
            publisher.rcl_init(
                event.publisher_handle,
                event.topic_name,
                event.queue_depth,
                Arc::downgrade(node_arc),
            );
        }

        processed_events::ros2::RclPublisherInit {
            publisher: publisher_arc.clone(),
        }
    }

    fn process_rclcpp_publish(
        &mut self,
        event: raw_events::ros2::RclcppPublish,
        context_id: ContextId,
        time: Time,
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
        context_id: ContextId,
        time: Time,
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
        context_id: ContextId,
        time: Time,
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
                eprintln!("Missing timestamp for RMW publish event. Subscribtion messages will not match it: [{time}] {event:?}");

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
        context_id: ContextId,
    ) -> processed_events::ros2::RclcppIntraPublish {
        let message_arc = self
            .published_messages_by_rclcpp
            .get(&event.message.into_id(context_id))
            .unwrap()
            .clone();

        // TODO: check if event has rcl handle or other
        let publisher_arc = self
            .publishers_by_rcl
            .get(&event.publisher_handle.into_id(context_id))
            .unwrap()
            .clone();

        {
            let mut message = message_arc.lock().unwrap();
            message.set_publisher(publisher_arc);
        }

        processed_events::ros2::RclcppIntraPublish {
            message: message_arc,
        }
    }

    fn process_rmw_subscription_init(
        &mut self,
        event: raw_events::ros2::RmwSubscriptionInit,
        context_id: ContextId,
    ) -> processed_events::ros2::RmwSubscriptionInit {
        let subscriber_arc = self
            .subscribers_by_rmw
            .entry(event.rmw_subscription_handle.into_id(context_id))
            .or_default()
            .clone();

        {
            let mut subscriber = subscriber_arc.lock().unwrap();
            subscriber.rmw_init(event.rmw_subscription_handle, event.gid);
        }

        processed_events::ros2::RmwSubscriptionInit {
            subscription: subscriber_arc,
        }
    }

    fn process_rcl_subscription_init(
        &mut self,
        event: raw_events::ros2::RclSubscriptionInit,
        context_id: ContextId,
    ) -> processed_events::ros2::RclSubscriptionInit {
        let subscriber_arc = self
            .subscribers_by_rmw
            .entry(event.rmw_subscription_handle.into_id(context_id))
            .or_insert_with_key(|key| {
                let mut subscriber = Subscriber::default();
                subscriber.set_rmw_handle(key.id);
                Arc::new(Mutex::new(subscriber))
            });
        if self
            .subscribers_by_rcl
            .insert(
                event.subscription_handle.into_id(context_id),
                subscriber_arc.clone(),
            )
            .is_some()
        {
            panic!("Subscribers by rcl already contains key");
        }

        let node_arc = self
            .nodes_by_rcl
            .get(&event.node_handle.into_id(context_id))
            .unwrap_or_else(|| {
                panic!("Node not found for subscriber: {event:?}");
            });

        {
            let mut node = node_arc.lock().unwrap();
            node.add_subscriber(subscriber_arc.clone());
        }
        {
            let mut subscriber = subscriber_arc.lock().unwrap();
            subscriber.rcl_init(
                event.subscription_handle,
                event.topic_name,
                event.queue_depth,
                Arc::downgrade(node_arc),
            );
        }

        processed_events::ros2::RclSubscriptionInit {
            subscription: subscriber_arc.clone(),
        }
    }

    fn process_rclcpp_subscription_init(
        &mut self,
        event: raw_events::ros2::RclcppSubscriptionInit,
        context_id: ContextId,
    ) -> processed_events::ros2::RclcppSubscriptionInit {
        let subscriber_arc = self
            .subscribers_by_rcl
            .get(&event.subscription_handle.into_id(context_id))
            .unwrap()
            .clone();

        {
            let mut subscriber = subscriber_arc.lock().unwrap();
            subscriber.rclcpp_init(event.subscription);
        }

        if self
            .subscribers_by_rclcpp
            .insert(
                event.subscription.into_id(context_id),
                subscriber_arc.clone(),
            )
            .is_some()
        {
            panic!("Subscriber already exists for id: {event:?}");
        }

        processed_events::ros2::RclcppSubscriptionInit {
            subscription: subscriber_arc,
        }
    }

    fn process_rclcpp_subscription_callback_added(
        &mut self,
        event: raw_events::ros2::RclcppSubscriptionCallbackAdded,
        context_id: ContextId,
    ) -> processed_events::ros2::RclcppSubscriptionCallbackAdded {
        let subscription_arc = self
            .subscribers_by_rclcpp
            .get(&event.subscription.into_id(context_id))
            .unwrap();

        let callback_arc = Callback::new_subscription(event.callback, subscription_arc);

        if self
            .callbacks_by_id
            .insert(event.callback.into_id(context_id), callback_arc.clone())
            .is_some()
        {
            panic!("Callback already exists for id: {event:?}")
        }

        subscription_arc
            .lock()
            .unwrap()
            .set_callback(callback_arc.clone());

        processed_events::ros2::RclcppSubscriptionCallbackAdded {
            callback: callback_arc,
        }
    }

    fn process_rmw_take(
        &mut self,
        event: raw_events::ros2::RmwTake,
        context_id: ContextId,
        time: Time,
    ) -> processed_events::ros2::RmwTake {
        let subscriber = self
            .subscribers_by_rmw
            .get(&event.rmw_subscription_handle.into_id(context_id))
            .unwrap();

        let mut message = SubscriptionMessage::new(event.message);

        match (event.source_timestamp != 0)
            .then_some(())
            .and_then(|()| self.published_messages.get(&event.source_timestamp))
        {
            Some(published_message) => {
                message.rmw_take_matched(subscriber.clone(), published_message.clone(), time);
            }
            None => {
                if event.source_timestamp == 0 {
                    eprintln!("rmw_take: Missing source timestamp. [{time}] {event:?}");
                }
                message.rmw_take_unmatched(subscriber.clone(), event.source_timestamp, time);
            }
        }

        let message_arc = Arc::new(Mutex::new(message));

        if event.taken {
            let mut subscriber = subscriber.lock().unwrap();
            if let Some(_old) = subscriber.replace_taken_message(message_arc.clone()) {
                // TODO: Save message to dropped messages
            }

            self.received_messages
                .insert(event.message.into_id(context_id), message_arc.clone());
        }

        processed_events::ros2::RmwTake {
            message: message_arc,
            taken: event.taken,
        }
    }

    fn process_rcl_take(
        &mut self,
        event: raw_events::ros2::RclTake,
        context_id: ContextId,
        time: Time,
    ) -> processed_events::ros2::RclTake {
        let message_arc = self
            .received_messages
            .get(&event.message.into_id(context_id))
            .unwrap();

        {
            let mut message = message_arc.lock().unwrap();
            message.rcl_take(time);
        }

        processed_events::ros2::RclTake {
            message: message_arc.clone(),
        }
    }

    fn process_rclcpp_take(
        &mut self,
        event: raw_events::ros2::RclcppTake,
        context_id: ContextId,
        time: Time,
    ) -> processed_events::ros2::RclCppTake {
        let message_arc = self
            .received_messages
            .get(&event.message.into_id(context_id))
            .unwrap();

        {
            let mut message = message_arc.lock().unwrap();
            message.rclcpp_take(time);
        }

        processed_events::ros2::RclCppTake {
            message: message_arc.clone(),
        }
    }

    fn process_rcl_service_init(
        &mut self,
        event: raw_events::ros2::RclServiceInit,
        context_id: ContextId,
    ) -> processed_events::ros2::RclServiceInit {
        let service_arc = self
            .services_by_rcl
            .entry(event.service_handle.into_id(context_id))
            .or_insert_with_key(|key| {
                let service = Service::new(key.id);
                Arc::new(Mutex::new(service))
            })
            .clone();

        let node_arc = self
            .nodes_by_rcl
            .get(&event.node_handle.into_id(context_id))
            .unwrap_or_else(|| {
                panic!("Node not found for service: {event:?}");
            });

        {
            let mut service = service_arc.lock().unwrap();
            service.rcl_init(event.rmw_service_handle, event.service_name, node_arc);
        }
        {
            let mut node = node_arc.lock().unwrap();
            node.add_service(service_arc.clone());
        }

        processed_events::ros2::RclServiceInit {
            service: service_arc,
        }
    }

    fn process_rclcpp_service_callback_added(
        &mut self,
        event: raw_events::ros2::RclcppServiceCallbackAdded,
        context_id: ContextId,
    ) -> processed_events::ros2::RclCppServiceCallbackAdded {
        let service_arc = self
            .services_by_rcl
            .entry(event.service_handle.into_id(context_id))
            .or_insert_with_key(|key| {
                eprintln!("Service not found for callback. Creating new service. Event: {event:?} Creating new service");
                let service = Service::new(key.id);
                Arc::new(Mutex::new(service))
            });

        let callback_arc = Callback::new_service(event.callback, service_arc);

        if self
            .callbacks_by_id
            .insert(event.callback.into_id(context_id), callback_arc.clone())
            .is_some()
        {
            panic!("Callback already exists for id: {event:?}")
        }

        service_arc
            .lock()
            .unwrap()
            .set_callback(callback_arc.clone());

        processed_events::ros2::RclCppServiceCallbackAdded {
            callback: callback_arc,
        }
    }

    fn process_rcl_client_init(
        &mut self,
        event: raw_events::ros2::RclClientInit,
        context_id: ContextId,
    ) -> processed_events::ros2::RclClientInit {
        let client_arc = self
            .clients_by_rcl
            .entry(event.client_handle.into_id(context_id))
            .or_insert_with_key(|key| {
                let client = Client::new(key.id);
                Arc::new(Mutex::new(client))
            })
            .clone();

        let node_arc = self
            .nodes_by_rcl
            .get(&event.node_handle.into_id(context_id))
            .unwrap_or_else(|| {
                panic!("Node not found for client: {event:?}");
            });

        {
            let mut client = client_arc.lock().unwrap();
            client.rcl_init(event.rmw_client_handle, event.service_name, node_arc);
        }
        {
            let mut node = node_arc.lock().unwrap();
            node.add_client(client_arc.clone());
        }

        processed_events::ros2::RclClientInit { client: client_arc }
    }

    fn process_rcl_timer_init(
        &mut self,
        event: raw_events::ros2::RclTimerInit,
        context_id: ContextId,
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
    ) -> processed_events::ros2::RclcppTimerCallbackAdded {
        let timer_arc = self
            .timers_by_rcl
            .get(&event.timer_handle.into_id(context_id))
            .unwrap();

        let callback_arc = Callback::new_timer(event.callback, timer_arc);

        if self
            .callbacks_by_id
            .insert(event.callback.into_id(context_id), callback_arc.clone())
            .is_some()
        {
            panic!("Callback already exists for id: {event:?}")
        }

        timer_arc.lock().unwrap().set_callback(callback_arc.clone());

        processed_events::ros2::RclcppTimerCallbackAdded {
            callback: callback_arc,
        }
    }

    fn process_rclcpp_timer_link_node(
        &mut self,
        event: raw_events::ros2::RclcppTimerLinkNode,
        context_id: ContextId,
    ) -> processed_events::ros2::RclcppTimerLinkNode {
        let timer_arc = self
            .timers_by_rcl
            .get(&event.timer_handle.into_id(context_id))
            .unwrap()
            .clone();

        let node_arc = self
            .nodes_by_rcl
            .get(&event.node_handle.into_id(context_id))
            .unwrap_or_else(|| {
                panic!("Node not found for timer: {event:?}");
            });

        {
            let mut timer = timer_arc.lock().unwrap();
            timer.link_node(node_arc);
        }
        {
            let mut node = node_arc.lock().unwrap();
            node.add_timer(timer_arc.clone());
        }

        processed_events::ros2::RclcppTimerLinkNode { timer: timer_arc }
    }

    fn process_rclcpp_callback_register(
        &mut self,
        event: raw_events::ros2::RclcppCallbackRegister,
        context_id: ContextId,
    ) -> processed_events::ros2::RclcppCallbackRegister {
        let callback_arc = self
            .callbacks_by_id
            .get(&event.callback.into_id(context_id))
            .expect("Callback not found. Missing rclcpp_*_callback_added event?")
            .clone();

        {
            let mut callback = callback_arc.lock().unwrap();
            callback.set_name(event.symbol);
        }

        processed_events::ros2::RclcppCallbackRegister {
            callback: callback_arc,
        }
    }

    fn process_callback_start(
        &mut self,
        event: raw_events::ros2::CallbackStart,
        time: Time,
        context_id: ContextId,
    ) -> processed_events::ros2::CallbackStart {
        let callback_arc = self
            .callbacks_by_id
            .get(&event.callback.into_id(context_id))
            .expect("Callback not found. Missing rclcpp_*_callback_added event?");

        let calback_instance = CallbackInstance::new(callback_arc.clone(), time);

        processed_events::ros2::CallbackStart {
            callback: calback_instance,
            is_intra_process: event.is_intra_process,
        }
    }

    fn process_callback_end(
        &mut self,
        event: raw_events::ros2::CallbackEnd,
        time: Time,
        context_id: ContextId,
    ) -> processed_events::ros2::CallbackEnd {
        let callback_arc = self
            .callbacks_by_id
            .get(&event.callback.into_id(context_id))
            .expect("Callback not found. Missing rclcpp_*_callback_added event?")
            .clone();

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

        processed_events::ros2::CallbackEnd {
            callback: callback_instance,
        }
    }
}
