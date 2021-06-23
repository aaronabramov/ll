use crate::uniq_id::UniqID;
use std::time::SystemTime;

#[derive(Clone)]
pub struct TaskInternal {
    pub id: UniqID,
    pub name: String,
    pub started_at: SystemTime,
    pub status: TaskStatus,
}

#[derive(Clone)]
pub enum TaskStatus {
    Running,
    Finished(TaskResult, SystemTime),
}

#[derive(Clone)]
pub enum TaskResult {
    Success,
    Failure(String),
}

impl TaskInternal {
    pub(crate) fn new<S: Into<String>>(s: S) -> Self {
        Self {
            status: TaskStatus::Running,
            name: s.into(),
            id: UniqID::new(),
            started_at: SystemTime::now(),
        }
    }

    pub(crate) fn mark_done(&mut self, error_message: Option<String>) {
        let tast_status = match error_message {
            None => TaskResult::Success,
            Some(msg) => TaskResult::Failure(msg),
        };
        self.status = TaskStatus::Finished(tast_status, SystemTime::now());
    }
}
