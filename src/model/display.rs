use std::string::ToString;
use std::sync::{Mutex, Weak};

use crate::utils::{
    DisplayArcMutex, DisplayDebug, DisplayDuration, DisplayLargeDuration, DisplayWeakMutex, Known,
    WeakKnown,
};

use super::{
    Callback, CallbackCaller, CallbackInstance, CallbackType, Client, DdsGid, Gid, Name, Node,
    PartiallyKnown, PublicationMessage, Publisher, RmwGid, Service, SpinInstance, Subscriber,
    SubscriptionMessage, Time, Timer,
};

impl std::fmt::Debug for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Time").field(&self.as_datetime()).finish()
    }
}

impl std::fmt::Display for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:0<29}",
            self.as_datetime()
                .naive_local()
                .format("%Y-%m-%d %H:%M:%S%.9f")
        )
    }
}

impl std::fmt::Debug for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Name")
            .field("namespace", &self.get_namespace())
            .field("name", &self.get_name())
            .finish()
    }
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.get_full_name().fmt(f)
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

pub fn get_node_name_from_weak(node_weak: &Weak<Mutex<Node>>) -> WeakKnown<String> {
    let Some(node) = node_weak.upgrade() else {
        return WeakKnown::Droped;
    };
    let node = node.lock().unwrap();
    node.get_full_name().map(ToString::to_string).into()
}

impl std::fmt::Display for Publisher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let node = self
            .node
            .as_ref()
            .map(|node| DisplayWeakMutex::new(node, f.alternate()));

        write!(
            f,
            "(topic={}, handles={{rmw={:x}, rcl={:x}, rclcpp={:x}}}, queue_depth={}, node={node})",
            self.topic_name.as_ref().map(DisplayDebug),
            self.rmw_handle,
            self.rcl_handle,
            self.rclcpp_handle,
            self.queue_depth
        )
    }
}

pub(super) struct DisplayPublisherWithoutNode<'a>(pub &'a Publisher);

impl<'a> std::fmt::Display for DisplayPublisherWithoutNode<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "(topic={}, handles={{rmw={:x}, rcl={:x}, rclcpp={:x}}}, queue_depth={})",
            self.0.topic_name.as_ref().map(DisplayDebug),
            self.0.rmw_handle,
            self.0.rcl_handle,
            self.0.rclcpp_handle,
            self.0.queue_depth
        )
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

        write!(f, "(topic={}, handles={{rmw={:x}, rcl={:x}, rclcpp={:x}}}, queue_depth={}, node={node} callback={callback:#})",
            self.topic_name.as_ref().map(DisplayDebug), self.rmw_handle, self.rcl_handle, self.rclcpp_handle, self.queue_depth,
        )
    }
}

pub fn get_subscriber_topic_from_weak(
    subscriber_weak: &Weak<Mutex<Subscriber>>,
) -> WeakKnown<String> {
    let Some(node) = subscriber_weak.upgrade() else {
        return WeakKnown::Droped;
    };
    let subscriber = node.lock().unwrap();
    subscriber.get_topic().map(ToString::to_string).into()
}

pub(super) struct DisplaySubscriberWithoutNode<'a>(pub &'a Subscriber);

impl<'a> std::fmt::Display for DisplaySubscriberWithoutNode<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let callback = self
            .0
            .callback
            .as_ref()
            .map(|callback| DisplayArcMutex::new(callback, false));

        write!(f, "(topic={}, handles={{rmw={:x}, rcl={:x}, rclcpp={:x}}}, queue_depth={}, callback={callback:#})",
            self.0.topic_name.as_ref().map(DisplayDebug), self.0.rmw_handle, self.0.rcl_handle, self.0.rclcpp_handle, self.0.queue_depth,
        )
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

        write!(f, "(service={}, handles={{rmw={:x}, rcl={:x}, rclcpp={:x}}}, node={node}, callback={callback:#})",
            self.name.as_ref().map(DisplayDebug), self.rmw_handle, self.rcl_handle, self.rclcpp_handle,
        )
    }
}

pub fn get_service_name_from_weak(service_weak: &Weak<Mutex<Service>>) -> WeakKnown<String> {
    let Some(service) = service_weak.upgrade() else {
        return WeakKnown::Droped;
    };
    let service = service.lock().unwrap();
    service.get_name().map(ToString::to_string).into()
}

