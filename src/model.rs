use std::sync::{Arc, Mutex, Weak};

use crate::utils::Known;

#[derive(Debug, Default)]
pub struct Node {
    rcl_handle: u64,
    rmw_handle: Known<u64>,
    name: Known<String>,
    namespace: Known<String>,

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

    dds_gid: Known<Gid>,
    rmw_gid: Known<RmwGid>,

    topic_name: Known<String>,
    topic_name_dds: Known<String>,
    node: Known<Weak<Mutex<Node>>>,
    queue_depth: Known<u64>,

    callback: Arc<Callback>,
}

#[derive(Debug, Default)]
pub struct Publisher {
    dds_handle: Known<u64>,
    rmw_handle: Known<u64>,
    rcl_handle: Known<u64>,
    rclcpp_handle: Known<u64>,

    dds_gid: Known<Gid>,
    rmw_gid: Known<RmwGid>,

    topic_name: Known<String>,
    topic_name_dds: Known<String>,
    node: Known<Weak<Mutex<Node>>>,
    queue_depth: Known<u64>,
}

#[derive(Debug, Default)]
pub struct Service {
    rmw_handle: Known<u64>,
    rcl_handle: u64,
    rclcpp_handle: Known<u64>,

    name: Known<String>,
    node: Known<Weak<Mutex<Node>>>,
    callback: Arc<Mutex<Callback>>,
}

#[derive(Debug, Default)]
pub struct Client {}

#[derive(Debug, Default)]
pub struct Timer {
    rcl_handle: u64,
    period: Known<u64>,

    callback: Arc<Mutex<Callback>>,
    node: Known<Weak<Mutex<Node>>>,
}

#[derive(Debug)]
pub enum CallbackCaller {
    Subscription(Weak<Mutex<Subscriber>>),
    Service(Weak<Mutex<Service>>),
    Timer(Weak<Mutex<Timer>>),
}

#[derive(Debug, Default)]
pub struct Callback {
    handle: Known<u64>,
    caller: Known<CallbackCaller>,
    name: Known<String>,
}

pub enum Gid {
    DdsGid(DdsGid),
    RmwGid(RmwGid),
}

pub struct DdsGid {
    gid: [u8; 16],
}

pub struct RmwGid {
    gid: DdsGid,
    gid_suffix: [u8; 8],
}

impl Node {
    pub fn new(rcl_handle: u64) -> Self {
        Self {
            rcl_handle,
            ..Default::default()
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
