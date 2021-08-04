pub mod term_status;
pub mod text;
pub mod utils;

use crate::task_tree::TaskInternal;
use std::sync::Arc;
pub use term_status::TermStatus;

pub use text::StdioReporter;
pub use text::StringReporter;

pub const DONTPRINT_TAG: &str = "dontprint";

/// Logging levers, by default all tasks log as L1, but can be changed to
/// l0, l2, l3 by using #l0 #l2 #l3 tags in the task name.
/// Reporters can be set to ignore anything up from a certain level.
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub enum Level {
    L0,
    L1,
    L2,
    L3,
}

pub trait Reporter: Send + Sync {
    fn task_start(&self, _task: Arc<TaskInternal>) {}
    fn task_end(&self, _task: Arc<TaskInternal>) {}
}
