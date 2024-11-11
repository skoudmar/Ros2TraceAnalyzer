pub(crate) mod display;

use std::fmt::Debug;
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use chrono::{Local, TimeZone};
use derive_more::derive::Unwrap;
use thiserror::Error;

use crate::raw_events;
use crate::utils::Known;

const GID_SIZE: usize = 16;
const GID_SUFFIX_SIZE: usize = 8;

#[derive(Debug, Error)]
#[error("Already set Error: {msg}\nobject: {object:#?}\nnew_value: {new_value:#?}")]
pub struct AlreadySetError<T: Debug, U: Debug> {
    object: T,
    new_value: U,
    msg: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Time {
    /// Nanoseconds since the UNIX epoch (1970-01-01 00:00:00 UTC)
    timestamp: i64,
}

impl Time {
    pub const fn from_nanos(nanos: i64) -> Self {
        Self { timestamp: nanos }
    }

    pub const fn timestamp_nanos(self) -> i64 {
        self.timestamp
    }

    pub fn as_datetime(self) -> chrono::DateTime<chrono::Local> {
        Local.timestamp_nanos(self.timestamp)
    }
}

#[derive(Clone)]
pub struct Name {
    full_name: String,
    namespace_len: usize,
}

impl Name {
    pub fn new(namespace: &str, name: &str) -> Self {
        Self {
            full_name: format!("{namespace}{name}"),
            namespace_len: namespace.len(),
        }
    }

    pub fn get_full_name(&self) -> &str {
        &self.full_name
    }

    pub fn get_namespace(&self) -> &str {
        &self.full_name[..self.namespace_len]
    }

    pub fn get_name(&self) -> &str {
        &self.full_name[self.namespace_len..]
    }
}

#[derive(Debug)]
pub struct Node {
    rcl_handle: u64,
    rmw_handle: Known<u64>,
    full_name: Known<Name>,

    subscribers: Vec<Arc<Mutex<Subscriber>>>,
    publishers: Vec<Arc<Mutex<Publisher>>>,
    services: Vec<Arc<Mutex<Service>>>,
    clients: Vec<Arc<Mutex<Client>>>,
    timers: Vec<Arc<Mutex<Timer>>>,

    spin_instance: Option<Arc<Mutex<SpinInstance>>>,
}

impl Node {
    pub fn new(rcl_handle: u64) -> Self {
        Self {
            rcl_handle,
            rmw_handle: Known::Unknown,
            full_name: Known::Unknown,
            subscribers: Vec::new(),
            publishers: Vec::new(),
            services: Vec::new(),
            clients: Vec::new(),
            timers: Vec::new(),
            spin_instance: None,
        }
    }

    pub fn add_subscriber(&mut self, subscriber: Arc<Mutex<Subscriber>>) {
        self.subscribers.push(subscriber);
    }

    pub fn add_publisher(&mut self, publisher: Arc<Mutex<Publisher>>) {
        self.publishers.push(publisher);
    }

    pub fn add_service(&mut self, service: Arc<Mutex<Service>>) {
        self.services.push(service);
    }

    pub fn add_client(&mut self, client: Arc<Mutex<Client>>) {
        self.clients.push(client);
    }

    pub fn add_timer(&mut self, timer: Arc<Mutex<Timer>>) {
        self.timers.push(timer);
    }

    pub fn rcl_init(&mut self, rmw_handle: u64, name: &str, namespace: &str) {
        assert!(
            self.rmw_handle.is_unknown() && self.full_name.is_unknown(),
            "Node already initialized with rcl_init_node. {self:#?}"
        );

        self.rmw_handle = Known::new(rmw_handle);
        self.full_name = Known::new(Name::new(namespace, name));
    }

    pub fn get_full_name(&self) -> Known<&str> {
        self.full_name.as_ref().map(Name::get_full_name)
    }

    pub fn get_namespace(&self) -> Known<&str> {
        self.full_name.as_ref().map(Name::get_namespace)
    }

    pub fn get_name(&self) -> Known<&str> {
        self.full_name.as_ref().map(Name::get_name)
    }

    pub fn timers(&self) -> &[Arc<Mutex<Timer>>] {
        &self.timers
    }

    pub fn subscribers(&self) -> &[Arc<Mutex<Subscriber>>] {
        &self.subscribers
    }

