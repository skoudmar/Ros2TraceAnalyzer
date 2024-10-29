use std::sync::{Arc, Mutex, Weak};

use chrono::{Local, TimeZone};

use crate::raw_events;
use crate::utils::{DisplayArcMutex, DisplayDebug, DisplayWeakMutex, Known};

const GID_SIZE: usize = 16;
const GID_SUFFIX_SIZE: usize = 8;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Time {
    /// Nanoseconds since the UNIX epoch (1970-01-01 00:00:00 UTC)
    timestamp: i64,
}

impl Time {
    pub fn from_nanos(nanos: i64) -> Self {
        Self { timestamp: nanos }
    }

    pub fn timestamp_nanos(self) -> i64 {
        self.timestamp
    }

    pub fn as_datetime(self) -> chrono::DateTime<chrono::Local> {
        Local.timestamp_nanos(self.timestamp)
    }
}

impl std::fmt::Debug for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Time").field(&self.as_datetime()).finish()
    }
}

impl std::fmt::Display for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_datetime().naive_local())
    }
}

#[derive(Debug, Clone)]
pub struct Name {
    full_name: String,
    namespace_len: usize,
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
}

#[derive(Debug, Default)]
pub struct Subscriber {
    dds_handle: Known<u64>,
    rmw_handle: Known<u64>,
    rcl_handle: Known<u64>,
    rclcpp_handle: Known<u64>,

    rmw_gid: Known<RmwGid>,

    topic_name: Known<String>,
    topic_name_dds: Known<String>,
    node: Known<Weak<Mutex<Node>>>,
    queue_depth: Known<usize>,

    callback: Known<Arc<Mutex<Callback>>>,
    taken_message: Option<Arc<Mutex<SubscriptionMessage>>>,
}

#[derive(Debug, Default)]
pub struct Publisher {
    dds_handle: Known<u64>,
    rmw_handle: Known<u64>,
    rcl_handle: Known<u64>,
    rclcpp_handle: Known<u64>,

    rmw_gid: Known<RmwGid>,

    topic_name: Known<String>,
    topic_name_dds: Known<String>,
    node: Known<Weak<Mutex<Node>>>,
    queue_depth: Known<usize>,
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

#[derive(Debug, Default)]
pub struct Client {
    rcl_handle: u64,
    rmw_handle: Known<u64>,
    node: Known<Weak<Mutex<Node>>>,
    service_name: Known<String>,
}

#[derive(Debug)]
pub struct Timer {
    rcl_handle: u64,
    period: Known<i64>,

    callback: Known<Arc<Mutex<Callback>>>,
    node: Known<Weak<Mutex<Node>>>,
}

#[derive(Debug)]
pub enum CallbackCaller {
    Subscription(Weak<Mutex<Subscriber>>),
    Service(Weak<Mutex<Service>>),
    Timer(Weak<Mutex<Timer>>),
}

#[derive(Debug)]
pub struct Callback {
    handle: u64,
    caller: Known<CallbackCaller>,
    name: Known<String>,
    running_instance: Option<Arc<Mutex<CallbackInstance>>>,
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

#[derive(Debug)]
pub struct PublicationMessage {
    ptr: u64,
    publisher: Known<Arc<Mutex<Publisher>>>,
    sender_timestamp: Known<Time>,
    rmw_publish_time: Known<Time>,
    rcl_publish_time: Known<Time>,
    rclcpp_publish_time: Known<Time>,
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

    pub fn rcl_take(&mut self, time: Time) {
        assert!(
            self.rcl_receive_time.is_unknown(),
            "SubscriptionMessage rcl_receive_time already set. {self:#?}"
        );
        self.rcl_receive_time = Known::new(time);
    }

    pub fn rclcpp_take(&mut self, time: Time) {
        assert!(
            self.rclcpp_receive_time.is_unknown(),
            "SubscriptionMessage rclcpp_receive_time already set. {self:#?}"
        );
        self.rclcpp_receive_time = Known::new(time);
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
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.get_full_name().map(DisplayDebug);
        write!(
            f,
            "(name={name}, handles={{rmw={:x}, rcl={:x}}})",
            self.rmw_handle, self.rcl_handle
        )
    }
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
}

impl std::fmt::Display for Publisher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let node = self
            .node
            .as_ref()
            .map(|node| DisplayWeakMutex::new(node, f.alternate()));

        write!(f, "(topic={}, handles={{dds={:x}, rmw={:x}, rcl={:x}, rclcpp={:x}}}, queue_depth={}, node={node})",
        self.topic_name.as_ref().map(DisplayDebug), self.dds_handle, self.rmw_handle, self.rcl_handle, self.rclcpp_handle, self.queue_depth)
    }
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
        self.topic_name.as_ref().map(String::as_str)
    }
}

impl std::fmt::Display for Subscriber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let node = self
            .node
            .as_ref()
            .map(|node| DisplayWeakMutex::new(node, false));
        let callback = self
            .callback
            .as_ref()
            .map(|callback| DisplayArcMutex::new(callback, f.alternate()));

        write!(f, "(topic={}, handles={{dds={:x}, rmw={:x}, rcl={:x}, rclcpp={:x}}}, queue_depth={}, node={node} callback={callback:#})",
            self.topic_name.as_ref().map(DisplayDebug), self.dds_handle, self.rmw_handle, self.rcl_handle, self.rclcpp_handle, self.queue_depth,
        )
    }
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

