// Output plugin implementations

pub mod syslog;
pub mod vector;

pub use syslog::SyslogOutput;
pub use vector::VectorOutput;