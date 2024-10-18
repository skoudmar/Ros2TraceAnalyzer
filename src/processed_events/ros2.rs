use std::{fmt::Display, sync::{Arc, Mutex}};

use derive_more::derive::Display;

use crate::{
    events_common::Time,
    impl_from_for_enum,
    model::{
        Callback, Client, Node, PublicationMessage, Publisher, Service, Subscriber,
        SubscriptionMessage, Timer,
    },
};

type RefCount<T> = Arc<Mutex<T>>;

impl_from_for_enum! {
#[derive(Debug, Clone)]
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
    RclCppTake(RclCppTake),
    RclServiceInit(RclServiceInit),
    RclCppServiceCallbackAdded(RclCppServiceCallbackAdded),
    RclClientInit(RclClientInit),
    RclTimerInit(RclTimerInit),
    RclcppTimerCallbackAdded(RclcppTimerCallbackAdded),
    RclcppTimerLinkNode(RclcppTimerLinkNode),
    RclcppCallbackRegister(RclcppCallbackRegister),
    CallbackStart(CallbackStart),
    CallbackEnd(CallbackEnd),
}}

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::RclInit(rcl_init) => write!(f, "rcl_init({rcl_init})"),
            Event::RclNodeInit(rcl_node_init) => write!(f, "rcl_node_init({rcl_node_init})"),
            Event::RmwPublisherInit(rmw_publisher_init) => {
                write!(f, "rmw_publisher_init({rmw_publisher_init})")
            }
            Event::RclPublisherInit(rcl_publisher_init) => {
                write!(f, "rcl_publisher_init({rcl_publisher_init})")
            }
            Event::RclcppPublish(rclcpp_publish) => write!(f, "rclcpp_publish({rclcpp_publish})"),
            Event::RclcppIntraPublish(rclcpp_intra_publish) => {
                write!(f, "rclcpp_intra_publish({rclcpp_intra_publish})")
            }
            Event::RclPublish(rcl_publish) => write!(f, "rcl_publish({rcl_publish})"),
            Event::RmwPublish(rmw_publish) => write!(f, "rmw_publish({rmw_publish})"),
            Event::RmwSubscriptionInit(rmw_subscription_init) => {
                write!(f, "rmw_subscription_init({rmw_subscription_init})")
            }
            Event::RclSubscriptionInit(rcl_subscription_init) => {
                write!(f, "rcl_subscription_init({rcl_subscription_init})")
            }
            Event::RclcppSubscriptionInit(rclcpp_subscription_init) => {
                write!(f, "rclcpp_subscription_init({rclcpp_subscription_init})")
            }
            Event::RclcppSubscriptionCallbackAdded(rclcpp_subscription_callback_added) => {
                write!(f, "rclcpp_subscription_callback_added({rclcpp_subscription_callback_added})")
            }
            Event::RmwTake(rmw_take) => write!(f, "rmw_take({rmw_take})"),
            Event::RclTake(rcl_take) => write!(f, "rcl_take({rcl_take})"),
            Event::RclCppTake(rcl_cpp_take) => write!(f, "rcl_cpp_take({rcl_cpp_take})"),
            Event::RclServiceInit(rcl_service_init) => write!(f, "rcl_service_init({rcl_service_init})"),
            Event::RclCppServiceCallbackAdded(rcl_cpp_service_callback_added) => {
                write!(f, "rcl_cpp_service_callback_added({rcl_cpp_service_callback_added})")
            }
            Event::RclClientInit(rcl_client_init) => write!(f, "rcl_client_init({rcl_client_init})"),
            Event::RclTimerInit(rcl_timer_init) => write!(f, "rcl_timer_init({rcl_timer_init})"),
            Event::RclcppTimerCallbackAdded(rclcpp_timer_callback_added) => {
                write!(f, "rclcpp_timer_callback_added({rclcpp_timer_callback_added})")
            }
            Event::RclcppTimerLinkNode(rclcpp_timer_link_node) => {
                write!(f, "rclcpp_timer_link_node({rclcpp_timer_link_node})")
            }
            Event::RclcppCallbackRegister(rclcpp_callback_register) => {
                write!(f, "rclcpp_callback_register({rclcpp_callback_register})")
            }
            Event::CallbackStart(callback_start) => write!(f, "callback_start({callback_start})"),
            Event::CallbackEnd(callback_end) => write!(f, "callback_end({callback_end})"),
        }
    }
}

#[derive(Debug, Clone, Display)]
#[display("context: 0x{context_handle:x}, version: {version}")]
pub struct RclInit {
    pub context_handle: u64,
    pub version: String,
}