    pub fn publishers(&self) -> &[Arc<Mutex<Publisher>>] {
        &self.publishers
    }

    pub fn services(&self) -> &[Arc<Mutex<Service>>] {
        &self.services
    }

    pub fn clients(&self) -> &[Arc<Mutex<Client>>] {
        &self.clients
    }

    pub fn take_spin_instance(&mut self) -> Option<Arc<Mutex<SpinInstance>>> {
        self.spin_instance.take()
    }

    pub fn put_spin_instance(&mut self, spin_instance: Arc<Mutex<SpinInstance>>) {
        assert!(
            self.spin_instance.is_none(),
            "Node spin_instance already set. {self:#?}"
        );
        self.spin_instance = Some(spin_instance);
    }

    pub fn borrow_spin_instance(&self) -> Option<&Arc<Mutex<SpinInstance>>> {
        self.spin_instance.as_ref()
    }

    pub fn print_node_info(&self) {
        println!("Node{}", self);
        for subscriber in self.subscribers() {
            println!(
                "  Subscriber{:#}",
                display::DisplaySubscriberWithoutNode(&subscriber.lock().unwrap())
            );
        }
        for publisher in self.publishers() {
            println!(
                "  Publisher{:#}",
                display::DisplayPublisherWithoutNode(&publisher.lock().unwrap())
            );
        }
        for service in self.services() {
            println!(
                "  Service{:#}",
                display::DisplayServiceWithoutNode(&service.lock().unwrap())
            );
        }
        for client in self.clients() {
            println!(
                "  Client{:#}",
                display::DisplayClientWithoutNode(&client.lock().unwrap())
            );
        }
        for timer in self.timers() {
            println!(
                "  Timer{:#}",
                display::TimerDisplayWithoutNode(&timer.lock().unwrap())
            );
        }
    }
}

#[derive(Debug, Default)]
pub struct Subscriber {
    dds_handle: Known<u64>,
    rmw_handle: Known<u64>,
    rcl_handle: Known<u64>,
    rclcpp_handle: Known<u64>,

    rmw_gid: Known<RmwGid>,

    topic_name: Known<String>,
    node: Known<Weak<Mutex<Node>>>,
    queue_depth: Known<usize>,

    callback: Known<Arc<Mutex<Callback>>>,
    taken_message: Option<Arc<Mutex<SubscriptionMessage>>>,
}

impl Subscriber {
    pub fn rmw_init(&mut self, rmw_handle: u64, gid: [u8; raw_events::ros2::GID_SIZE]) {
        assert!(
            (self.rmw_handle.is_unknown() || self.rmw_handle.eq_inner(&rmw_handle))
                && self.rmw_gid.is_unknown(),
            "Subscriber already initialized with rmw_init_subscription. {self:#?}"
        );
        self.rmw_handle = Known::new(rmw_handle);
        self.rmw_gid = Known::new(RmwGid::new(gid));
    }

    pub fn rcl_init(
        &mut self,
        rcl_handle: u64,
        topic_name: String,
        queue_depth: usize,
        node: Weak<Mutex<Node>>,
    ) {
        assert!(
            self.rcl_handle.is_unknown(),
            "Subscriber already initialized with rcl_subscription_init. {self:#?}"
        );
        self.rcl_handle = Known::new(rcl_handle);
        self.topic_name = Known::new(topic_name);
        self.queue_depth = Known::new(queue_depth);
        self.node = Known::new(node);
    }

    pub fn rclcpp_init(&mut self, rclcpp_handle: u64) {
        assert!(
            self.rclcpp_handle.is_unknown(),
            "Subscriber already initialized with rclcpp_subscription_init. {self:#?}"
        );
        self.rclcpp_handle = Known::new(rclcpp_handle);
    }

    pub(crate) fn set_rmw_handle(&mut self, rmw_subscription_handle: u64) {
        assert!(
            self.rmw_handle.is_unknown(),
            "Subscriber rmw_handle already set. {self:#?}"
        );
        self.rmw_handle = Known::new(rmw_subscription_handle);
    }

    pub fn set_callback(&mut self, callback: Arc<Mutex<Callback>>) {
        assert!(
            self.callback.is_unknown(),
            "Subscriber callback already set. {self:#?}"
        );

        self.callback = Known::new(callback);
    }