impl std::fmt::Display for Service {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let node = self
            .node
            .as_ref()
            .map(|node| DisplayWeakMutex::new(node, false));
        let callback = self
            .callback
            .as_ref()
            .map(|callback| DisplayArcMutex::new(callback, f.alternate()));

        write!(f, "(service={}, handles={{rmw={:x}, rcl={:x}, rclcpp={:x}}}, node={node} callback={callback:#})",
            self.name.as_ref().map(DisplayDebug), self.rmw_handle, self.rcl_handle, self.rclcpp_handle,
        )
    }
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

impl std::fmt::Display for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let node = self
            .node
            .as_ref()
            .map(|node| DisplayWeakMutex::new(node, f.alternate()));

        write!(
            f,
            "(service={}, handles={{rmw={:x}, rcl={:x}}}, node={node})",
            self.service_name.as_ref().map(DisplayDebug),
            self.rmw_handle,
            self.rcl_handle
        )
    }
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

impl std::fmt::Display for Timer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let node = self
            .node
            .as_ref()
            .map(|node| DisplayWeakMutex::new(node, false));
        let callback = self
            .callback
            .as_ref()
            .map(|callback| DisplayArcMutex::new(callback, f.alternate()));

        write!(
            f,
            "(period={}, rcl_handle={}, node={node} callback={callback:#})",
            self.period, self.rcl_handle,
        )
    }
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
}

impl std::fmt::Display for Callback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(
                f,
                "(handle={:x}, name={})",
                self.handle,
                self.name.as_ref().map(DisplayDebug)
            )
        } else {
            match &self.caller {
                Known::Known(CallbackCaller::Subscription(subscriber)) => {
                    write!(
                        f,
                        "(handle={:x}, caller=Subscriber{}, name={})",
                        self.handle,
                        DisplayWeakMutex::new(subscriber, false),
                        self.name.as_ref().map(DisplayDebug)
                    )
                }
                Known::Known(CallbackCaller::Service(service)) => {
                    write!(
                        f,
                        "(handle={:x}, caller=Service{}, name={})",
                        self.handle,
                        DisplayWeakMutex::new(service, false),
                        self.name.as_ref().map(DisplayDebug)
                    )
                }
                Known::Known(CallbackCaller::Timer(timer)) => {
                    write!(
                        f,
                        "(handle={:x}, caller=Timer{}, name={})",
                        self.handle,
                        DisplayWeakMutex::new(timer, false),
                        self.name.as_ref().map(DisplayDebug)
                    )
                }
                Known::Unknown => {
                    write!(
                        f,
                        "(handle={:x}, caller=Unknown, name={})",
                        self.handle,
                        self.name.as_ref().map(DisplayDebug)
                    )
                }
            }
        }
    }
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

impl std::fmt::Display for DdsGid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.gid.iter().take(12) {
            write!(f, "{byte:02x}")?;
        }
        write!(f, "-")?;
        for byte in self.gid.iter().skip(12) {
            write!(f, "{byte:02x}")?;
        }

        Ok(())
    }
}

impl std::fmt::Debug for DdsGid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(stringify!(DdsGid))
            .field(&self.to_string())
            .finish()
    }
}

impl std::fmt::Display for RmwGid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-", self.gid)?;
        for byte in &self.gid_suffix {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for RmwGid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(stringify!(RmwGid))
            .field(&self.to_string())
            .finish()
    }
}

impl std::fmt::Display for Gid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Gid::DdsGid(gid) => write!(f, "{gid}"),
            Gid::RmwGid(gid) => write!(f, "{gid}"),
        }
    }
}

impl std::fmt::Debug for Gid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Gid::DdsGid(gid) => std::fmt::Debug::fmt(gid, f),
            Gid::RmwGid(gid) => std::fmt::Debug::fmt(gid, f),
        }
    }
}

impl std::fmt::Display for PublicationMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let publisher = self
            .publisher
            .as_ref()
            .map(|publisher| publisher.lock().unwrap());
        write!(
            f,
            "(ptr={:x}, sender_timestamp={}, publisher={publisher})",
            self.ptr, self.sender_timestamp
        )
    }
}

impl std::fmt::Display for SubscriptionMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let subscriber = self
            .subscriber
            .as_ref()
            .map(|subscriber| subscriber.lock().unwrap());
        match &self.message {
            PartiallyKnown::Fully(message) => {
                let message = message.lock().unwrap();
                write!(
                    f,
                    "(ptr={:x}, message={message}, subscriber={subscriber})",
                    self.ptr
                )
            }
            PartiallyKnown::Partially(timestamp) => {
                write!(
                    f,
                    "(ptr={:x}, message=(sender_timestamp={timestamp}), subscriber={subscriber})",
                    self.ptr
                )
            }
            PartiallyKnown::Unknown => {
                write!(
                    f,
                    "(ptr={:x}, message=Unknown, subscriber={subscriber})",
                    self.ptr
                )
            }
        }
    }
}

#[derive(Debug)]
pub enum CallbackTrigger {
    SubscriptionMessage(Arc<Mutex<SubscriptionMessage>>),
    Service(Arc<Mutex<Service>>),
    Timer(Arc<Mutex<Timer>>),
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
}

impl std::fmt::Display for CallbackInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let callback = self.callback.lock().unwrap();
        write!(
            f,
            "(start_time={}, end_time={}, callback={callback})",
            self.start_time, self.end_time,
        )
    }
}
