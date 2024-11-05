use derive_more::derive::{Display, From};

use crate::model::{
    Callback, CallbackInstance, Client, Node, PublicationMessage, Publisher, Service, Subscriber,
    SubscriptionMessage, Timer,
};

use super::RefCount;

#[derive(Debug, Clone, From, Display)]
pub enum Event {
    #[display("rcl_init({_0})")]
    RclInit(RclInit),

    #[display("rcl_node_init({_0})")]
    RclNodeInit(RclNodeInit),

    #[display("rmw_publisher_init({_0})")]
    RmwPublisherInit(RmwPublisherInit),

    #[display("rcl_publisher_init({_0})")]
    RclPublisherInit(RclPublisherInit),

    #[display("rclcpp_publish({_0})")]
    RclcppPublish(RclcppPublish),

    #[display("rclcpp_intra_publish({_0})")]
    RclcppIntraPublish(RclcppIntraPublish),

    #[display("rcl_publish({_0})")]
    RclPublish(RclPublish),

    #[display("rmw_publish({_0})")]
    RmwPublish(RmwPublish),

    #[display("rmw_subscription_init({_0})")]
    RmwSubscriptionInit(RmwSubscriptionInit),

    #[display("rcl_subscription_init({_0})")]
    RclSubscriptionInit(RclSubscriptionInit),

    #[display("rclcpp_subscription_init({_0})")]
    RclcppSubscriptionInit(RclcppSubscriptionInit),

    #[display("rclcpp_subscription_callback_added({_0})")]
    RclcppSubscriptionCallbackAdded(RclcppSubscriptionCallbackAdded),

    #[display("rmw_take({_0})")]
    RmwTake(RmwTake),

    #[display("rcl_take({_0})")]
    RclTake(RclTake),

    #[display("rclcpp_take({_0})")]
    RclCppTake(RclCppTake),

    #[display("rcl_service_init({_0})")]
    RclServiceInit(RclServiceInit),

    #[display("rclcpp_service_callback_added({_0})")]
    RclCppServiceCallbackAdded(RclCppServiceCallbackAdded),

    #[display("rcl_client_init({_0})")]
    RclClientInit(RclClientInit),

    #[display("rcl_timer_init({_0})")]
    RclTimerInit(RclTimerInit),

    #[display("rclcpp_timer_callback_added({_0})")]
    RclcppTimerCallbackAdded(RclcppTimerCallbackAdded),

    #[display("rclcpp_timer_link_node({_0})")]
    RclcppTimerLinkNode(RclcppTimerLinkNode),

    #[display("rclcpp_callback_register({_0})")]
    RclcppCallbackRegister(RclcppCallbackRegister),

    #[display("callback_start({_0})")]
    CallbackStart(CallbackStart),

    #[display("callback_end({_0})")]
    CallbackEnd(CallbackEnd),
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
#[display("CallbackInstance({})", callback.lock().unwrap())]
pub struct CallbackStart {
    pub is_intra_process: bool,
    pub callback: RefCount<CallbackInstance>,
}

#[derive(Debug, Clone, Display)]
#[display("CallbackInstance({})", callback.lock().unwrap())]
pub struct CallbackEnd {
    pub callback: RefCount<CallbackInstance>,
}
