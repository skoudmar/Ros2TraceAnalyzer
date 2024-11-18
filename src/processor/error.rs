use std::sync::{Arc, Mutex};

use derive_more::derive::Display;
use thiserror::Error;

use crate::events_common::Context;
use crate::model::{
    AlreadyInitializedError, Callback, Client, Node, PublicationMessage, Publisher, Service,
    Subscriber, SubscriptionMessage, Time, Timer,
};
use crate::raw_events;
use crate::utils::CyclicDependency;

#[derive(Debug, Display)]
pub enum Object {
    #[display("Node{}", _0.lock().unwrap())]
    Node(Arc<Mutex<Node>>),

    #[display("Subscriber{}", _0.lock().unwrap())]
    Subscriber(Arc<Mutex<Subscriber>>),

    #[display("Publisher{}", _0.lock().unwrap())]
    Publisher(Arc<Mutex<Publisher>>),

    #[display("Service{}", _0.lock().unwrap())]
    Service(Arc<Mutex<Service>>),

    #[display("Client{}", _0.lock().unwrap())]
    Client(Arc<Mutex<Client>>),

    #[display("Timer{}", _0.lock().unwrap())]
    Timer(Arc<Mutex<Timer>>),

    #[display("Callback{}", _0.lock().unwrap())]
    Callback(Arc<Mutex<Callback>>),

    #[display("PublishedMessage")]
    PublishedMessage(Arc<Mutex<PublicationMessage>>),

    #[display("SubscribedMessage")]
    SubscribedMessage(Arc<Mutex<SubscriptionMessage>>),
}

impl Object {
    pub fn as_type(&self) -> ObjectType {
        match self {
            Self::Node(_) => ObjectType::Node,
            Self::Subscriber(_) => ObjectType::Subscriber,
            Self::Publisher(_) => ObjectType::Publisher,
            Self::Service(_) => ObjectType::Service,
            Self::Client(_) => ObjectType::Client,
            Self::Timer(_) => ObjectType::Timer,
            Self::Callback(_) => ObjectType::Callback,
            Self::PublishedMessage(_) => ObjectType::PublishedMessage,
            Self::SubscribedMessage(_) => ObjectType::SubscribedMessage,
        }
    }
}

impl From<Arc<Mutex<Node>>> for Object {
    fn from(node_arc: Arc<Mutex<Node>>) -> Self {
        Self::Node(node_arc)
    }
}

impl From<Arc<Mutex<Subscriber>>> for Object {
    fn from(subscriber_arc: Arc<Mutex<Subscriber>>) -> Self {
        subscriber_arc.lock().unwrap().create_cycle();
        Self::Subscriber(subscriber_arc)
    }
}

impl From<Arc<Mutex<Publisher>>> for Object {
    fn from(publisher_arc: Arc<Mutex<Publisher>>) -> Self {
        publisher_arc.lock().unwrap().create_cycle();
        Self::Publisher(publisher_arc)
    }
}

impl From<Arc<Mutex<Service>>> for Object {
    fn from(service_arc: Arc<Mutex<Service>>) -> Self {
        service_arc.lock().unwrap().create_cycle();
        Self::Service(service_arc)
    }
}

impl From<Arc<Mutex<Client>>> for Object {
    fn from(client_arc: Arc<Mutex<Client>>) -> Self {
        client_arc.lock().unwrap().create_cycle();
        Self::Client(client_arc)
    }
}

impl From<Arc<Mutex<Timer>>> for Object {
    fn from(timer_arc: Arc<Mutex<Timer>>) -> Self {
        timer_arc.lock().unwrap().create_cycle();
        Self::Timer(timer_arc)
    }
}

impl From<Arc<Mutex<Callback>>> for Object {
    fn from(callback_arc: Arc<Mutex<Callback>>) -> Self {
        callback_arc.lock().unwrap().create_cycle();
        Self::Callback(callback_arc)
    }
}

impl From<Arc<Mutex<PublicationMessage>>> for Object {
    fn from(publication_arc: Arc<Mutex<PublicationMessage>>) -> Self {
        Self::PublishedMessage(publication_arc)
    }
}

impl From<Arc<Mutex<SubscriptionMessage>>> for Object {
    fn from(subscription_arc: Arc<Mutex<SubscriptionMessage>>) -> Self {
        Self::SubscribedMessage(subscription_arc)
    }
}

impl<T> From<&T> for Object
where
    T: Into<Self> + Clone,
{
    fn from(value: &T) -> Self {
        value.clone().into()
    }
}

impl<T> From<&mut T> for Object
where
    T: Into<Self> + Clone,
{
    fn from(value: &mut T) -> Self {
        value.clone().into()
    }
}

