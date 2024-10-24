use bt2_sys::event::BtEventConst;
use derive_more::derive::From;

use crate::impl_from_for_enum;

use super::FromBtEvent;

pub const GID_SIZE: usize = 24;

#[derive(Debug)]
pub struct RclInit {
    pub context_handle: u64,
    pub version: String,
}

#[derive(Debug)]
pub struct RclNodeInit {
    pub node_handle: u64,
    pub rmw_handle: u64,
    pub node_name: String,
    pub namespace: String,
}

#[derive(Debug)]
pub struct RmwPublisherInit {
    pub rmw_publisher_handle: u64,
    pub gid: [u8; GID_SIZE],
}

#[derive(Debug)]
pub struct RclPublisherInit {
    pub publisher_handle: u64,
    pub node_handle: u64,
    pub rmw_publisher_handle: u64,
    pub topic_name: String,
    pub queue_depth: usize,
}

#[derive(Debug)]
pub struct RclcppPublish {
    pub message: u64,
}

#[derive(Debug)]
pub struct RclcppIntraPublish {
    pub publisher_handle: u64,
    pub message: u64,
}

#[derive(Debug)]
pub struct RclPublish {
    pub publisher_handle: u64,
    pub message: u64,
}

#[derive(Debug)]
pub struct RmwPublish {
    pub rmw_publisher_handle: Option<u64>,
    pub message: u64,
    pub timestamp: Option<i64>,
}

#[derive(Debug)]
pub struct RmwSubscriptionInit {
    pub rmw_subscription_handle: u64,
    pub gid: [u8; GID_SIZE],
}

#[derive(Debug)]
pub struct RclSubscriptionInit {
    pub subscription_handle: u64,
    pub node_handle: u64,
    pub rmw_subscription_handle: u64,
    pub topic_name: String,
    pub queue_depth: usize,
}

#[derive(Debug)]
pub struct RclcppSubscriptionInit {
    pub subscription_handle: u64,
    pub subscription: u64,
}

#[derive(Debug)]
pub struct RclcppSubscriptionCallbackAdded {
    pub subscription: u64,
    pub callback: u64,
}

#[derive(Debug)]
pub struct RmwTake {
    pub rmw_subscription_handle: u64,
    pub message: u64,
    pub source_timestamp: i64,
    pub taken: bool,
}

#[derive(Debug)]
pub struct RclTake {
    pub message: u64,
}

#[derive(Debug)]
pub struct RclcppTake {
    pub message: u64,
}

#[derive(Debug)]
pub struct RclServiceInit {
    pub service_handle: u64,
    pub node_handle: u64,
    pub rmw_service_handle: u64,
    pub service_name: String,
}

#[derive(Debug)]
pub struct RclcppServiceCallbackAdded {
    pub service_handle: u64,
    pub callback: u64,
}

#[derive(Debug)]
pub struct RclClientInit {
    pub client_handle: u64,
    pub node_handle: u64,
    pub rmw_client_handle: u64,
    pub service_name: String,
}

#[derive(Debug)]
pub struct RclTimerInit {
    pub timer_handle: u64,
    pub period: i64,
}

#[derive(Debug)]
pub struct RclcppTimerCallbackAdded {
    pub timer_handle: u64,
    pub callback: u64,
}

#[derive(Debug)]
pub struct RclcppTimerLinkNode {
    pub timer_handle: u64,
    pub node_handle: u64,
}

#[derive(Debug)]
pub struct RclcppCallbackRegister {
    pub callback: u64,
    pub symbol: String,
}

#[derive(Debug)]
pub struct CallbackStart {
    pub callback: u64,
    pub is_intra_process: bool,
}

#[derive(Debug)]
pub struct CallbackEnd {
    pub callback: u64,
}

#[derive(Debug)]
pub struct RclLifecycleStateMachineInit {
    pub node_handle: u64,
    pub state_machine: u64,
}

#[derive(Debug)]
pub struct RclLifecycleTransition {
    pub state_machine: u64,
    pub start_label: String,
    pub goal_label: String,
}

#[derive(Debug)]
pub struct RclCppExecutorGetNextReady;

#[derive(Debug)]
pub struct RclCppExecutorWaitForWork {
    pub timeout: i64,
}

#[derive(Debug)]
pub struct RclCppExecutorExecute {
    pub handle: u64,
}

