use bt2_derive::TryFromBtFieldConst;
use bt2_sys::event::BtEventConst;
use derive_more::derive::{Debug, From};

use crate::utils::DebugOptionHex;

use super::FromBtEvent;

pub const GID_SIZE: usize = 24;

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclInit {
    #[debug("{context_handle:#x}")]
    pub context_handle: u64,
    pub version: String,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclNodeInit {
    #[debug("{node_handle:#x}")]
    pub node_handle: u64,
    #[debug("{rmw_handle:#x}")]
    pub rmw_handle: u64,
    pub node_name: String,
    pub namespace: String,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RmwPublisherInit {
    #[debug("{rmw_publisher_handle:#x}")]
    pub rmw_publisher_handle: u64,
    pub gid: [u8; GID_SIZE],
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclPublisherInit {
    #[debug("{publisher_handle:#x}")]
    pub publisher_handle: u64,
    #[debug("{node_handle:#x}")]
    pub node_handle: u64,
    #[debug("{rmw_publisher_handle:#x}")]
    pub rmw_publisher_handle: u64,
    pub topic_name: String,
    pub queue_depth: usize,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclcppPublish {
    #[debug("{message:#x}")]
    pub message: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclcppIntraPublish {
    #[debug("{publisher_handle:#x}")]
    pub publisher_handle: u64,
    #[debug("{message:#x}")]
    pub message: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclPublish {
    #[debug("{publisher_handle:#x}")]
    pub publisher_handle: u64,
    #[debug("{message:#x}")]
    pub message: u64,
}

#[derive(Debug)]
pub struct RmwPublish {
    #[debug("{:?}", DebugOptionHex(rmw_publisher_handle))]
    pub rmw_publisher_handle: Option<u64>,
    #[debug("{message:#x}")]
    pub message: u64,
    pub timestamp: Option<i64>,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RmwSubscriptionInit {
    #[debug("{rmw_subscription_handle:#x}")]
    pub rmw_subscription_handle: u64,
    pub gid: [u8; GID_SIZE],
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclSubscriptionInit {
    #[debug("{subscription_handle:#x}")]
    pub subscription_handle: u64,
    #[debug("{node_handle:#x}")]
    pub node_handle: u64,
    #[debug("{rmw_subscription_handle:#x}")]
    pub rmw_subscription_handle: u64,
    pub topic_name: String,
    pub queue_depth: usize,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclcppSubscriptionInit {
    #[debug("{subscription_handle:#x}")]
    pub subscription_handle: u64,
    #[debug("{subscription:#x}")]
    pub subscription: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclcppSubscriptionCallbackAdded {
    #[debug("{subscription:#x}")]
    pub subscription: u64,
    #[debug("{callback:#x}")]
    pub callback: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RmwTake {
    #[debug("{rmw_subscription_handle:#x}")]
    pub rmw_subscription_handle: u64,
    #[debug("{message:#x}")]
    pub message: u64,
    pub source_timestamp: i64,
    #[bt2(try_from = i64, is_not_zero)]
    pub taken: bool,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclTake {
    #[debug("{message:#x}")]
    pub message: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclcppTake {
    #[debug("{message:#x}")]
    pub message: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclServiceInit {
    #[debug("{service_handle:#x}")]
    pub service_handle: u64,
    #[debug("{node_handle:#x}")]
    pub node_handle: u64,
    #[debug("{rmw_service_handle:#x}")]
    pub rmw_service_handle: u64,
    pub service_name: String,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclcppServiceCallbackAdded {
    #[debug("{service_handle:#x}")]
    pub service_handle: u64,
    #[debug("{callback:#x}")]
    pub callback: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclClientInit {
    #[debug("{client_handle:#x}")]
    pub client_handle: u64,
    #[debug("{node_handle:#x}")]
    pub node_handle: u64,
    #[debug("{rmw_client_handle:#x}")]
    pub rmw_client_handle: u64,
    pub service_name: String,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclTimerInit {
    #[debug("{timer_handle:#x}")]
    pub timer_handle: u64,
    pub period: i64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclcppTimerCallbackAdded {
    #[debug("{timer_handle:#x}")]
    pub timer_handle: u64,
    pub callback: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclcppTimerLinkNode {
    #[debug("{timer_handle:#x}")]
    pub timer_handle: u64,
    #[debug("{node_handle:#x}")]
    pub node_handle: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclcppCallbackRegister {
    #[debug("{callback:#x}")]
    pub callback: u64,
    pub symbol: String,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct CallbackStart {
    #[debug("{callback:#x}")]
    pub callback: u64,
    #[bt2(try_from = i64, is_not_zero)]
    pub is_intra_process: bool,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct CallbackEnd {
    #[debug("{callback:#x}")]
    pub callback: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclLifecycleStateMachineInit {
    #[debug("{node_handle:#x}")]
    pub node_handle: u64,
    #[debug("{state_machine:#x}")]
    pub state_machine: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclLifecycleTransition {
    #[debug("{state_machine:#x}")]
    pub state_machine: u64,
    pub start_label: String,
    pub goal_label: String,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclCppExecutorGetNextReady;

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclCppExecutorWaitForWork {
    pub timeout: i64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclCppExecutorExecute {
    #[debug("{handle:#x}")]
    pub handle: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclCppIpbToSubscription {
    #[debug("{ipb:#x}")]
    pub ipb: u64,
    #[debug("{subscription:#x}")]
    pub subscription: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclCppBufferToIpb {
    #[debug("{buffer:#x}")]
    pub buffer: u64,
    #[debug("{ipb:#x}")]
    pub ipb: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclCppConstructRingBuffer {
    #[debug("{buffer:#x}")]
    pub buffer: u64,
    pub capacity: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclCppRingBufferEnqueue {
    #[debug("{buffer:#x}")]
    pub buffer: u64,
    pub index: u64,
    pub size: u64,
    #[bt2(try_from = i64, is_not_zero)]
    pub overwritten: bool,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclCppRingBufferDequeue {
    #[debug("{buffer:#x}")]
    pub buffer: u64,
    pub index: u64,
    pub size: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclCppRingBufferClear {
    #[debug("{buffer:#x}")]
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

impl FromBtEvent for Event {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let event_class = event.get_class();
        let full_event_name = event_class.get_name().unwrap();
        let (provider_name, event_name) = full_event_name.split_once(':').unwrap();

        assert!(provider_name == "ros2");

        Some(match event_name {
            "rcl_init" => RclInit::from_event(event)?.into(),
            "rcl_node_init" => RclNodeInit::from_event(event)?.into(),
            "rmw_publisher_init" => RmwPublisherInit::from_event(event)?.into(),
            "rcl_publisher_init" => RclPublisherInit::from_event(event)?.into(),
            "rclcpp_publish" => RclcppPublish::from_event(event)?.into(),
            "rclcpp_intra_publish" => RclcppIntraPublish::from_event(event)?.into(),
            "rcl_publish" => RclPublish::from_event(event)?.into(),
            "rmw_publish" => RmwPublish::from_event(event)?.into(),
            "rmw_subscription_init" => RmwSubscriptionInit::from_event(event)?.into(),
            "rcl_subscription_init" => RclSubscriptionInit::from_event(event)?.into(),
            "rclcpp_subscription_init" => RclcppSubscriptionInit::from_event(event)?.into(),
            "rclcpp_subscription_callback_added" => {
                RclcppSubscriptionCallbackAdded::from_event(event)?.into()
            }
            "rmw_take" => RmwTake::from_event(event)?.into(),
            "rcl_take" => RclTake::from_event(event)?.into(),
            "rclcpp_take" => RclcppTake::from_event(event)?.into(),
            "rcl_service_init" => RclServiceInit::from_event(event)?.into(),
            "rclcpp_service_callback_added" => {
                RclcppServiceCallbackAdded::from_event(event)?.into()
            }
            "rcl_client_init" => RclClientInit::from_event(event)?.into(),
            "rcl_timer_init" => RclTimerInit::from_event(event)?.into(),
            "rclcpp_timer_callback_added" => RclcppTimerCallbackAdded::from_event(event)?.into(),
            "rclcpp_timer_link_node" => RclcppTimerLinkNode::from_event(event)?.into(),
            "rclcpp_callback_register" => RclcppCallbackRegister::from_event(event)?.into(),
            "callback_start" => CallbackStart::from_event(event)?.into(),
            "callback_end" => CallbackEnd::from_event(event)?.into(),
            "rcl_lifecycle_state_machine_init" => {
                RclLifecycleStateMachineInit::from_event(event)?.into()
            }
            "rcl_lifecycle_transition" => RclLifecycleTransition::from_event(event)?.into(),
            "rclcpp_executor_get_next_ready" => {
                RclCppExecutorGetNextReady::from_event(event)?.into()
            }
            "rclcpp_executor_wait_for_work" => RclCppExecutorWaitForWork::from_event(event)?.into(),
            "rclcpp_executor_execute" => RclCppExecutorExecute::from_event(event)?.into(),
            "rcl_ipb_to_subscription" => RclCppIpbToSubscription::from_event(event)?.into(),
            "rclcpp_buffer_to_ipb" => RclCppBufferToIpb::from_event(event)?.into(),
            "rclcpp_construct_ring_buffer" => RclCppConstructRingBuffer::from_event(event)?.into(),
            "rclcpp_ring_buffer_enqueue" => RclCppRingBufferEnqueue::from_event(event)?.into(),
            "rclcpp_ring_buffer_dequeue" => RclCppRingBufferDequeue::from_event(event)?.into(),
            "rclcpp_ring_buffer_clear" => RclCppRingBufferClear::from_event(event)?.into(),
            _ => return None,
        })
    }
}