impl Drop for Object {
    fn drop(&mut self) {
        match self {
            Self::Subscriber(subscriber) => {
                let mut subscriber = subscriber.lock().unwrap();
                subscriber.break_cycle();
            }
            Self::Publisher(publisher) => {
                let mut publisher = publisher.lock().unwrap();
                publisher.break_cycle();
            }
            Self::Service(service) => {
                let mut service = service.lock().unwrap();
                service.break_cycle();
            }
            Self::Client(client) => {
                let mut client = client.lock().unwrap();
                client.break_cycle();
            }
            Self::Timer(timer) => {
                let mut timer = timer.lock().unwrap();
                timer.break_cycle();
            }
            Self::Callback(callback) => {
                let mut callback = callback.lock().unwrap();
                callback.break_cycle();
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectType {
    Node,
    Subscriber,
    Publisher,
    Service,
    Client,
    Timer,
    Callback,
    PublishedMessage,
    SubscribedMessage,
}

#[derive(Debug, Error)]
pub struct AlreadyExists {
    key: u64,
    key_variable: &'static str,
    new_object: Object,
    old_object: Object,
}

impl AlreadyExists {
    pub fn new(
        key: u64,
        key_variable: &'static str,
        new_object: impl Into<Object>,
        old_object: impl Into<Object>,
    ) -> Self {
        Self {
            key,
            key_variable,
            new_object: new_object.into(),
            old_object: old_object.into(),
        }
    }

    pub fn with_id(id: u64, old_object: impl Into<Object>, new_object: impl Into<Object>) -> Self {
        Self::new(id, stringify!(id), new_object, old_object)
    }

    pub fn with_ros2_event<E: Clone + Into<raw_events::ros2::Event>>(
        self,
        event: &E,
        time: Time,
        context: &Context,
    ) -> ProcessingEvent {
        ProcessingEvent {
            raw_event: raw_events::Event::from(event.clone().into()),
            timestamp: time,
            context: context.clone(),
            cause: self.into(),
        }
    }
}

impl std::fmt::Display for AlreadyExists {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Object already exists: {:?} with {} = {:#x}",
            self.new_object.as_type(),
            self.key_variable,
            self.key
        )?;
        writeln!(f, "Original object: {}", self.old_object)?;
        write!(f, "New object: {}", self.new_object)
    }
}

#[derive(Debug, Error)]
#[error("Object not found: {object_type:?} with {key_variable} = {key:#x}")]
pub struct NotFound {
    key: u64,
    object_type: ObjectType,
    key_variable: &'static str,
}

impl NotFound {
    pub fn new(key: u64, key_variable: &'static str, object_type: ObjectType) -> Self {
        Self {
            key,
            object_type,
            key_variable,
        }
    }

    pub fn timer(key: u64) -> Self {
        Self::new(key, "rcl_handle", ObjectType::Timer)
    }

    pub fn callback(key: u64) -> Self {
        Self::new(key, "id", ObjectType::Callback)
    }

    pub fn published_message(key: u64) -> Self {
        Self::new(key, "address", ObjectType::PublishedMessage)
    }

    pub fn dependent_object(self, object: impl Into<Object>) -> ObjectMissingDependency {
        ObjectMissingDependency {
            object: object.into(),
            cause: self,
        }
    }

    pub fn with_ros2_event<E: Clone + Into<raw_events::ros2::Event>>(
        self,
        event: &E,
        time: Time,
        context: &Context,
    ) -> ProcessingEvent {
        ProcessingEvent {
            raw_event: raw_events::Event::from(event.clone().into()),
            timestamp: time,
            context: context.clone(),
            cause: self.into(),
        }
    }

    pub fn with_r2r_event<E: Clone + Into<raw_events::r2r::Event>>(
        self,
        event: &E,
        time: Time,
        context: &Context,
    ) -> ProcessingEvent {
        ProcessingEvent {
            raw_event: raw_events::Event::from(event.clone().into()),
            timestamp: time,
            context: context.clone(),
            cause: self.into(),
        }
    }
}

#[derive(Debug, Error)]
pub struct ObjectMissingDependency {
    object: Object,
    cause: NotFound,
}

impl ObjectMissingDependency {
    pub fn with_ros2_event<E: Clone + Into<raw_events::ros2::Event>>(
        self,
        event: &E,
        time: Time,
        context: &Context,
    ) -> ProcessingEvent {
        ProcessingEvent {
            raw_event: raw_events::Event::from(event.clone().into()),
            timestamp: time,
            context: context.clone(),
            cause: self.into(),
        }
    }
}

impl std::fmt::Display for ObjectMissingDependency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "{:?} is missing {:?}",
            self.object.as_type(),
            self.cause.object_type
        )?;
        write!(f, "  Affected {:?}: {}", self.object.as_type(), self.object)
    }
}

#[derive(Debug, Error)]
pub enum Causes {
    #[error(transparent)]
    AlreadyExists(#[from] AlreadyExists),

    #[error(transparent)]
    NotFound(#[from] NotFound),

    #[error(transparent)]
    ObjectMissingDependency(#[from] ObjectMissingDependency),

    #[error("{_0}: {_1}")]
    AlreadyInitialized(#[source] AlreadyInitializedError, Object),
}

#[derive(Debug, Error)]
#[error("Error processing event: [{timestamp}] {raw_event:?} {context:?}")]
pub struct ProcessingEvent {
    raw_event: raw_events::Event,
    timestamp: Time,
    context: Context,

    #[source]
    cause: Causes,
}
