use crate::task::Task;
use crate::task_internal::TaskInternal;
use crate::uniq_id::UniqID;
use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::sync::Arc;
use std::sync::RwLock;

lazy_static::lazy_static! {
    pub(crate) static ref TASK_TREE: TaskTree  = TaskTree::new();
}

#[derive(Clone, Default)]

pub(crate) struct TaskTree(pub(crate) Arc<RwLock<TaskTreeInternal>>);

#[derive(Default)]
pub(crate) struct TaskTreeInternal {
    tasks_internal: BTreeMap<UniqID, TaskInternal>,
    parent_to_children: BTreeMap<UniqID, BTreeSet<UniqID>>,
    child_to_parents: BTreeMap<UniqID, BTreeSet<UniqID>>,
    root_tasks: BTreeSet<UniqID>,
}

impl TaskTree {
    pub fn new() -> Self {
        Default::default()
    }

    pub async fn create_task_internal<S: Into<String>>(
        &self,
        name: S,
        parent: Option<UniqID>,
    ) -> Task {
        let task_internal = TaskInternal::new(name.into());
        let task = Task {
            id: task_internal.id,
            task_tree: self.clone(),
            mark_done_on_drop: true,
        };
        self.add_task(task_internal, parent);
        task
    }

    pub(crate) async fn spawn_internal<F, FT, T, S: Into<String> + Clone>(
        &self,
        name: S,
        f: F,
        parent: Option<UniqID>,
    ) -> Result<T>
    where
        F: FnOnce(Task) -> FT,
        FT: Future<Output = Result<T>> + Send + 'static,
        T: Send + 'static,
    {
        let task_internal = TaskInternal::new(name.clone().into());
        let task = Task {
            id: task_internal.id,
            task_tree: self.clone(),
            mark_done_on_drop: false,
        };
        let id = task_internal.id;

        self.add_task(task_internal, parent);

        let result = tokio::spawn(f(task)).await?;

        let mut tree = self.0.write().unwrap();
        tree.get_task_mut(id)?.mark_done(result.is_ok());

        result.with_context(|| format!("Failed to execute task `{}`", name.into()))
    }

    pub fn add_task(&self, task_internal: TaskInternal, parent: Option<UniqID>) {
        let mut tree = self.0.write().unwrap();
        let id = task_internal.id;
        tree.tasks_internal.insert(id, task_internal);
        if let Some(parent) = parent {
            tree.parent_to_children
                .entry(parent)
                .or_insert_with(BTreeSet::new)
                .insert(id);
            tree.child_to_parents
                .entry(id)
                .or_insert_with(BTreeSet::new)
                .insert(parent);
        } else {
            tree.root_tasks.insert(id);
        }
    }
}

impl TaskTreeInternal {
    pub fn get_task_mut(&mut self, id: UniqID) -> Result<&mut TaskInternal> {
        self.tasks_internal
            .get_mut(&id)
            .context("task must be present")
    }

    pub fn get_task(&self, id: UniqID) -> Result<&TaskInternal> {
        self.tasks_internal.get(&id).context("task must be present")
    }

    pub fn root_tasks(&self) -> &BTreeSet<UniqID> {
        &self.root_tasks
    }

    pub fn child_to_parents(&self) -> &BTreeMap<UniqID, BTreeSet<UniqID>> {
        &self.child_to_parents
    }

    pub fn parent_to_children(&self) -> &BTreeMap<UniqID, BTreeSet<UniqID>> {
        &self.parent_to_children
    }
}
