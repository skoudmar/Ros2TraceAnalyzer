pub(crate) mod display;

use std::collections::VecDeque;
use std::fmt::Debug;
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use chrono::{Local, TimeZone};
use derive_more::derive::Unwrap;
use display::{
    get_service_name_from_weak, get_subscriber_topic_from_weak, get_timer_period_from_weak,
};
use thiserror::Error;

use crate::raw_events;
use crate::utils::{ArcWeak, CyclicDependency, DisplayDuration, Known, WeakKnown};

const GID_SIZE: usize = 16;
const GID_SUFFIX_SIZE: usize = 8;

#[derive(Debug, Error)]
#[error("Object parameter already set: {msg}\nobject={object:#?}\nnew_value={new_value:#?}")]
pub struct AlreadySetError<T: Debug, U: Debug> {
    object: T,
    new_value: U,
    msg: &'static str,
}

#[derive(Debug, Error)]
#[error("{object} already initialized by event {event_name}")]
pub struct AlreadyInitializedError {
    object: &'static str,
    event_name: &'static str,
}

impl AlreadyInitializedError {
    pub const fn new(object: &'static str, event_name: &'static str) -> Self {
        Self { object, event_name }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

    pub fn replace_spin_instance(
        &mut self,
        spin_instance: Arc<Mutex<SpinInstance>>,
    ) -> Option<Arc<Mutex<SpinInstance>>> {
        self.spin_instance.replace(spin_instance)
    }

    pub fn borrow_spin_instance(&self) -> Option<&Arc<Mutex<SpinInstance>>> {
        self.spin_instance.as_ref()
    }

    pub fn print_node_info(&self) {
        println!("Node{self}");
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
    rmw_handle: Known<u64>,
    rcl_handle: Known<u64>,
    rclcpp_handle: Known<u64>,

    rmw_gid: Known<RmwGid>,

    topic_name: Known<String>,
    node: Known<ArcWeak<Mutex<Node>>>,
    queue_depth: Known<usize>,

    callback: Known<Arc<Mutex<Callback>>>,
    // TODO: messages that are taken but not yet processed by the callback cause a memory leak
    // at the end of the program. (strong reference count cycle: Subscriber -> Message -> Subscriber)
    // For now, we just limit the number of messages stored here.
    taken_message: VecDeque<Arc<Mutex<SubscriptionMessage>>>,

    removed: bool,
}

impl Subscriber {
    // The maximum number of taken messages that the subscriber stores.
    // This constant was chosen to match the number of messages stored by the R2R's channels.
    const TAKEN_MESSAGES_MAX: usize = 11;

    pub fn rmw_init(
        &mut self,
        rmw_handle: u64,
        gid: [u8; raw_events::ros2::GID_SIZE],
    ) -> Result<(), AlreadyInitializedError> {
        assert!(!self.is_removed());
        if self.rmw_handle.is_unknown_or_eq(&rmw_handle)
            && self.rmw_gid.is_unknown_or_eq(&RmwGid::new(gid))
        {
            self.rmw_handle = Known::new(rmw_handle);
            self.rmw_gid = Known::new(RmwGid::new(gid));
            Ok(())
        } else {
            Err(AlreadyInitializedError::new(
                "Subscriber",
                "rmw_init_subscription",
            ))
        }
    }

    pub fn rcl_init(
        &mut self,
        rcl_handle: u64,
        topic_name: String,
        queue_depth: usize,
        node: Weak<Mutex<Node>>,
    ) -> Result<(), AlreadyInitializedError> {
        assert!(!self.is_removed());
        let node = node.into();
        if self.rcl_handle.is_unknown() {
            self.rcl_handle = Known::new(rcl_handle);
            self.topic_name = Known::new(topic_name);
            self.queue_depth = Known::new(queue_depth);
            self.node = Known::new(node);
            Ok(())
        } else {
            Err(AlreadyInitializedError::new(
                "Subscriber",
                "rcl_subscription_init",
            ))
        }
    }

    pub fn rclcpp_init(&mut self, rclcpp_handle: u64) -> Result<(), AlreadyInitializedError> {
        assert!(!self.is_removed());
        if self.rclcpp_handle.is_unknown() {
            self.rclcpp_handle = Known::new(rclcpp_handle);
            Ok(())
        } else {
            Err(AlreadyInitializedError::new(
                "Subscriber",
                "rclcpp_subscription_init",
            ))
        }
    }

    pub(crate) fn set_rmw_handle(
        &mut self,
        rmw_subscription_handle: u64,
    ) -> Result<(), AlreadySetError<&Self, u64>> {
        assert!(!self.is_removed());
        if self.rmw_handle.is_unknown_or_eq(&rmw_subscription_handle) {
            self.rmw_handle = Known::new(rmw_subscription_handle);
            Ok(())
        } else {
            Err(AlreadySetError {
                object: self,
                new_value: rmw_subscription_handle,
                msg: "rmw_handle already set",
            })
        }
    }

    pub fn set_callback(
        &mut self,
        callback: Arc<Mutex<Callback>>,
    ) -> Result<(), AlreadySetError<&Self, Arc<Mutex<Callback>>>> {
        assert!(!self.is_removed());
        if self.callback.is_unknown() {
            self.callback = Known::new(callback);
            Ok(())
        } else {
            Err(AlreadySetError {
                object: self,
                new_value: callback,
                msg: "callback already set",
            })
        }
    }

    pub fn replace_taken_message(
        &mut self,
        message: Arc<Mutex<SubscriptionMessage>>,
    ) -> Option<Arc<Mutex<SubscriptionMessage>>> {
        assert!(!self.is_removed());

        // We store a maximum of `TAKEN_MESSAGES_MAX` messages. These messages are used by callbacks.
        // Usually, in C++ on single-threaded executor only one message is stored. However, in Rust r2r
        // multiple messages can be taken before the callback is called. This is why we store multiple
        // messages.
        // Because Python do not include tracepoints for callbacks but do for message taking (in rcl and rmw),
        // messages are added but not removed.
        // We need to bound the number of messages stored.
        // TODO: Find better way to handle this
        let ret = if self.taken_message.len() >= Self::TAKEN_MESSAGES_MAX {
            self.taken_message.pop_front()
        } else {
            None
        };

        self.taken_message.push_back(message);

        ret
    }

    pub fn get_rmw_handle(&self) -> Known<u64> {
        self.rmw_handle
    }

    pub fn get_rcl_handle(&self) -> Known<u64> {
        self.rcl_handle
    }

    pub fn get_rclcpp_handle(&self) -> Known<u64> {
        self.rclcpp_handle
    }

    pub fn take_message(&mut self) -> Option<Arc<Mutex<SubscriptionMessage>>> {
        self.taken_message.pop_front()
    }

    pub fn get_topic(&self) -> Known<&str> {
        self.topic_name.as_deref()
    }

    pub fn get_node(&self) -> Known<ArcWeak<Mutex<Node>>> {
        self.node.clone()
    }

    pub fn get_callback(&self) -> Known<Arc<Mutex<Callback>>> {
        self.callback.clone()
    }

    pub fn mark_removed(&mut self) {
        self.removed = true;
    }

    pub fn is_removed(&self) -> bool {
        self.removed
    }
}

impl CyclicDependency for Subscriber {
    fn break_cycle(&mut self) {
        self.node.as_mut().map(ArcWeak::downgrade_in_place);
    }

    fn create_cycle(&mut self) -> bool {
        self.node
            .as_mut()
            .map(ArcWeak::upgrade_in_place)
            .unwrap_or(true)
    }
}

#[derive(Debug, Default)]
pub struct Publisher {
    rmw_handle: Known<u64>,
    rcl_handle: Known<u64>,
    rclcpp_handle: Known<u64>,

    rmw_gid: Known<RmwGid>,

    topic_name: Known<String>,
    node: Known<ArcWeak<Mutex<Node>>>,
    queue_depth: Known<usize>,

    removed: bool,
}

impl Publisher {
    pub fn rmw_init(
        &mut self,
        rmw_handle: u64,
        gid: [u8; raw_events::ros2::GID_SIZE],
    ) -> Result<(), AlreadyInitializedError> {
        assert!(!self.is_removed());
        if self.rmw_handle.is_unknown_or_eq(&rmw_handle)
            && self.rmw_gid.is_unknown_or_eq(&RmwGid::new(gid))
        {
            self.rmw_handle = Known::new(rmw_handle);
            self.rmw_gid = Known::new(RmwGid::new(gid));
            Ok(())
        } else {
            Err(AlreadyInitializedError::new(
                "Publisher",
                "rmw_init_publisher",
            ))
        }
    }

    pub fn rcl_init(
        &mut self,
        rcl_handle: u64,
        topic_name: String,
        queue_depth: usize,
        node: Weak<Mutex<Node>>,
    ) -> Result<(), AlreadyInitializedError> {
        assert!(!self.is_removed());
        if self.rcl_handle.is_unknown() {
            self.rcl_handle = Known::new(rcl_handle);
            self.topic_name = Known::new(topic_name);
            self.queue_depth = Known::new(queue_depth);
            self.node = Known::new(node.into());
            Ok(())
        } else {
            Err(AlreadyInitializedError::new(
                "Publisher",
                "rcl_publisher_init",
            ))
        }
    }

    pub(crate) fn set_rmw_handle(
        &mut self,
        rmw_publisher_handle: u64,
    ) -> Result<(), AlreadySetError<&Self, u64>> {
        assert!(!self.is_removed());
        if self.rmw_handle.is_unknown_or_eq(&rmw_publisher_handle) {
            self.rmw_handle = Known::new(rmw_publisher_handle);
            Ok(())
        } else {
            Err(AlreadySetError {
                object: self,
                new_value: rmw_publisher_handle,
                msg: "rmw_handle already set",
            })
        }
    }

    pub fn set_rcl_handle(
        &mut self,
        rcl_publisher_handle: u64,
    ) -> Result<(), AlreadySetError<&Self, u64>> {
        assert!(!self.is_removed());
        if self.rcl_handle.is_unknown_or_eq(&rcl_publisher_handle) {
            self.rcl_handle = Known::new(rcl_publisher_handle);
            Ok(())
        } else {
            Err(AlreadySetError {
                object: self,
                new_value: rcl_publisher_handle,
                msg: "rcl_handle already set",
            })
        }
    }

    pub fn get_rmw_handle(&self) -> Known<u64> {
        self.rmw_handle
    }

    pub fn get_rcl_handle(&self) -> Known<u64> {
        self.rcl_handle
    }

    pub fn get_topic(&self) -> Known<&str> {
        self.topic_name.as_deref()
    }

    pub fn get_node(&self) -> Known<ArcWeak<Mutex<Node>>> {
        self.node.clone()
    }

    pub fn mark_removed(&mut self) {
        self.removed = true;
    }

    /// Returns whether the publisher is a stub.
    ///
    /// A stub publisher is a publisher that has not been initialized by any init method.
    pub fn is_stub(&self) -> bool {
        self.rmw_gid.is_unknown()
            && self.topic_name.is_unknown()
            && self.node.is_unknown()
            && self.queue_depth.is_unknown()
    }

    pub fn change_rcl_handle(&mut self, rcl_handle: u64) {
        self.rcl_handle = Known::new(rcl_handle);
    }

    pub fn is_removed(&self) -> bool {
        self.removed
    }
}

impl CyclicDependency for Publisher {
    fn break_cycle(&mut self) {
        self.node.as_mut().map(ArcWeak::downgrade_in_place);
    }

    fn create_cycle(&mut self) -> bool {
        self.node
            .as_mut()
            .map(ArcWeak::upgrade_in_place)
            .unwrap_or(true)
    }
}

#[derive(Debug)]
pub struct Service {
    rmw_handle: Known<u64>,
    rcl_handle: u64,
    rclcpp_handle: Known<u64>,

    name: Known<String>,
    node: Known<ArcWeak<Mutex<Node>>>,
    callback: Known<Arc<Mutex<Callback>>>,

    removed: bool,
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
            removed: false,
        }
    }

