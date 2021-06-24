#![allow(clippy::new_without_default)]

mod data;
mod level;
mod task;
mod task_tree;
mod uniq_id;
mod utils;

pub use task::Task;

pub mod reporters;
pub use task_tree::add_reporter;

#[cfg(test)]
mod tests;

pub use reporters::term_status::TermStatus;
pub use reporters::text::StdoutReporter;
pub use reporters::text::StringReporter;
pub use task_tree::TaskTree;

pub use reporters::term_status::stdio::stdout;
