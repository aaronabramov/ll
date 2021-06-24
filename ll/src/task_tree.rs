use crate::data::{Data, DataEntry, DataValue};
use crate::reporters::Reporter;
use crate::task::Task;
use crate::uniq_id::UniqID;
use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::future::Future;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;
use std::time::SystemTime;

const REMOVE_TASK_AFTER_DONE_MS: u64 = 5000;

lazy_static::lazy_static! {
    pub(crate) static ref TASK_TREE: TaskTree  = TaskTree::new();
}

pub fn add_reporter(reporter: Arc<dyn Reporter>) {
    TASK_TREE.add_reporter(reporter);
}

#[derive(Clone, Default)]
pub struct TaskTree(pub(crate) Arc<RwLock<TaskTreeInternal>>);

#[derive(Default)]
pub(crate) struct TaskTreeInternal {
    tasks_internal: BTreeMap<UniqID, TaskInternal>,
    parent_to_children: BTreeMap<UniqID, BTreeSet<UniqID>>,
    child_to_parents: BTreeMap<UniqID, BTreeSet<UniqID>>,
    root_tasks: BTreeSet<UniqID>,
    reporters: Vec<Arc<dyn Reporter>>,
    tasks_marked_for_deletion: HashMap<UniqID, SystemTime>,
}

#[derive(Clone)]
pub struct TaskInternal {
    pub id: UniqID,
    pub name: String,
    pub parent_names: Vec<String>,
    pub started_at: SystemTime,
    pub status: TaskStatus,
    pub data: Data,
    pub data_transitive: Data,
    pub tags: BTreeSet<String>,
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

impl TaskTree {
    pub fn new() -> Self {
        let s = Self::default();
        let clone = s.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                let mut tree = clone.0.write().unwrap();
                tree.garbage_collect();
            }
        });
        s
    }

    pub fn create_task(&self, name: &str) -> Task {
        let id = self.create_task_internal(name, None);
        Task {
            id,
            task_tree: self.clone(),
            mark_done_on_drop: true,
        }
    }

    pub fn add_reporter(&self, reporter: Arc<dyn Reporter>) {
        self.0.write().unwrap().reporters.push(reporter);
    }

    fn pre_spawn(&self, name: String, parent: Option<UniqID>) -> Task {
        Task {
            id: self.create_task_internal(&name, parent),
            task_tree: self.clone(),
            mark_done_on_drop: false,
        }
    }

    fn post_spawn<T>(&self, id: UniqID, result: Result<T>) -> Result<T> {
        let result = result.with_context(|| {
            let mut desc = String::from("[Task]");
            if let Some(task_internal) = self.get_cloned_task(id) {
                desc.push_str(&format!(" {}", task_internal.name));
                for (k, v) in task_internal.all_data() {
                    desc.push_str(&format!("\n  {}: {}", k, v.0));
                }
                if !desc.is_empty() {
                    desc.push('\n');
                }
            }
            desc
        });
        self.mark_done(id, result.as_ref().err().map(|e| format!("{:?}", e)));
        result
    }

    pub fn spawn_sync<F, T>(&self, name: &str, f: F, parent: Option<UniqID>) -> Result<T>
    where
        F: FnOnce(Task) -> Result<T>,
        T: Send + 'static,
    {
        let task = self.pre_spawn(name.into(), parent);
        let id = task.id;
        let result = f(task);
        self.post_spawn(id, result)
    }

    pub(crate) async fn spawn<F, FT, T, S: Into<String> + Clone>(
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
        let task = self.pre_spawn(name.into(), parent);
        let id = task.id;
        let result = tokio::spawn(f(task)).await?;
        self.post_spawn(id, result)
    }

    pub fn create_task_internal<S: Into<String>>(&self, name: S, parent: Option<UniqID>) -> UniqID {
        let mut tree = self.0.write().unwrap();

        let mut parent_names = vec![];
        let mut data_transitive = Data::empty();
        let (name, tags) = crate::utils::extract_tags(name.into());
        if let Some(parent) = parent {
            if let Ok(parent_task) = tree.get_task(parent) {
                parent_names = parent_task.parent_names.clone();
                parent_names.push(parent_task.name.clone());
                data_transitive = parent_task.data_transitive.clone();
            }
        }

        let task_internal = TaskInternal {
            status: TaskStatus::Running,
            name,
            parent_names,
            id: UniqID::new(),
            started_at: SystemTime::now(),
            data: Data::empty(),
            data_transitive,
            tags,
        };

        let t_arc = Arc::new(task_internal.clone());
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
        for reporter in &tree.reporters {
            let r = reporter.clone();
            let t = t_arc.clone();
            tokio::spawn(async move {
                r.task_start(t).await;
            });
        }
        id
    }

    pub fn mark_done(&self, id: UniqID, error_message: Option<String>) {
        let mut tree = self.0.write().unwrap();
        if let Some(task_internal) = tree.tasks_internal.get_mut(&id) {
            task_internal.mark_done(error_message);
            let t_arc = Arc::new(task_internal.clone());

            for reporter in &tree.reporters {
                let r = reporter.clone();
                let t = t_arc.clone();
                tokio::spawn(async move {
                    r.task_end(t).await;
                });
            }

            tree.mark_for_gc(id);
        }
    }

    pub fn add_data<S: Into<String>, D: Into<DataValue>>(&self, id: UniqID, key: S, value: D) {
        let mut tree = self.0.write().unwrap();
        if let Some(task_internal) = tree.tasks_internal.get_mut(&id) {
            task_internal.data.add(key, value);
        }
    }

    pub fn add_data_transitive<S: Into<String>, D: Into<DataValue>>(
        &self,
        id: UniqID,
        key: S,
        value: D,
    ) {
        let mut tree = self.0.write().unwrap();
        if let Some(task_internal) = tree.tasks_internal.get_mut(&id) {
            task_internal.data_transitive.add(key, value);
        }
    }

    fn get_cloned_task(&self, id: UniqID) -> Option<TaskInternal> {
        let tree = self.0.read().unwrap();
        tree.get_task(id).ok().cloned()
    }
}