#[derive(Debug, Clone, Display)]
#[display("Node({})", node.lock().unwrap())]
pub struct RclNodeInit {
    pub node: RefCount<Node>,
}

// Publisher

#[derive(Debug, Clone, Display)]
#[display("Publisher({})", publisher.lock().unwrap())]
pub struct RmwPublisherInit {
    pub publisher: RefCount<Publisher>,
}

#[derive(Debug, Clone, Display)]
#[display("Publisher({})", publisher.lock().unwrap())]
pub struct RclPublisherInit {
    pub publisher: RefCount<Publisher>,
}

// publish

#[derive(Debug, Clone, Display)]
#[display("Message({})", message.lock().unwrap())]
pub struct RclcppPublish {
    /// message pointer
    pub message: RefCount<PublicationMessage>,
}

#[derive(Debug, Clone, Display)]
#[display("Message({})", message.lock().unwrap())]
pub struct RclcppIntraPublish {
    pub message: RefCount<PublicationMessage>,
}

#[derive(Debug, Clone, Display)]
#[display("Message({})", message.lock().unwrap())]
pub struct RclPublish {
    pub message: RefCount<PublicationMessage>,
}

#[derive(Debug, Clone, Display)]
#[display("Message({})", message.lock().unwrap())]
pub struct RmwPublish {
    pub message: RefCount<PublicationMessage>,
}

// Subscription
#[derive(Debug, Clone, Display)]
#[display("Subscriber({})", subscription.lock().unwrap())]
pub struct RmwSubscriptionInit {
    pub subscription: RefCount<Subscriber>,
}

#[derive(Debug, Clone, Display)]
#[display("Subscriber({})", subscription.lock().unwrap())]
pub struct RclSubscriptionInit {
    pub subscription: RefCount<Subscriber>,
}

#[derive(Debug, Clone, Display)]
#[display("Subscriber({})", subscription.lock().unwrap())]
pub struct RclcppSubscriptionInit {
    pub subscription: RefCount<Subscriber>,
}

#[derive(Debug, Clone, Display)]
#[display("Callback({})", callback.lock().unwrap())]
pub struct RclcppSubscriptionCallbackAdded {
    pub callback: RefCount<Callback>,
}

// Take

#[derive(Debug, Clone, Display)]
#[display("taken={taken}, Message({})", message.lock().unwrap())]
pub struct RmwTake {
    pub message: RefCount<SubscriptionMessage>,
    pub taken: bool,
}

#[derive(Debug, Clone, Display)]
#[display("Message({})", message.lock().unwrap())]
pub struct RclTake {
    pub message: RefCount<SubscriptionMessage>,
}

#[derive(Debug, Clone, Display)]
#[display("Message({})", message.lock().unwrap())]
pub struct RclCppTake {
    pub message: RefCount<SubscriptionMessage>,
}

// Service

#[derive(Debug, Clone, Display)]
#[display("Service({})", service.lock().unwrap())]
pub struct RclServiceInit {
    pub service: RefCount<Service>,
}

#[derive(Debug, Clone, Display)]
#[display("Callback({})", callback.lock().unwrap())]
pub struct RclCppServiceCallbackAdded {
    pub callback: RefCount<Callback>,
}

// Client

#[derive(Debug, Clone, Display)]
#[display("Client({})", client.lock().unwrap())]
pub struct RclClientInit {
    pub client: RefCount<Client>,
}

// Timer

#[derive(Debug, Clone, Display)]
#[display("Timer({})", timer.lock().unwrap())]
pub struct RclTimerInit {
    pub timer: RefCount<Timer>,
}

#[derive(Debug, Clone, Display)]
#[display("Callback({})", callback.lock().unwrap())]
pub struct RclcppTimerCallbackAdded {
    pub callback: RefCount<Callback>,
}

#[derive(Debug, Clone, Display)]
#[display("Timer({})", timer.lock().unwrap())]
pub struct RclcppTimerLinkNode {
    pub timer: RefCount<Timer>,
}

// Callback

#[derive(Debug, Clone, Display)]
#[display("Callback({})", callback.lock().unwrap())]
pub struct RclcppCallbackRegister {
    pub callback: RefCount<Callback>,
}

#[derive(Debug, Clone, Display)]
#[display("Callback({})", callback.lock().unwrap())]
pub struct CallbackStart {
    pub is_intra_process: bool,
    pub callback: RefCount<Callback>,
}

#[derive(Debug, Clone, Display)]
#[display("Callback({})", callback.lock().unwrap())]
pub struct CallbackEnd {
    pub callback: RefCount<Callback>,
    pub start_time: Time,
    pub end_time: Time,
}