    pub fn rcl_init(
        &mut self,
        rmw_handle: u64,
        name: String,
        node: &Arc<Mutex<Node>>,
    ) -> Result<(), AlreadyInitializedError> {
        assert!(!self.is_removed());
        if !self.name.is_unknown()
            || !self.node.is_unknown()
            || !self.rmw_handle.is_unknown_or_eq(&rmw_handle)
        {
            return Err(AlreadyInitializedError::new("Service", "rcl_service_init"));
        }

        self.rmw_handle = Known::new(rmw_handle);
        self.name = Known::new(name);
        self.node = Known::new(Arc::downgrade(node).into());

        Ok(())
    }

    pub fn set_callback(
        &mut self,
        callback: Arc<Mutex<Callback>>,
    ) -> Result<(), AlreadySetError<&Self, Arc<Mutex<Callback>>>> {
        assert!(!self.is_removed());
        if !self.callback.is_unknown() {
            return Err(AlreadySetError {
                object: self,
                new_value: callback,
                msg: "callback already set",
            });
        }

        self.callback = Known::new(callback);
        Ok(())
    }

    pub fn get_node(&self) -> Known<ArcWeak<Mutex<Node>>> {
        self.node.clone()
    }

    pub fn get_name(&self) -> Known<&str> {
        self.name.as_deref()
    }

