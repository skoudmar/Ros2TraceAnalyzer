use derive_more::derive::{Display, From};

use crate::model::Node;
use crate::utils::DisplayLargeDuration;

use super::RefCount;

#[derive(Debug, Clone, From, Display)]
pub enum Event {
    #[display("spin_start({_0})")]
    SpinStart(SpinStart),
    #[display("spin_end({_0})")]
    SpinEnd(SpinEnd),
    #[display("spin_wake({_0})")]
    SpinWake(SpinWake),
    #[display("spin_timeout({_0})")]
    SpinTimeout(SpinTimeout),
    #[display("update_time({_0})")]
    UpdateTime(UpdateTime),
}

#[derive(Debug, Clone, Display)]
#[display("timeout({}), Node({})", DisplayLargeDuration(timeout.as_nanos()), node.lock().unwrap())]
pub struct SpinStart {
    pub node: RefCount<Node>,
    pub timeout: std::time::Duration,
}

#[derive(Debug, Clone, Display)]
#[display("Node({})", node.lock().unwrap())]
pub struct SpinEnd {
    pub node: RefCount<Node>,
}

#[derive(Debug, Clone, Display)]
#[display("Node({})", node.lock().unwrap())]
pub struct SpinWake {
    pub node: RefCount<Node>,
}

#[derive(Debug, Clone, Display)]
#[display("Node({})", node.lock().unwrap())]
pub struct SpinTimeout {
    pub node: RefCount<Node>,
}

#[derive(Debug, Clone, Display)]
#[display("subscriber: {subscriber}, time: {time_s}.{time_ns:09}")]
pub struct UpdateTime {
    pub subscriber: u64,
    pub time_s: i32,
    pub time_ns: u32,
}