#[derive(Debug)]
pub struct RclCppIpbToSubscription {
    pub ipb: u64,
    pub subscription: u64,
}

#[derive(Debug)]
pub struct RclCppBufferToIpb {
    pub buffer: u64,
    pub ipb: u64,
}

#[derive(Debug)]
pub struct RclCppConstructRingBuffer {
    pub buffer: u64,
    pub capacity: u64,
}

#[derive(Debug)]
pub struct RclCppRingBufferEnqueue {
    pub buffer: u64,
    pub index: u64,
    pub size: u64,
    pub overwritten: bool,
}

#[derive(Debug)]
pub struct RclCppRingBufferDequeue {
    pub buffer: u64,
    pub index: u64,
    pub size: u64,
}

#[derive(Debug)]
pub struct RclCppRingBufferClear {
    pub buffer: u64,
}

#[derive(Debug, From)]
pub enum Event {
    RclInit(RclInit),
    RclNodeInit(RclNodeInit),
    RmwPublisherInit(RmwPublisherInit),
    RclPublisherInit(RclPublisherInit),
    RclcppPublish(RclcppPublish),
    RclcppIntraPublish(RclcppIntraPublish),
    RclPublish(RclPublish),
    RmwPublish(RmwPublish),
    RmwSubscriptionInit(RmwSubscriptionInit),
    RclSubscriptionInit(RclSubscriptionInit),
    RclcppSubscriptionInit(RclcppSubscriptionInit),
    RclcppSubscriptionCallbackAdded(RclcppSubscriptionCallbackAdded),
    RmwTake(RmwTake),
    RclTake(RclTake),
    RclcppTake(RclcppTake),
    RclServiceInit(RclServiceInit),
    RclcppServiceCallbackAdded(RclcppServiceCallbackAdded),
    RclClientInit(RclClientInit),
    RclTimerInit(RclTimerInit),
    RclcppTimerCallbackAdded(RclcppTimerCallbackAdded),
    RclcppTimerLinkNode(RclcppTimerLinkNode),
    RclcppCallbackRegister(RclcppCallbackRegister),
    CallbackStart(CallbackStart),
    CallbackEnd(CallbackEnd),
    RclLifecycleStateMachineInit(RclLifecycleStateMachineInit),
    RclLifecycleTransition(RclLifecycleTransition),
    RclcppExecutorGetNextReady(RclCppExecutorGetNextReady),
    RclcppExecutorWaitForWork(RclCppExecutorWaitForWork),
    RclcppExecutorExecute(RclCppExecutorExecute),
    RclcppIpbToSubscription(RclCppIpbToSubscription),
    RclcppBufferToIpb(RclCppBufferToIpb),
    RclcppConstructRingBuffer(RclCppConstructRingBuffer),
    RclcppRingBufferEnqueue(RclCppRingBufferEnqueue),
    RclcppRingBufferDequeue(RclCppRingBufferDequeue),
    RclcppRingBufferClear(RclCppRingBufferClear),
}

impl FromBtEvent for RclInit {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let context_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("context_handle")
            .unwrap_or_else(|| panic!("Missing context_handle in event {event:#?}"))
            .into_uint()
            .get_value();
        let version = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("version")
            .unwrap_or_else(|| panic!("Missing version in event {event:#?}"))
            .into_string()
            .get_value()
            .to_string();
        Some(Self {
            context_handle,
            version,
        })
    }
}

impl FromBtEvent for RclNodeInit {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let node_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("node_handle")
            .unwrap_or_else(|| panic!("Missing node_handle in event {event:#?}"))
            .into_uint()
            .get_value();
        let rmw_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("rmw_handle")
            .unwrap_or_else(|| panic!("Missing rmw_handle in event {event:#?}"))
            .into_uint()
            .get_value();
        let node_name = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("node_name")
            .unwrap_or_else(|| panic!("Missing node_name in event {event:#?}"))
            .into_string()
            .get_value()
            .to_string();
        let namespace = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("namespace")
            .unwrap_or_else(|| panic!("Missing namespace in event {event:#?}"))
            .into_string()
            .get_value()
            .to_string();
        Some(Self {
            node_handle,
            rmw_handle,
            node_name,
            namespace,
        })
    }
}