    pub fn mark_removed(&mut self) {
        self.removed = true;
    }

    pub fn is_removed(&self) -> bool {
        self.removed
    }
}

impl CyclicDependency for Service {
    fn break_cycle(&mut self) {
        self.node.as_mut().map(ArcWeak::downgrade_in_place);
    }

    fn create_cycle(&mut self) -> bool {
        self.node
            .as_mut()
            .map(ArcWeak::upgrade_in_place)
            .unwrap_or(true)
    }
}

#[derive(Debug, Default)]
pub struct Client {
    rcl_handle: u64,
    rmw_handle: Known<u64>,
    node: Known<ArcWeak<Mutex<Node>>>,
    service_name: Known<String>,

    removed: bool,
}

impl Client {
    pub fn new(id: u64) -> Self {
        Self {
            rcl_handle: id,
            rmw_handle: Known::Unknown,
            node: Known::Unknown,
            service_name: Known::Unknown,
            removed: false,
        }
    }

    pub fn rcl_init(
        &mut self,
        rmw_handle: u64,
        service_name: String,
        node: &Arc<Mutex<Node>>,
    ) -> Result<(), AlreadyInitializedError> {
        assert!(!self.is_removed());

        if !self.rmw_handle.is_unknown()
            || !self.node.is_unknown()
            || !self.service_name.is_unknown()
        {
            return Err(AlreadyInitializedError::new("Client", "rcl_client_init"));
        }
        self.rmw_handle = Known::new(rmw_handle);
        self.node = Known::new(Arc::downgrade(node).into());
        self.service_name = Known::new(service_name);

        Ok(())
    }

