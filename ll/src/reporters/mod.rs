pub mod term_status;
pub mod text;

use crate::task_tree::TaskInternal;
use std::sync::Arc;
pub use term_status::TermStatus;

pub use text::StdioReporter;
pub use text::StringReporter;

pub const DONTPRINT_TAG: &str = "dontprint";

pub trait Reporter: Send + Sync {
    fn task_start(&self, _task: Arc<TaskInternal>) {}
    fn task_end(&self, _task: Arc<TaskInternal>) {}
}