impl FromBtEvent for RmwPublisherInit {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let rmw_publisher_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("rmw_publisher_handle")
            .expect("Missing rmw_publisher_handle")
            .into_uint()
            .get_value();
        let gid = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("gid")
            .expect("Missing gid")
            .into_array()
            .read_static_unsigned_array();
        Some(Self {
            rmw_publisher_handle,
            gid,
        })
    }
}

impl FromBtEvent for RclPublisherInit {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let publisher_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("publisher_handle")
            .expect("Missing publisher_handle")
            .into_uint()
            .get_value();
        let node_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("node_handle")
            .expect("Missing node_handle")
            .into_uint()
            .get_value();
        let rmw_publisher_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("rmw_publisher_handle")
            .expect("Missing rmw_publisher_handle")
            .into_uint()
            .get_value();
        let topic_name = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("topic_name")
            .expect("Missing topic_name")
            .into_string()
            .get_value()
            .to_string();
        let queue_depth = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("queue_depth")
            .expect("Missing queue_depth")
            .into_uint()
            .get_value()
            .try_into()
            .unwrap();
        Some(Self {
            publisher_handle,
            node_handle,
            rmw_publisher_handle,
            topic_name,
            queue_depth,
        })
    }
}

impl FromBtEvent for RclcppPublish {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let message = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("message")
            .expect("Missing message")
            .into_uint()
            .get_value();
        Some(Self { message })
    }
}

impl FromBtEvent for RclcppIntraPublish {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let publisher_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("publisher_handle")
            .expect("Missing publisher_handle")
            .into_uint()
            .get_value();
        let message = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("message")
            .expect("Missing message")
            .into_uint()
            .get_value();
        Some(Self {
            publisher_handle,
            message,
        })
    }
}

impl FromBtEvent for RclPublish {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let publisher_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("publisher_handle")
            .expect("Missing publisher_handle")
            .into_uint()
            .get_value();
        let message = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("message")
            .expect("Missing message")
            .into_uint()
            .get_value();
        Some(Self {
            publisher_handle,
            message,
        })
    }
}

impl FromBtEvent for RmwPublish {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let rmw_publisher_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("rmw_publisher_handle")
            .map(|field| field.into_uint().get_value());
        let message = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("message")
            .expect("Missing message")
            .into_uint()
            .get_value();
        let timestamp = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("timestamp")
            .map(|field| field.into_int().get_value());
        Some(Self {
            rmw_publisher_handle,
            message,
            timestamp,
        })
    }
}

impl FromBtEvent for RmwSubscriptionInit {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let rmw_subscription_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("rmw_subscription_handle")
            .expect("Missing rmw_subscription_handle")
            .into_uint()
            .get_value();
        let gid = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("gid")
            .expect("Missing gid")
            .into_array()
            .read_static_unsigned_array();
        Some(Self {
            rmw_subscription_handle,
            gid,
        })
    }
}

impl FromBtEvent for RclSubscriptionInit {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let subscription_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("subscription_handle")
            .expect("Missing subscription_handle")
            .into_uint()
            .get_value();
        let node_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("node_handle")
            .expect("Missing node_handle")
            .into_uint()
            .get_value();
        let rmw_subscription_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("rmw_subscription_handle")
            .expect("Missing rmw_subscription_handle")
            .into_uint()
            .get_value();
        let topic_name = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("topic_name")
            .expect("Missing topic_name")
            .into_string()
            .get_value()
            .to_string();
        let queue_depth = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("queue_depth")
            .expect("Missing queue_depth")
            .into_uint()
            .get_value()
            .try_into()
            .unwrap();
        Some(Self {
            subscription_handle,
            node_handle,
            rmw_subscription_handle,
            topic_name,
            queue_depth,
        })
    }
}

impl FromBtEvent for RclcppSubscriptionInit {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let subscription_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("subscription_handle")
            .expect("Missing subscription_handle")
            .into_uint()
            .get_value();
        let subscription = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("subscription")
            .expect("Missing subscription")
            .into_uint()
            .get_value();
        Some(Self {
            subscription_handle,
            subscription,
        })
    }
}

impl FromBtEvent for RclcppSubscriptionCallbackAdded {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let subscription = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("subscription")
            .expect("Missing subscription")
            .into_uint()
            .get_value();
        let callback = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("callback")
            .expect("Missing callback")
            .into_uint()
            .get_value();
        Some(Self {
            subscription,
            callback,
        })
    }
}