    pub fn replace_taken_message(
        &mut self,
        message: Arc<Mutex<SubscriptionMessage>>,
    ) -> Option<Arc<Mutex<SubscriptionMessage>>> {
        self.taken_message.replace(message)
    }

    pub fn take_message(&mut self) -> Option<Arc<Mutex<SubscriptionMessage>>> {
        self.taken_message.take()
    }

    pub fn get_topic(&self) -> Known<&str> {
        self.topic_name.as_deref()
    }

    pub fn get_node(&self) -> Known<Weak<Mutex<Node>>> {
        self.node.clone()
    }
}

#[derive(Debug, Default)]
pub struct Publisher {
    dds_handle: Known<u64>,
    rmw_handle: Known<u64>,
    rcl_handle: Known<u64>,
    rclcpp_handle: Known<u64>,

    rmw_gid: Known<RmwGid>,

    topic_name: Known<String>,
    node: Known<Weak<Mutex<Node>>>,
    queue_depth: Known<usize>,
}

impl Publisher {
    pub fn rmw_init(&mut self, rmw_handle: u64, gid: [u8; raw_events::ros2::GID_SIZE]) {
        assert!(
            (self.rmw_handle.is_unknown() || self.rmw_handle.eq_inner(&rmw_handle))
                && self.rmw_gid.is_unknown(),
            "Publisher already initialized with rmw_init_publisher. {self:#?}"
        );
        self.rmw_handle = Known::new(rmw_handle);
        self.rmw_gid = Known::new(RmwGid::new(gid));
    }

    pub fn rcl_init(
        &mut self,
        rcl_handle: u64,
        topic_name: String,
        queue_depth: usize,
        node: Weak<Mutex<Node>>,
    ) {
        assert!(
            self.rcl_handle.is_unknown(),
            "Publisher already initialized with rcl_publisher_init. {self:#?}"
        );
        self.rcl_handle = Known::new(rcl_handle);
        self.topic_name = Known::new(topic_name);
        self.queue_depth = Known::new(queue_depth);
        self.node = Known::new(node);
    }

    pub(crate) fn set_rmw_handle(&mut self, rmw_publisher_handle: u64) {
        assert!(
            self.rmw_handle.is_unknown(),
            "Publisher rmw_handle already set. {self:#?}"
        );
        self.rmw_handle = Known::new(rmw_publisher_handle);
    }

    pub fn set_rcl_handle(&mut self, rcl_publisher_handle: u64) {
        assert!(
            self.rcl_handle.is_unknown(),
            "Publisher rcl_handle already set. {self:#?}"
        );
        self.rcl_handle = Known::new(rcl_publisher_handle);
    }

    pub fn get_topic(&self) -> Option<&str> {
        self.topic_name.as_deref().into()
    }

    pub fn get_node(&self) -> Option<Weak<Mutex<Node>>> {
        self.node.clone().into()
    }
}

#[derive(Debug)]
pub struct Service {
    rmw_handle: Known<u64>,
    rcl_handle: u64,
    rclcpp_handle: Known<u64>,

    name: Known<String>,
    node: Known<Weak<Mutex<Node>>>,
    callback: Known<Arc<Mutex<Callback>>>,
}

impl Service {
    pub fn new(rcl_handle: u64) -> Self {
        Self {
            rmw_handle: Known::Unknown,
            rcl_handle,
            rclcpp_handle: Known::Unknown,
            name: Known::Unknown,
            node: Known::Unknown,
            callback: Known::Unknown,
        }
    }

    pub fn rcl_init(&mut self, rmw_handle: u64, name: String, node: &Arc<Mutex<Node>>) {
        assert!(
            self.name.is_unknown() && self.node.is_unknown(),
            "Service already initialized with rcl_service_init. {self:#?}"
        );
        self.rmw_handle = Known::new(rmw_handle);
        self.name = Known::new(name);
        self.node = Known::new(Arc::downgrade(node));
    }