    pub fn mark_removed(&mut self) {
        self.removed = true;
    }

    pub fn is_removed(&self) -> bool {
        self.removed
    }
}

impl CyclicDependency for Client {
    fn break_cycle(&mut self) {
        self.node.as_mut().map(ArcWeak::downgrade_in_place);
    }

    fn create_cycle(&mut self) -> bool {
        self.node
            .as_mut()
            .map(ArcWeak::upgrade_in_place)
            .unwrap_or(true)
    }
}

#[derive(Debug)]
pub struct Timer {
    rcl_handle: u64,
    period: Known<i64>,

    callback: Known<Arc<Mutex<Callback>>>,
    node: Known<ArcWeak<Mutex<Node>>>,

    removed: bool,
}

impl Timer {
    pub fn new(id: u64) -> Self {
        Self {
            rcl_handle: id,
            period: Known::Unknown,
            callback: Known::Unknown,
            node: Known::Unknown,
            removed: false,
        }
    }

    pub fn rcl_init(&mut self, period: i64) -> Result<(), AlreadyInitializedError> {
        assert!(!self.is_removed());
        if !self.period.is_unknown() {
            return Err(AlreadyInitializedError::new("Timer", "rcl_timer_init"));
        }

        self.period = Known::new(period);
        Ok(())
    }

