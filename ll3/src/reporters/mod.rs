pub mod term_status;
mod text;

pub use term_status::TermStatus;

pub trait Reporter {
    fn task_start(task: &crate::task_internal::TaskInternal);
}