    pub fn set_callback(&mut self, callback: Arc<Mutex<Callback>>) {
        assert!(
            self.callback.is_unknown(),
            "Service callback already set. {self:#?}"
        );

        self.callback = Known::new(callback);
    }
}

#[derive(Debug, Default)]
pub struct Client {
    rcl_handle: u64,
    rmw_handle: Known<u64>,
    node: Known<Weak<Mutex<Node>>>,
    service_name: Known<String>,
}

impl Client {
    pub fn new(id: u64) -> Self {
        Self {
            rcl_handle: id,
            rmw_handle: Known::Unknown,
            node: Known::Unknown,
            service_name: Known::Unknown,
        }
    }

    pub fn rcl_init(&mut self, rmw_handle: u64, service_name: String, node: &Arc<Mutex<Node>>) {
        assert!(
            self.rmw_handle.is_unknown()
                && self.node.is_unknown()
                && self.service_name.is_unknown(),
            "Client already initialized with rcl_client_init. {self:#?}"
        );
        self.rmw_handle = Known::new(rmw_handle);
        self.node = Known::new(Arc::downgrade(node));
        self.service_name = Known::new(service_name);
    }
}

#[derive(Debug)]
pub struct Timer {
    rcl_handle: u64,
    period: Known<i64>,

    callback: Known<Arc<Mutex<Callback>>>,
    node: Known<Weak<Mutex<Node>>>,
}

impl Timer {
    pub fn new(id: u64) -> Self {
        Self {
            rcl_handle: id,
            period: Known::Unknown,
            callback: Known::Unknown,
            node: Known::Unknown,
        }
    }

    pub fn rcl_init(&mut self, period: i64) {
        assert!(
            self.period.is_unknown(),
            "Timer already initialized with rcl_timer_init. {self:#?}"
        );
        self.period = Known::new(period);
    }

    pub fn set_callback(&mut self, callback: Arc<Mutex<Callback>>) {
        assert!(
            self.callback.is_unknown(),
            "Timer callback already set. {self:#?}"
        );

        self.callback = Known::new(callback);
    }

    pub fn link_node(&mut self, node: &Arc<Mutex<Node>>) {
        assert!(self.node.is_unknown(), "Timer node already set. {self:#?}");

        self.node = Known::new(Arc::downgrade(node));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallbackType {
    Subscription,
    Service,
    Timer,
}

#[derive(Debug, Unwrap)]
#[unwrap(ref)]
pub enum CallbackCaller {
    Subscription(Weak<Mutex<Subscriber>>),
    Service(Weak<Mutex<Service>>),
    Timer(Weak<Mutex<Timer>>),
}

impl From<CallbackCaller> for CallbackType {
    fn from(caller: CallbackCaller) -> Self {
        match caller {
            CallbackCaller::Subscription(_) => Self::Subscription,
            CallbackCaller::Service(_) => Self::Service,
            CallbackCaller::Timer(_) => Self::Timer,
        }
    }
}

#[derive(Debug)]
pub struct Callback {
    handle: u64,
    caller: Known<CallbackCaller>,
    name: Known<String>,
    running_instance: Option<Arc<Mutex<CallbackInstance>>>,
}

impl Callback {
    fn new(handle: u64, caller: CallbackCaller) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            handle,
            caller: Known::Known(caller),
            name: Known::Unknown,
            running_instance: None,
        }))
    }

    pub fn new_subscription(handle: u64, caller: &Arc<Mutex<Subscriber>>) -> Arc<Mutex<Self>> {
        let caller = CallbackCaller::Subscription(Arc::downgrade(caller));
        Self::new(handle, caller)
    }

    pub fn new_service(handle: u64, caller: &Arc<Mutex<Service>>) -> Arc<Mutex<Self>> {
        let caller = CallbackCaller::Service(Arc::downgrade(caller));
        Self::new(handle, caller)
    }

    pub fn new_timer(handle: u64, caller: &Arc<Mutex<Timer>>) -> Arc<Mutex<Self>> {
        let caller = CallbackCaller::Timer(Arc::downgrade(caller));
        Self::new(handle, caller)
    }

    pub fn set_name(&mut self, name: String) {
        assert!(
            self.name.is_unknown(),
            "Callback name already set. {self:#?}"
        );

        self.name = Known::new(name);
    }

    pub fn take_running_instance(&mut self) -> Option<Arc<Mutex<CallbackInstance>>> {
        self.running_instance.take()
    }

    pub fn is_timer_driven(&self) -> bool {
        matches!(self.caller, Known::Known(CallbackCaller::Timer(_)))
    }

    pub fn is_service_driven(&self) -> bool {
        matches!(self.caller, Known::Known(CallbackCaller::Service(_)))
    }

    pub fn is_subscription_driven(&self) -> bool {
        matches!(self.caller, Known::Known(CallbackCaller::Subscription(_)))
    }

    pub fn get_type(&self) -> Option<CallbackType> {
        self.caller
            .as_ref()
            .map(|caller| match caller {
                CallbackCaller::Subscription(_) => CallbackType::Subscription,
                CallbackCaller::Service(_) => CallbackType::Service,
                CallbackCaller::Timer(_) => CallbackType::Timer,
            })
            .into()
    }

    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref().into()
    }

    pub fn get_caller(&self) -> Option<&CallbackCaller> {
        self.caller.as_ref().into()
    }
}