    pub fn set_callback(
        &mut self,
        callback: Arc<Mutex<Callback>>,
    ) -> Result<(), AlreadySetError<&Self, Arc<Mutex<Callback>>>> {
        assert!(!self.is_removed());
        if !self.callback.is_unknown() {
            return Err(AlreadySetError {
                object: self,
                new_value: callback,
                msg: "Timer callback already set",
            });
        }

        self.callback = Known::new(callback);
        Ok(())
    }

    pub fn link_node(
        &mut self,
        node: &Arc<Mutex<Node>>,
    ) -> Result<(), AlreadySetError<&Self, Arc<Mutex<Node>>>> {
        assert!(!self.is_removed());
        if !self.node.is_unknown() {
            return Err(AlreadySetError {
                object: self,
                new_value: Arc::clone(node),
                msg: "Timer node already set",
            });
        }

        self.node = Known::new(Arc::downgrade(node).into());
        Ok(())
    }

    pub fn get_period(&self) -> Known<i64> {
        self.period
    }

    pub fn get_node(&self) -> Known<ArcWeak<Mutex<Node>>> {
        self.node.clone()
    }

    pub fn mark_removed(&mut self) {
        self.removed = true;
    }

    pub fn is_removed(&self) -> bool {
        self.removed
    }
}

impl CyclicDependency for Timer {
    fn break_cycle(&mut self) {
        self.node.as_mut().map(ArcWeak::downgrade_in_place);
    }

    fn create_cycle(&mut self) -> bool {
        self.node
            .as_mut()
            .map(ArcWeak::upgrade_in_place)
            .unwrap_or(true)
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
    Subscription(ArcWeak<Mutex<Subscriber>>),
    Service(ArcWeak<Mutex<Service>>),
    Timer(ArcWeak<Mutex<Timer>>),
}

impl CallbackCaller {
    /// Returns the main information about the caller of the callback.
    ///
    /// For subscriptions, it returns the topic name.
    /// For services, it returns the service name.
    /// For timers, it returns the period.
    pub fn get_caller_as_string(&self) -> WeakKnown<String> {
        match self {
            Self::Subscription(sub) => get_subscriber_topic_from_weak(&sub.get_weak()),
            Self::Service(service) => get_service_name_from_weak(&service.get_weak()),
            Self::Timer(timer) => get_timer_period_from_weak(&timer.get_weak())
                .map(|period| DisplayDuration(period).to_string()),
        }
    }

    /// Returns whether the caller of the callback has been marked as removed
    /// or `None` if its weak reference cannot be upgraded.
    pub fn is_removed(&self) -> Option<bool> {
        match self {
            Self::Subscription(sub) => Some(sub.get_arc()?.lock().unwrap().is_removed()),
            Self::Service(service) => Some(service.get_arc()?.lock().unwrap().is_removed()),
            Self::Timer(timer) => Some(timer.get_arc()?.lock().unwrap().is_removed()),
        }
    }
}

impl CyclicDependency for CallbackCaller {
    fn break_cycle(&mut self) {
        match self {
            Self::Subscription(sub) => sub.downgrade_in_place(),
            Self::Service(service) => service.downgrade_in_place(),
            Self::Timer(timer) => timer.downgrade_in_place(),
        }
    }

