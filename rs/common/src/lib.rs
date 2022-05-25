#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeType {
    Machine,
    Server,
    Environment,
    MachineOrEnvironment,
    Any,
}

pub mod message;
#[macro_use]
pub mod util;
#[cfg(feature = "tokio")]
pub mod io;