#[derive(Debug)]
pub struct PublicationMessage {
    ptr: u64,
    publisher: Known<Arc<Mutex<Publisher>>>,
    sender_timestamp: Known<Time>,
    rmw_publish_time: Known<Time>,
    rcl_publish_time: Known<Time>,
    rclcpp_publish_time: Known<Time>,
}

impl PublicationMessage {
    pub(crate) fn new(message: u64) -> Self {
        Self {
            ptr: message,
            publisher: Known::Unknown,
            sender_timestamp: Known::Unknown,
            rmw_publish_time: Known::Unknown,
            rcl_publish_time: Known::Unknown,
            rclcpp_publish_time: Known::Unknown,
        }
    }

    pub fn get_publisher(&self) -> Option<Arc<Mutex<Publisher>>> {
        self.publisher.clone().into()
    }

    pub(crate) fn set_publisher(&mut self, publisher: Arc<Mutex<Publisher>>) {
        assert!(
            self.publisher.is_unknown(),
            "PublicationMessage publisher already set. {self:#?}"
        );
        self.publisher = Known::new(publisher);
    }

    pub(crate) fn rclcpp_publish(&mut self, time: Time) {
        assert!(
            self.rclcpp_publish_time.is_unknown(),
            "PublicationMessage rclcpp_publish_time already set. {self:#?}"
        );
        self.rclcpp_publish_time = Known::new(time);
    }

    pub(crate) fn rcl_publish(&mut self, time: Time) {
        assert!(
            self.rcl_publish_time.is_unknown(),
            "PublicationMessage rcl_publish_time already set. {self:#?}"
        );
        self.rcl_publish_time = Known::new(time);
    }

    pub(crate) fn rmw_publish(&mut self, time: Time, timestamp: i64) {
        assert!(
            self.rmw_publish_time.is_unknown() && self.sender_timestamp.is_unknown(),
            "PublicationMessage rmw_publish already called on this message. {self:#?}"
        );
        self.rmw_publish_time = Known::new(time);
        self.sender_timestamp = Known::new(Time::from_nanos(timestamp));
    }

    pub(crate) fn rmw_publish_old(&mut self, time: Time) {
        assert!(
            self.rmw_publish_time.is_unknown(),
            "PublicationMessage rmw_publish already called on this message. {self:#?}"
        );
        self.rmw_publish_time = Known::new(time);
    }

    pub fn get_rmw_publication_time(&self) -> Option<Time> {
        self.rmw_publish_time.into()
    }

    pub fn get_rcl_publication_time(&self) -> Option<Time> {
        self.rcl_publish_time.into()
    }

    pub fn get_rclcpp_publication_time(&self) -> Option<Time> {
        self.rclcpp_publish_time.into()
    }

    pub fn get_publication_time(&self) -> Option<Time> {
        self.get_rclcpp_publication_time()
            .or_else(|| self.get_rcl_publication_time())
            .or_else(|| self.get_rmw_publication_time())
    }
}

#[derive(Debug, Default)]
pub enum PartiallyKnown<T, U> {
    Fully(T),
    Partially(U),
    #[default]
    Unknown,
}
#[derive(Debug)]
pub struct SubscriptionMessage {
    ptr: u64,
    message: PartiallyKnown<Arc<Mutex<PublicationMessage>>, Time>,
    subscriber: Known<Arc<Mutex<Subscriber>>>,
    rmw_receive_time: Known<Time>,
    rcl_receive_time: Known<Time>,
    rclcpp_receive_time: Known<Time>,
}

