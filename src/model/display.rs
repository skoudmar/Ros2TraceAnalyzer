use derive_more::derive::From;

use crate::utils::DisplayLargeDuration;

use super::*;

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

#[derive(Debug, From)]
pub(super) struct SubscriberDisplayWithoutNode<'a>(&'a Subscriber);

impl<'a> std::fmt::Display for SubscriberDisplayWithoutNode<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let callback = self
            .0
            .callback
            .as_ref()
            .map(|callback| DisplayArcMutex::new(callback, false));

        write!(f, "(topic={}, handles={{dds={:x}, rmw={:x}, rcl={:x}, rclcpp={:x}}}, queue_depth={}, callback={callback:#})",
            self.0.topic_name.as_ref().map(DisplayDebug), self.0.dds_handle, self.0.rmw_handle, self.0.rcl_handle, self.0.rclcpp_handle, self.0.queue_depth,
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

        write!(f, "(service={}, handles={{rmw={:x}, rcl={:x}, rclcpp={:x}}}, node={node} callback={callback:#})",
            self.name.as_ref().map(DisplayDebug), self.rmw_handle, self.rcl_handle, self.rclcpp_handle,
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

#[derive(Debug, From)]
pub(super) struct TimerDisplayWithoutNode<'a>(&'a Timer);

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
