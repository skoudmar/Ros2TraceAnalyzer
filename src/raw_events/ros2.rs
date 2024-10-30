use bt2_derive::TryFromBtFieldConst;
use bt2_sys::event::BtEventConst;
use derive_more::derive::From;

use super::FromBtEvent;

pub const GID_SIZE: usize = 24;

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclInit {
    pub context_handle: u64,
    pub version: String,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclNodeInit {
    pub node_handle: u64,
    pub rmw_handle: u64,
    pub node_name: String,
    pub namespace: String,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RmwPublisherInit {
    pub rmw_publisher_handle: u64,
    pub gid: [u8; GID_SIZE],
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclPublisherInit {
    pub publisher_handle: u64,
    pub node_handle: u64,
    pub rmw_publisher_handle: u64,
    pub topic_name: String,
    pub queue_depth: usize,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclcppPublish {
    pub message: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclcppIntraPublish {
    pub publisher_handle: u64,
    pub message: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
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

#[derive(Debug, TryFromBtFieldConst)]
pub struct RmwSubscriptionInit {
    pub rmw_subscription_handle: u64,
    pub gid: [u8; GID_SIZE],
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclSubscriptionInit {
    pub subscription_handle: u64,
    pub node_handle: u64,
    pub rmw_subscription_handle: u64,
    pub topic_name: String,
    pub queue_depth: usize,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclcppSubscriptionInit {
    pub subscription_handle: u64,
    pub subscription: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclcppSubscriptionCallbackAdded {
    pub subscription: u64,
    pub callback: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RmwTake {
    pub rmw_subscription_handle: u64,
    pub message: u64,
    pub source_timestamp: i64,
    #[bt2(try_from = i64, is_not_zero)]
    pub taken: bool,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclTake {
    pub message: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclcppTake {
    pub message: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclServiceInit {
    pub service_handle: u64,
    pub node_handle: u64,
    pub rmw_service_handle: u64,
    pub service_name: String,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclcppServiceCallbackAdded {
    pub service_handle: u64,
    pub callback: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclClientInit {
    pub client_handle: u64,
    pub node_handle: u64,
    pub rmw_client_handle: u64,
    pub service_name: String,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclTimerInit {
    pub timer_handle: u64,
    pub period: i64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclcppTimerCallbackAdded {
    pub timer_handle: u64,
    pub callback: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclcppTimerLinkNode {
    pub timer_handle: u64,
    pub node_handle: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclcppCallbackRegister {
    pub callback: u64,
    pub symbol: String,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct CallbackStart {
    pub callback: u64,
    #[bt2(try_from = i64, is_not_zero)]
    pub is_intra_process: bool,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct CallbackEnd {
    pub callback: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclLifecycleStateMachineInit {
    pub node_handle: u64,
    pub state_machine: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclLifecycleTransition {
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
    pub handle: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclCppIpbToSubscription {
    pub ipb: u64,
    pub subscription: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclCppBufferToIpb {
    pub buffer: u64,
    pub ipb: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclCppConstructRingBuffer {
    pub buffer: u64,
    pub capacity: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclCppRingBufferEnqueue {
    pub buffer: u64,
    pub index: u64,
    pub size: u64,
    #[bt2(try_from = i64, is_not_zero)]
    pub overwritten: bool,
}

#[derive(Debug, TryFromBtFieldConst)]
pub struct RclCppRingBufferDequeue {
    pub buffer: u64,
    pub index: u64,
    pub size: u64,
}

#[derive(Debug, TryFromBtFieldConst)]
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
