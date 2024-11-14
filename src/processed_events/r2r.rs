use derive_more::derive::{Display, From};

use crate::model::{Node, SpinInstance, Subscriber};

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
#[display("Spin({})", spin.lock().unwrap())]
pub struct SpinStart {
    pub node: RefCount<Node>,
    pub spin: RefCount<SpinInstance>,
}

#[derive(Debug, Clone)]
pub struct SpinEnd {
    pub node: RefCount<Node>,
    pub spin: Option<RefCount<SpinInstance>>,
}

#[derive(Debug, Clone)]
pub struct SpinWake {
    pub node: RefCount<Node>,
    pub spin: Option<RefCount<SpinInstance>>,
}

#[derive(Debug, Clone)]
pub struct SpinTimeout {
    pub node: RefCount<Node>,
    pub spin: Option<RefCount<SpinInstance>>,
}

macro_rules! impl_display_for_spin_events {
    ($($event:ident),*) => {
        $(
            impl std::fmt::Display for $event {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    match &self.spin {
                        Some(spin) => write!(f, "Spin({})", spin.lock().unwrap()),
                        None => write!(f, "Node({})", self.node.lock().unwrap()),
                    }
                }
            }
        )*
    };
}

impl_display_for_spin_events!(SpinEnd, SpinWake, SpinTimeout);

#[derive(Debug, Clone, Display)]
#[display("subscriber: {}, new_time: {time_s}.{time_ns:09}", subscriber.lock().unwrap())]
pub struct UpdateTime {
    pub subscriber: RefCount<Subscriber>,
    pub time_s: i32,
    pub time_ns: u32,
}