impl FromBtEvent for RmwTake {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let rmw_subscription_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("rmw_subscription_handle")
            .expect("Missing rmw_subscription_handle")
            .into_uint()
            .get_value();
        let message = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("message")
            .expect("Missing message")
            .into_uint()
            .get_value();
        let source_timestamp = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("source_timestamp")
            .expect("Missing source_timestamp")
            .into_int()
            .get_value();
        let taken = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("taken")
            .expect("Missing taken")
            .into_int()
            .get_value()
            != 0;
        Some(Self {
            rmw_subscription_handle,
            message,
            source_timestamp,
            taken,
        })
    }
}

impl FromBtEvent for RclTake {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let message = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("message")
            .expect("Missing message")
            .into_uint()
            .get_value();
        Some(Self { message })
    }
}

impl FromBtEvent for RclcppTake {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let message = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("message")
            .expect("Missing message")
            .into_uint()
            .get_value();
        Some(Self { message })
    }
}

impl FromBtEvent for RclServiceInit {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let service_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("service_handle")
            .expect("Missing service_handle")
            .into_uint()
            .get_value();
        let node_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("node_handle")
            .expect("Missing node_handle")
            .into_uint()
            .get_value();
        let rmw_service_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("rmw_service_handle")
            .expect("Missing rmw_service_handle")
            .into_uint()
            .get_value();
        let service_name = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("service_name")
            .expect("Missing service_name")
            .into_string()
            .get_value()
            .to_string();
        Some(Self {
            service_handle,
            node_handle,
            rmw_service_handle,
            service_name,
        })
    }
}

impl FromBtEvent for RclcppServiceCallbackAdded {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let service_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("service_handle")
            .expect("Missing service_handle")
            .into_uint()
            .get_value();
        let callback = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("callback")
            .expect("Missing callback")
            .into_uint()
            .get_value();
        Some(Self {
            service_handle,
            callback,
        })
    }
}

impl FromBtEvent for RclClientInit {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let client_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("client_handle")
            .expect("Missing client_handle")
            .into_uint()
            .get_value();
        let node_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("node_handle")
            .expect("Missing node_handle")
            .into_uint()
            .get_value();
        let rmw_client_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("rmw_client_handle")
            .expect("Missing rmw_client_handle")
            .into_uint()
            .get_value();
        let service_name = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("service_name")
            .expect("Missing service_name")
            .into_string()
            .get_value()
            .to_string();
        Some(Self {
            client_handle,
            node_handle,
            rmw_client_handle,
            service_name,
        })
    }
}

impl FromBtEvent for RclTimerInit {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let timer_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("timer_handle")
            .expect("Missing timer_handle")
            .into_uint()
            .get_value();
        let period = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("period")
            .expect("Missing period")
            .into_int()
            .get_value();
        Some(Self {
            timer_handle,
            period,
        })
    }
}

impl FromBtEvent for RclcppTimerCallbackAdded {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let timer_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("timer_handle")
            .expect("Missing timer_handle")
            .into_uint()
            .get_value();
        let callback = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("callback")
            .expect("Missing callback")
            .into_uint()
            .get_value();
        Some(Self {
            timer_handle,
            callback,
        })
    }
}

impl FromBtEvent for RclcppTimerLinkNode {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let timer_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("timer_handle")
            .expect("Missing timer_handle")
            .into_uint()
            .get_value();
        let node_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("node_handle")
            .expect("Missing node_handle")
            .into_uint()
            .get_value();
        Some(Self {
            timer_handle,
            node_handle,
        })
    }
}

impl FromBtEvent for RclcppCallbackRegister {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let callback = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("callback")
            .expect("Missing callback")
            .into_uint()
            .get_value();
        let symbol = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("symbol")
            .expect("Missing symbol")
            .into_string()
            .get_value()
            .to_string();
        Some(Self { callback, symbol })
    }
}

impl FromBtEvent for CallbackStart {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let callback = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("callback")
            .expect("Missing callback")
            .into_uint()
            .get_value();
        let is_intra_process = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("is_intra_process")
            .expect("Missing is_intra_process")
            .into_int()
            .get_value()
            != 0;
        Some(Self {
            callback,
            is_intra_process,
        })
    }
}