impl TaskTreeInternal {
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

    fn mark_for_gc(&mut self, id: UniqID) {
        let mut stack = vec![id];

        let mut tasks_to_finished_status = BTreeMap::new();

        while let Some(id) = stack.pop() {
            if let Some(task_internal) = self.tasks_internal.get(&id) {
                tasks_to_finished_status
                    .insert(id, matches!(task_internal.status, TaskStatus::Finished(..)));
            }

            for child_id in self.parent_to_children.get(&id).into_iter().flatten() {
                stack.push(*child_id);
            }
        }

        if tasks_to_finished_status
            .iter()
            .all(|(_, finished)| *finished)
        {
            for id in tasks_to_finished_status.keys().copied() {
                self.tasks_marked_for_deletion
                    .entry(id)
                    .or_insert_with(SystemTime::now);
            }

            // This sub branch might have been holding other parent branches that
            // weren't able to be garbage collected because of this subtree. we'll go
            // level up and perform the same logic.
            let parents = self.child_to_parents.get(&id).cloned().unwrap_or_default();
            for parent_id in parents {
                self.mark_for_gc(parent_id);
            }
        }
    }

    fn garbage_collect(&mut self) {
        let mut will_delete = vec![];
        for (id, time) in &self.tasks_marked_for_deletion {
            if let Ok(elapsed) = time.elapsed() {
                if elapsed > Duration::from_millis(REMOVE_TASK_AFTER_DONE_MS) {
                    will_delete.push(*id);
                }
            }
        }

        for id in will_delete {
            self.tasks_internal.remove(&id);
            self.parent_to_children.remove(&id);
            if let Some(parents) = self.child_to_parents.remove(&id) {
                for parent in parents {
                    if let Some(children) = self.parent_to_children.get_mut(&parent) {
                        children.remove(&id);
                    }
                }
            }
            self.tasks_marked_for_deletion.remove(&id);
        }
    }
}

impl TaskInternal {
    pub(crate) fn mark_done(&mut self, error_message: Option<String>) {
        let tast_status = match error_message {
            None => TaskResult::Success,
            Some(msg) => TaskResult::Failure(msg),
        };
        self.status = TaskStatus::Finished(tast_status, SystemTime::now());
    }

    pub fn full_name(&self) -> String {
        let mut full_name = String::new();
        for parent_name in &self.parent_names {
            full_name.push_str(parent_name);
            full_name.push(':');
        }
        full_name.push_str(&self.name);
        full_name
    }

    pub fn all_data(
        &self,
    ) -> std::iter::Chain<
        std::collections::btree_map::Iter<String, DataEntry>,
        std::collections::btree_map::Iter<String, DataEntry>,
    > {
        self.data.map.iter().chain(self.data_transitive.map.iter())
    }
}