impl SubscriptionMessage {
    pub fn new(ptr: u64) -> Self {
        Self {
            ptr,
            message: PartiallyKnown::Unknown,
            subscriber: Known::Unknown,
            rmw_receive_time: Known::Unknown,
            rcl_receive_time: Known::Unknown,
            rclcpp_receive_time: Known::Unknown,
        }
    }

    pub fn rmw_take_matched(
        &mut self,
        subscriber: Arc<Mutex<Subscriber>>,
        published_message: Arc<Mutex<PublicationMessage>>,
        time: Time,
    ) {
        assert!(
            self.subscriber.is_unknown(),
            "SubscriptionMessage subscriber already set. {self:#?}"
        );
        self.subscriber = Known::new(subscriber);
        self.message = PartiallyKnown::Fully(published_message);
        self.rmw_receive_time = Known::new(time);
    }

    pub fn rmw_take_unmatched(
        &mut self,
        subscriber: Arc<Mutex<Subscriber>>,
        publication_time: i64,
        time: Time,
    ) {
        assert!(
            self.subscriber.is_unknown(),
            "SubscriptionMessage subscriber already set. {self:#?}"
        );
        self.subscriber = Known::new(subscriber);
        self.message = PartiallyKnown::Partially(Time::from_nanos(publication_time));
        self.rmw_receive_time = Known::new(time);
    }

    pub fn rcl_take(&mut self, time: Time) -> Result<(), AlreadySetError<&Self, Time>> {
        if !self.rcl_receive_time.is_unknown() {
            return Err(AlreadySetError {
                object: self,
                new_value: time,
                msg: "SubscriptionMessage rcl_receive_time already set.",
            });
        }
        self.rcl_receive_time = Known::new(time);

        Ok(())
    }

    pub fn rclcpp_take(&mut self, time: Time) -> Result<(), AlreadySetError<&Self, Time>> {
        if !self.rclcpp_receive_time.is_unknown() {
            return Err(AlreadySetError {
                object: self,
                new_value: time,
                msg: "SubscriptionMessage rclcpp_receive_time already set.",
            });
        }
        self.rclcpp_receive_time = Known::new(time);

        Ok(())
    }

    pub fn get_publication_message(&self) -> Option<Arc<Mutex<PublicationMessage>>> {
        match &self.message {
            PartiallyKnown::Fully(message) => Some(message.clone()),
            _ => None,
        }
    }

    pub fn get_sender_timestamp(&self) -> Option<Time> {
        match &self.message {
            PartiallyKnown::Partially(time) => Some(*time),
            _ => None,
        }
    }

    pub fn get_rmw_receive_time(&self) -> Option<Time> {
        self.rmw_receive_time.into()
    }

    pub fn get_rcl_receive_time(&self) -> Option<Time> {
        self.rcl_receive_time.into()
    }

    pub fn get_rclcpp_receive_time(&self) -> Option<Time> {
        self.rclcpp_receive_time.into()
    }

    pub fn get_receive_time(&self) -> Option<Time> {
        self.get_rclcpp_receive_time()
            .or_else(|| self.get_rcl_receive_time())
            .or_else(|| self.get_rmw_receive_time())
    }

    pub fn get_subscriber(&self) -> Option<Arc<Mutex<Subscriber>>> {
        self.subscriber.clone().into()
    }
}

pub enum Gid {
    DdsGid(DdsGid),
    RmwGid(RmwGid),
}

pub struct DdsGid {
    gid: [u8; GID_SIZE],
}

pub struct RmwGid {
    gid: DdsGid,
    gid_suffix: [u8; GID_SUFFIX_SIZE],
}

impl RmwGid {
    pub fn new(gid: [u8; raw_events::ros2::GID_SIZE]) -> Self {
        let gid_main = gid[..GID_SIZE].try_into().unwrap();
        let gid_suffix = gid[GID_SIZE..].try_into().unwrap();
        Self {
            gid: DdsGid { gid: gid_main },
            gid_suffix,
        }
    }
}

#[derive(Debug)]
pub enum CallbackTrigger {
    SubscriptionMessage(Arc<Mutex<SubscriptionMessage>>),
    Service(Arc<Mutex<Service>>),
    Timer(Arc<Mutex<Timer>>),
}