impl FromBtEvent for CallbackEnd {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let callback = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("callback")
            .expect("Missing callback")
            .into_uint()
            .get_value();
        Some(Self { callback })
    }
}

impl FromBtEvent for RclLifecycleStateMachineInit {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let node_handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("node_handle")
            .expect("Missing node_handle")
            .into_uint()
            .get_value();
        let state_machine = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("state_machine")
            .expect("Missing state_machine")
            .into_uint()
            .get_value();
        Some(Self {
            node_handle,
            state_machine,
        })
    }
}

impl FromBtEvent for RclLifecycleTransition {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let state_machine = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("state_machine")
            .expect("Missing state_machine")
            .into_uint()
            .get_value();
        let start_label = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("start_label")
            .expect("Missing start_label")
            .into_string()
            .get_value()
            .to_string();
        let goal_label = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("goal_label")
            .expect("Missing goal_label")
            .into_string()
            .get_value()
            .to_string();
        Some(Self {
            state_machine,
            start_label,
            goal_label,
        })
    }
}

impl FromBtEvent for RclCppExecutorGetNextReady {
    fn from_event(_event: &BtEventConst) -> Option<Self> {
        Some(Self {})
    }
}

impl FromBtEvent for RclCppExecutorWaitForWork {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let timeout = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("timeout")
            .expect("Missing timeout")
            .into_int()
            .get_value();
        Some(Self { timeout })
    }
}

impl FromBtEvent for RclCppExecutorExecute {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let handle = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("handle")
            .expect("Missing handle")
            .into_uint()
            .get_value();
        Some(Self { handle })
    }
}

impl FromBtEvent for RclCppIpbToSubscription {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let ipb = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("ipb")
            .expect("Missing ipb")
            .into_uint()
            .get_value();
        let subscription = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("subscription")
            .expect("Missing subscription")
            .into_uint()
            .get_value();
        Some(Self { ipb, subscription })
    }
}

impl FromBtEvent for RclCppBufferToIpb {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let buffer = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("buffer")
            .expect("Missing buffer")
            .into_uint()
            .get_value();
        let ipb = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("ipb")
            .expect("Missing ipb")
            .into_uint()
            .get_value();
        Some(Self { buffer, ipb })
    }
}

impl FromBtEvent for RclCppConstructRingBuffer {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let buffer = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("buffer")
            .expect("Missing buffer")
            .into_uint()
            .get_value();
        let capacity = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("capacity")
            .expect("Missing capacity")
            .into_uint()
            .get_value();
        Some(Self { buffer, capacity })
    }
}

impl FromBtEvent for RclCppRingBufferEnqueue {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let buffer = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("buffer")
            .expect("Missing buffer")
            .into_uint()
            .get_value();
        let index = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("index")
            .expect("Missing index")
            .into_uint()
            .get_value();
        let size = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("size")
            .expect("Missing size")
            .into_uint()
            .get_value();
        let overwritten = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("overwritten")
            .expect("Missing overwritten")
            .into_int()
            .get_value()
            != 0;
        Some(Self {
            buffer,
            index,
            size,
            overwritten,
        })
    }
}

impl FromBtEvent for RclCppRingBufferDequeue {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let buffer = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("buffer")
            .expect("Missing buffer")
            .into_uint()
            .get_value();
        let index = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("index")
            .expect("Missing index")
            .into_uint()
            .get_value();
        let size = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("size")
            .expect("Missing size")
            .into_uint()
            .get_value();
        Some(Self {
            buffer,
            index,
            size,
        })
    }
}

impl FromBtEvent for RclCppRingBufferClear {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let buffer = event
            .get_payload()?
            .into_struct()
            .get_field_by_name("buffer")
            .expect("Missing buffer")
            .into_uint()
            .get_value();
        Some(Self { buffer })
    }
}

impl FromBtEvent for Event {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let event_class = event.get_class();
        let full_event_name = event_class.get_name().unwrap();
        let (provider_name, event_name) = full_event_name.split_once(':').unwrap();

        assert!(provider_name == "ros2");

