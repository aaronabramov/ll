#![allow(clippy::new_without_default)]

pub mod data;
pub mod level;
pub mod task;
pub mod task_tree;
pub mod uniq_id;
pub mod utils;

pub use task::Task;

pub mod reporters;
pub use task_tree::add_reporter;

#[cfg(test)]
mod tests;

pub use data::{Data, DataEntry, DataValue};
pub use reporters::term_status::TermStatus;
pub use reporters::text::StdoutReporter;
pub use reporters::text::StringReporter;
pub use task_tree::TaskInternal;
pub use task_tree::TaskTree;

pub use reporters::term_status::stdio::stdout;
