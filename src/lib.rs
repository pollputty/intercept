pub mod config;
mod modules;
mod recorder;
mod syscall;
mod tracer;

pub use config::Config;
pub use recorder::{Record, Recorder};
pub use syscall::SysNum;
pub use tracer::{SpawnOptions, Tracer};
