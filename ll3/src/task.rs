use crate::task_tree::TaskTree;
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
        let tree = TaskTree::new();
        tree.create_task_internal(name, None).await
    }

    pub async fn create(&self, name: &str) -> Self {
        self.task_tree
            .create_task_internal(name, Some(self.id))
            .await
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
        let tree = TaskTree::new();
        tree.spawn_internal(name, f, None).await
    }

    pub async fn spawn<F, FT, T>(&self, name: &str, f: F) -> Result<T>
    where
        F: FnOnce(Task) -> FT,
        FT: Future<Output = Result<T>> + Send + 'static,
        T: Send + 'static,
    {
        self.task_tree.spawn_internal(name, f, Some(self.id)).await
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        if self.mark_done_on_drop {
            let task_tree = self.task_tree.clone();
            let id = self.id;
            tokio::spawn(async move {
                if let Ok(task_internal) = task_tree.0.write().await.get_task_mut(id) {
                    task_internal.mark_done(true)
                }
            });
        }
    }
}