pub(super) struct DisplayServiceWithoutNode<'a>(pub &'a Service);

impl std::fmt::Display for DisplayServiceWithoutNode<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let callback = self
            .0
            .callback
            .as_ref()
            .map(|callback| DisplayArcMutex::new(callback, false));

        write!(
            f,
            "(service={}, handles={{rmw={:x}, rcl={:x}, rclcpp={:x}}}, callback={callback:#})",
            self.0.name.as_ref().map(DisplayDebug),
            self.0.rmw_handle,
            self.0.rcl_handle,
            self.0.rclcpp_handle,
        )
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

pub(super) struct DisplayClientWithoutNode<'a>(pub &'a Client);

impl std::fmt::Display for DisplayClientWithoutNode<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "(service={}, handles={{rmw={:x}, rcl={:x}}})",
            self.0.service_name.as_ref().map(DisplayDebug),
            self.0.rmw_handle,
            self.0.rcl_handle
        )
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
        let period = self.period.map(DisplayDuration);

        write!(
            f,
            "(period={period}, rcl_handle={}, node={node} callback={callback:#})",
            self.rcl_handle,
        )
    }
}

pub fn get_timer_period_from_weak(timer_weak: &Weak<Mutex<Timer>>) -> WeakKnown<i64> {
    let Some(timer) = timer_weak.upgrade() else {
        return WeakKnown::Droped;
    };
    let timer = timer.lock().unwrap();
    timer.get_period().into()
}

pub(super) struct TimerDisplayWithoutNode<'a>(pub &'a Timer);

impl<'a> std::fmt::Display for TimerDisplayWithoutNode<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let callback = self
            .0
            .callback
            .as_ref()
            .map(|callback| DisplayArcMutex::new(callback, false));

        let period = self.0.period.map(DisplayDuration);

        write!(
            f,
            "(period={period}, rcl_handle={}, callback={callback:#})",
            self.0.rcl_handle,
        )
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

pub struct DisplayCallbackSummary<'a>(pub &'a Callback);

impl std::fmt::Display for DisplayCallbackSummary<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let this = self.0;
        let Some(caller) = this.get_caller() else {
            return write!(f, "(UNKNOWN)");
        };
        let Some(node) = this.get_node() else {
            let typ = CallbackType::from(caller);
            return write!(f, "({typ:?})");
        };

        let node_name = node.upgrade().map_or_else(
            || Known::Known(DisplayDebug("DROPED".to_owned())),
            |node_arc| {
                let node = node_arc.lock().unwrap();
                node.get_full_name()
                    .map(std::borrow::ToOwned::to_owned)
                    .map(DisplayDebug)
            },
        );

        match caller {
            CallbackCaller::Subscription(sub) => {
                let sub = sub.upgrade().unwrap();
                let sub = sub.lock().unwrap();
                let topic = sub.get_topic().map(DisplayDebug);
                write!(f, "(node={node_name}, Subscriber({topic}))")
            }
            CallbackCaller::Service(service) => {
                let service = service.upgrade().unwrap();
                let service = service.lock().unwrap();
                let name = service.get_name().map(DisplayDebug);

                write!(f, "(node={node_name}, Service({name})")
            }
            CallbackCaller::Timer(timer) => {
                let timer = timer.upgrade().unwrap();
                let timer = timer.lock().unwrap();
                let period = timer.get_period().map(DisplayDuration);

                write!(f, "(node={node_name}, Timer({period})")
            }
        }
    }
}

impl std::fmt::Display for CallbackType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timer => write!(f, "Timer"),
            Self::Subscription => write!(f, "Subscription"),
            Self::Service => write!(f, "Service"),
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
            Self::DdsGid(gid) => write!(f, "{gid}"),
            Self::RmwGid(gid) => write!(f, "{gid}"),
        }
    }
}

impl std::fmt::Debug for Gid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DdsGid(gid) => std::fmt::Debug::fmt(gid, f),
            Self::RmwGid(gid) => std::fmt::Debug::fmt(gid, f),
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

impl std::fmt::Display for SpinInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let node = DisplayWeakMutex::new(&self.node, false);
        let timeout = DisplayLargeDuration(self.timeout.as_nanos());
        write!(
            f,
            "(node={node}, start_time={}, end_time={}, timeout={timeout})",
            self.start_time, self.end_time
        )
    }
}
