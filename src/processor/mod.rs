mod error;
mod r2r;
mod ros2;

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use color_eyre::{Report, Result};
use thiserror::Error;

use crate::events_common::{Context, Time};
use crate::model::{
    Callback, Client, Node, PublicationMessage, Publisher, Service, Subscriber,
    SubscriptionMessage, Timer,
};
use crate::{processed_events, raw_events};

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