        match event_name {
            "rcl_init" => RclInit::from_event(event).map(Event::RclInit),
            "rcl_node_init" => RclNodeInit::from_event(event).map(Event::RclNodeInit),
            "rmw_publisher_init" => {
                RmwPublisherInit::from_event(event).map(Event::RmwPublisherInit)
            }
            "rcl_publisher_init" => {
                RclPublisherInit::from_event(event).map(Event::RclPublisherInit)
            }
            "rclcpp_publish" => RclcppPublish::from_event(event).map(Event::RclcppPublish),
            "rclcpp_intra_publish" => {
                RclcppIntraPublish::from_event(event).map(Event::RclcppIntraPublish)
            }
            "rcl_publish" => RclPublish::from_event(event).map(Event::RclPublish),
            "rmw_publish" => RmwPublish::from_event(event).map(Event::RmwPublish),
            "rmw_subscription_init" => {
                RmwSubscriptionInit::from_event(event).map(Event::RmwSubscriptionInit)
            }
            "rcl_subscription_init" => {
                RclSubscriptionInit::from_event(event).map(Event::RclSubscriptionInit)
            }
            "rclcpp_subscription_init" => {
                RclcppSubscriptionInit::from_event(event).map(Event::RclcppSubscriptionInit)
            }
            "rclcpp_subscription_callback_added" => {
                RclcppSubscriptionCallbackAdded::from_event(event)
                    .map(Event::RclcppSubscriptionCallbackAdded)
            }
            "rmw_take" => RmwTake::from_event(event).map(Event::RmwTake),
            "rcl_take" => RclTake::from_event(event).map(Event::RclTake),
            "rclcpp_take" => RclcppTake::from_event(event).map(Event::RclcppTake),
            "rcl_service_init" => RclServiceInit::from_event(event).map(Event::RclServiceInit),
            "rclcpp_service_callback_added" => {
                RclcppServiceCallbackAdded::from_event(event).map(Event::RclcppServiceCallbackAdded)
            }
            "rcl_client_init" => RclClientInit::from_event(event).map(Event::RclClientInit),
            "rcl_timer_init" => RclTimerInit::from_event(event).map(Event::RclTimerInit),
            "rclcpp_timer_callback_added" => {
                RclcppTimerCallbackAdded::from_event(event).map(Event::RclcppTimerCallbackAdded)
            }
            "rclcpp_timer_link_node" => {
                RclcppTimerLinkNode::from_event(event).map(Event::RclcppTimerLinkNode)
            }
            "rclcpp_callback_register" => {
                RclcppCallbackRegister::from_event(event).map(Event::RclcppCallbackRegister)
            }
            "callback_start" => CallbackStart::from_event(event).map(Event::CallbackStart),
            "callback_end" => CallbackEnd::from_event(event).map(Event::CallbackEnd),
            "rcl_lifecycle_state_machine_init" => RclLifecycleStateMachineInit::from_event(event)
                .map(Event::RclLifecycleStateMachineInit),
            "rcl_lifecycle_transition" => {
                RclLifecycleTransition::from_event(event).map(Event::RclLifecycleTransition)
            }
            "rclcpp_executor_get_next_ready" => {
                RclCppExecutorGetNextReady::from_event(event).map(Event::RclcppExecutorGetNextReady)
            }
            "rclcpp_executor_wait_for_work" => {
                RclCppExecutorWaitForWork::from_event(event).map(Event::RclcppExecutorWaitForWork)
            }
            "rclcpp_executor_execute" => {
                RclCppExecutorExecute::from_event(event).map(Event::RclcppExecutorExecute)
            }
            "rcl_ipb_to_subscription" => {
                RclCppIpbToSubscription::from_event(event).map(Event::RclcppIpbToSubscription)
            }
            "rclcpp_buffer_to_ipb" => {
                RclCppBufferToIpb::from_event(event).map(Event::RclcppBufferToIpb)
            }
            "rclcpp_construct_ring_buffer" => {
                RclCppConstructRingBuffer::from_event(event).map(Event::RclcppConstructRingBuffer)
            }
            "rclcpp_ring_buffer_enqueue" => {
                RclCppRingBufferEnqueue::from_event(event).map(Event::RclcppRingBufferEnqueue)
            }
            "rclcpp_ring_buffer_dequeue" => {
                RclCppRingBufferDequeue::from_event(event).map(Event::RclcppRingBufferDequeue)
            }
            "rclcpp_ring_buffer_clear" => {
                RclCppRingBufferClear::from_event(event).map(Event::RclcppRingBufferClear)
            }
            _ => {
                eprintln!("Unknown event: {full_event_name}");
                None
            }
        }
    }
}
