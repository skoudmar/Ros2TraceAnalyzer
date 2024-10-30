// Reexport the time struct from model.rs
pub use crate::model::Time;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    cpu_id: u32,
    vpid: u32,
    vtid: u32,
    procname: String,
    hostname: String,
}

impl Context {
    pub const fn new(
        cpu_id: u32,
        vpid: u32,
        vtid: u32,
        procname: String,
        hostname: String,
    ) -> Self {
        Self {
            cpu_id,
            vpid,
            vtid,
            procname,
            hostname,
        }
    }

    pub const fn cpu_id(&self) -> u32 {
        self.cpu_id
    }

    pub const fn vpid(&self) -> u32 {
        self.vpid
    }

    pub const fn vtid(&self) -> u32 {
        self.vtid
    }

    pub fn procname(&self) -> &str {
        &self.procname
    }

    pub fn hostname(&self) -> &str {
        &self.hostname
    }
}
