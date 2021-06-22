use crate::uniq_id::UniqID;
use std::time::SystemTime;

pub struct TaskInternal {
    pub id: UniqID,
    pub name: String,
    pub started_at: SystemTime,
    pub status: TaskStatus,
}

pub enum TaskStatus {
    Running,
    Finished(TaskResult, SystemTime),
}

pub enum TaskResult {
    Success,
    Failure,
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

    pub(crate) fn mark_done(&mut self, success: bool) {
        let tast_status = match success {
            true => TaskResult::Success,
            false => TaskResult::Failure,
        };
        self.status = TaskStatus::Finished(tast_status, SystemTime::now());
    }
}