    fn create_cycle(&mut self) -> bool {
        match self {
            Self::Subscription(sub) => sub.upgrade_in_place(),
            Self::Service(service) => service.upgrade_in_place(),
            Self::Timer(timer) => timer.upgrade_in_place(),
        }
    }
}

impl From<CallbackCaller> for CallbackType {
    fn from(caller: CallbackCaller) -> Self {
        (&caller).into()
    }
}

impl<'a> From<&'a CallbackCaller> for CallbackType {
    fn from(caller: &'a CallbackCaller) -> Self {
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
    hostname: String,

    is_removed: bool,
}

impl Callback {
    fn new(handle: u64, caller: CallbackCaller, hostname: String) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            handle,
            caller: Known::Known(caller),
            name: Known::Unknown,
            running_instance: None,
            hostname,
            is_removed: false,
        }))
    }

    pub fn new_subscription(
        handle: u64,
        caller: &Arc<Mutex<Subscriber>>,
        hostname: String,
    ) -> Arc<Mutex<Self>> {
        let caller = CallbackCaller::Subscription(Arc::downgrade(caller).into());
        Self::new(handle, caller, hostname)
    }

    pub fn new_service(
        handle: u64,
        caller: &Arc<Mutex<Service>>,
        hostname: String,
    ) -> Arc<Mutex<Self>> {
        let caller = CallbackCaller::Service(Arc::downgrade(caller).into());
        Self::new(handle, caller, hostname)
    }

    pub fn new_timer(
        handle: u64,
        caller: &Arc<Mutex<Timer>>,
        hostname: String,
    ) -> Arc<Mutex<Self>> {
        let caller = CallbackCaller::Timer(Arc::downgrade(caller).into());
        Self::new(handle, caller, hostname)
    }

    pub fn set_name(&mut self, name: String) -> Result<(), AlreadySetError<&Self, String>> {
        assert!(!self.is_removed());
        if self.name.is_unknown() {
            self.name = Known::new(name);
            Ok(())
        } else {
            Err(AlreadySetError {
                object: self,
                new_value: name,
                msg: "callback name already set",
            })
        }
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

    pub fn get_type(&self) -> Known<CallbackType> {
        self.caller.as_ref().map(Into::into)
    }

    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref().into()
    }

    pub fn get_caller(&self) -> Option<&CallbackCaller> {
        self.caller.as_ref().into()
    }

    pub fn get_node(&self) -> Option<ArcWeak<Mutex<Node>>> {
        let caller = Option::<&CallbackCaller>::from(self.caller.as_ref())?;

        match caller {
            CallbackCaller::Subscription(subscriber) => {
                subscriber.get_arc()?.lock().unwrap().get_node().into()
            }
            CallbackCaller::Service(service) => {
                service.get_arc()?.lock().unwrap().get_node().into()
            }
            CallbackCaller::Timer(timer) => timer.get_arc()?.lock().unwrap().get_node().into(),
        }
    }

    pub fn get_hostname(&self) -> &str {
        &self.hostname
    }

    pub fn mark_removed(&mut self) {
        assert!(self
            .get_caller()
            .map_or(Some(true), CallbackCaller::is_removed)
            .unwrap_or(true));
        self.is_removed = true;
    }

    #[inline]
    pub fn is_removed(&self) -> bool {
        self.is_removed
    }
}

impl CyclicDependency for Callback {
    fn break_cycle(&mut self) {
        self.caller.as_mut().map(CyclicDependency::break_cycle);
    }

    fn create_cycle(&mut self) -> bool {
        self.caller
            .as_mut()
            .map(CyclicDependency::create_cycle)
            .unwrap_or(true)
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

    pub(crate) fn replace_publisher(&mut self, publisher: Arc<Mutex<Publisher>>) {
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

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Gid {
    DdsGid(DdsGid),
    RmwGid(RmwGid),
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct DdsGid {
    gid: [u8; GID_SIZE],
}

#[derive(Clone, PartialEq, Eq, Hash)]
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
                let subscriber = weak.get_arc().unwrap();
                let mut subscriber = subscriber.lock().unwrap();
                let message = subscriber.take_message().unwrap_or_else(|| {
                    panic!(
                        "Subscriber does not have a message to trigger the callback at time {start_time}.\n{subscriber:#?}\n{callback:#?}",
                    )
                });
                CallbackTrigger::SubscriptionMessage(message)
            }
            CallbackCaller::Service(weak) => {
                let service = weak.get_arc().unwrap();
                CallbackTrigger::Service(service)
            }
            CallbackCaller::Timer(weak) => {
                let timer = weak.get_arc().unwrap();
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
    node: ArcWeak<Mutex<Node>>,
}

impl SpinInstance {
    pub fn new(node: &Arc<Mutex<Node>>, start_time: Time, timeout: Duration) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            start_time,
            wake_time: Known::Unknown,
            end_time: Known::Unknown,
            timeout,
            timeouted: Known::Unknown,
            node: Arc::downgrade(node).into(),
        }))
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

impl CyclicDependency for SpinInstance {
    fn break_cycle(&mut self) {
        self.node.downgrade_in_place();
    }

    fn create_cycle(&mut self) -> bool {
        self.node.upgrade_in_place()
    }
}
