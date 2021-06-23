pub mod term_status;
mod text;

use crate::task_internal::TaskInternal;
use std::sync::Arc;
pub use term_status::TermStatus;

pub use text::StdoutReporter;
pub use text::StringReporter;

pub const DONTPRINT_TAG: &str = "dontprint";

#[async_trait::async_trait]
pub trait Reporter: Send + Sync {
    async fn task_start(&self, _task: Arc<TaskInternal>) {}
    async fn task_end(&self, _task: Arc<TaskInternal>) {}
}