impl From<CallbackTrigger> for CallbackType {
    fn from(trigger: CallbackTrigger) -> Self {
        match trigger {
            CallbackTrigger::SubscriptionMessage(_) => Self::Subscription,
            CallbackTrigger::Service(_) => Self::Service,
            CallbackTrigger::Timer(_) => Self::Timer,
        }
    }
}

#[derive(Debug)]
pub struct CallbackInstance {
    start_time: Time,
    end_time: Known<Time>,
    callback: Arc<Mutex<Callback>>,
    trigger: CallbackTrigger,
}

impl CallbackInstance {
    pub fn new(callback: Arc<Mutex<Callback>>, start_time: Time) -> Arc<Mutex<Self>> {
        let callback_arc = callback;
        let mut callback = callback_arc.lock().unwrap();
        assert!(
            callback.running_instance.is_none(),
            "Callback already has a running instance. {callback:#?}"
        );

        let trigger = match callback.caller.as_ref().unwrap() {
            CallbackCaller::Subscription(weak) => {
                let subscriber = weak.upgrade().unwrap();
                let mut subscriber = subscriber.lock().unwrap();
                let message = subscriber.take_message().unwrap();
                CallbackTrigger::SubscriptionMessage(message)
            }
            CallbackCaller::Service(weak) => {
                let service = weak.upgrade().unwrap();
                CallbackTrigger::Service(service)
            }
            CallbackCaller::Timer(weak) => {
                let timer = weak.upgrade().unwrap();
                CallbackTrigger::Timer(timer)
            }
        };

        let new = Arc::new(Mutex::new(Self {
            start_time,
            end_time: Known::Unknown,
            callback: callback_arc.clone(),
            trigger,
        }));

        callback.running_instance = Some(new.clone());

        new
    }

    pub fn end(&mut self, time: Time) {
        assert!(
            self.end_time.is_unknown(),
            "CallbackInstance end_time already set. {self:#?}"
        );

        self.end_time = Known::Known(time);
    }

    pub fn get_callback(&self) -> Arc<Mutex<Callback>> {
        self.callback.clone()
    }

    pub fn get_start_time(&self) -> Time {
        self.start_time
    }

    pub fn get_end_time(&self) -> Option<Time> {
        self.end_time.into()
    }

    pub fn get_trigger(&self) -> &CallbackTrigger {
        &self.trigger
    }
}

#[derive(Debug)]
pub struct SpinInstance {
    start_time: Time,
    wake_time: Known<Time>,
    end_time: Known<Time>,
    timeout: Duration,
    timeouted: Known<bool>,
    node: Weak<Mutex<Node>>,
}

impl SpinInstance {
    pub fn new(node: &Arc<Mutex<Node>>, start_time: Time, timeout: Duration) -> Arc<Mutex<Self>> {
        let new = Arc::new(Mutex::new(Self {
            start_time,
            wake_time: Known::Unknown,
            end_time: Known::Unknown,
            timeout,
            timeouted: Known::Unknown,
            node: Arc::downgrade(node),
        }));

        let mut node = node.lock().unwrap();
        assert!(
            node.spin_instance.is_none(),
            "Node already has a running spin instance. {node:#?}"
        );

        node.spin_instance = Some(new.clone());

        new
    }

    pub fn set_wake_time(&mut self, time: Time) {
        assert!(
            self.wake_time.is_unknown(),
            "SpinInstance wake_time already set. {self:#?}"
        );

        self.timeouted = Known::Known(false);
        self.wake_time = Known::Known(time);
    }

    pub fn set_end_time(&mut self, time: Time) {
        assert!(
            self.end_time.is_unknown() && (!Option::from(self.timeouted).unwrap_or(false)),
            "SpinInstance end_time already set. {self:#?}"
        );

        self.timeouted = Known::Known(false);
        self.end_time = Known::Known(time);
    }

    pub fn set_timeouted_on(&mut self, wake_time: Time) {
        assert!(
            self.timeouted.is_unknown(),
            "SpinInstance timeouted already set. {self:#?}"
        );

        self.timeouted = Known::Known(true);
        self.wake_time = Known::Known(wake_time);
    }
}
