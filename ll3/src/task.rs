use crate::data::DataValue;
use crate::task_tree::{TaskTree, TASK_TREE};
use crate::uniq_id::UniqID;
use anyhow::Result;
use std::future::Future;

pub type MarkDoneOnDrop = bool;

pub struct Task {
    pub(crate) id: UniqID,
    pub(crate) task_tree: TaskTree,
    pub(crate) mark_done_on_drop: MarkDoneOnDrop,
}

impl Task {
    pub async fn create_new(name: &str) -> Self {
        let id = TASK_TREE.create_task_internal(name, None);
        Self {
            id,
            task_tree: TASK_TREE.clone(),
            mark_done_on_drop: true,
        }
    }

    pub async fn create(&self, name: &str) -> Self {
        let id = self.task_tree.create_task_internal(name, Some(self.id));
        Self {
            id,
            task_tree: self.task_tree.clone(),
            mark_done_on_drop: true,
        }
    }

    /// Spawn a new top level task, with no parent.
    /// This should usually be done in the very beginning of
    /// the process/application.
    pub async fn spawn_new<F, FT, T>(name: &str, f: F) -> Result<T>
    where
        F: FnOnce(Task) -> FT,
        FT: Future<Output = Result<T>> + Send + 'static,
        T: Send + 'static,
    {
        TASK_TREE.spawn(name, f, None).await
    }

    pub async fn spawn<F, FT, T>(&self, name: &str, f: F) -> Result<T>
    where
        F: FnOnce(Task) -> FT,
        FT: Future<Output = Result<T>> + Send + 'static,
        T: Send + 'static,
    {
        self.task_tree.spawn(name, f, Some(self.id)).await
    }

    pub fn spawn_sync<F, T>(&self, name: &str, f: F) -> Result<T>
    where
        F: FnOnce(Task) -> Result<T>,
        T: Send + 'static,
    {
        self.task_tree.spawn_sync(name, f, Some(self.id))
    }

    pub fn data<D: Into<DataValue>>(&self, name: &str, data: D) {
        self.task_tree.add_data(self.id, name, data);
    }

    pub fn data_transitive<D: Into<DataValue>>(&self, name: &str, data: D) {
        self.task_tree.add_data_transitive(self.id, name, data);
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        if self.mark_done_on_drop {
            self.task_tree.mark_done(self.id, None);
        }
    }
}
