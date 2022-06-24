use crate::data::{Data, DataEntry, DataValue};
use crate::reporters::Reporter;
use crate::task::{Task, TaskData};
use crate::uniq_id::UniqID;
use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use std::time::Duration;
use std::time::SystemTime;

lazy_static::lazy_static! {
    pub static ref TASK_TREE: Arc<TaskTree>  = TaskTree::new();
}

pub fn add_reporter(reporter: Arc<dyn Reporter>) {
    TASK_TREE.add_reporter(reporter);
}

#[derive(Default)]
pub struct TaskTree {
    pub(crate) tree_internal: RwLock<TaskTreeInternal>,
    /// If true, it will block the current thread until all task events are
    /// reported (e.g. written to STDOUT)
    force_flush: AtomicBool,
}

#[derive(Default)]
pub(crate) struct TaskTreeInternal {
    tasks_internal: BTreeMap<UniqID, TaskInternal>,
    parent_to_children: BTreeMap<UniqID, BTreeSet<UniqID>>,
    child_to_parents: BTreeMap<UniqID, BTreeSet<UniqID>>,
    root_tasks: BTreeSet<UniqID>,
    reporters: Vec<Arc<dyn Reporter>>,
    tasks_marked_for_deletion: HashMap<UniqID, SystemTime>,
    report_start: Vec<UniqID>,
    report_end: Vec<UniqID>,
    data_transitive: Data,
    remove_task_after_done_ms: u64,
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
    /// optional tuple containing values indicating task progress, where
    /// first value is how many items finished and the second value is how many
    /// items there are total. E.g. if it's a task processing 10 pieces of work,
    /// (1, 10) would mean that 1 out of ten pieces is done.
    pub progress: Option<(i64, i64)>,
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
    pub fn new() -> Arc<Self> {
        let s = Arc::new(Self {
            tree_internal: RwLock::new(TaskTreeInternal {
                tasks_internal: BTreeMap::new(),
                parent_to_children: BTreeMap::new(),
                child_to_parents: BTreeMap::new(),
                root_tasks: BTreeSet::new(),
                reporters: vec![],
                tasks_marked_for_deletion: HashMap::new(),
                report_start: vec![],
                report_end: vec![],
                data_transitive: Data::empty(),
                remove_task_after_done_ms: 0,
            }),
            force_flush: AtomicBool::new(false),
        });
        let clone = s.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                let mut tree = clone.tree_internal.write().unwrap();
                tree.garbage_collect();
            }
        });
        let clone = s.clone();
        thread::spawn(move || loop {
            thread::sleep(std::time::Duration::from_millis(10));
            clone.report_all();
        });

        s
    }

    pub fn set_force_flush(&self, enabled: bool) {
        self.force_flush.store(enabled, Ordering::SeqCst)
    }

    pub fn force_flush_enabled(&self) -> bool {
        self.force_flush.load(Ordering::SeqCst)
    }

    pub fn create_task(self: &Arc<Self>, name: &str) -> Task {
        let id = self.create_task_internal(name, None);
        Task(Arc::new(TaskData {
            id,
            task_tree: self.clone(),
            mark_done_on_drop: true,
        }))
    }

    pub fn add_reporter(&self, reporter: Arc<dyn Reporter>) {
        self.tree_internal.write().unwrap().reporters.push(reporter);
    }

    fn pre_spawn(self: &Arc<Self>, name: String, parent: Option<UniqID>) -> Task {
        let task = Task(Arc::new(TaskData {
            id: self.create_task_internal(&name, parent),
            task_tree: self.clone(),
            mark_done_on_drop: false,
        }));
        self.maybe_force_flush();
        task
    }

    fn post_spawn<T>(self: &Arc<Self>, id: UniqID, result: Result<T>) -> Result<T> {
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
        self.maybe_force_flush();
        result
    }

    pub fn spawn_sync<F, T>(self: &Arc<Self>, name: &str, f: F, parent: Option<UniqID>) -> Result<T>
    where
        F: FnOnce(Task) -> Result<T>,
        T: Send,
    {
        let task = self.pre_spawn(name.into(), parent);
        let id = task.0.id;
        let result = f(task);
        self.post_spawn(id, result)
    }

    pub(crate) async fn spawn<F, FT, T, S: Into<String> + Clone>(
        self: &Arc<Self>,
        name: S,
        f: F,
        parent: Option<UniqID>,
    ) -> Result<T>
    where
        F: FnOnce(Task) -> FT,
        FT: Future<Output = Result<T>> + Send,
        T: Send,
    {
        let task = self.pre_spawn(name.into(), parent);
        let id = task.0.id;
        let result = f(task).await;
        self.post_spawn(id, result)
    }

    pub fn create_task_internal<S: Into<String>>(
        self: &Arc<Self>,
        name: S,
        parent: Option<UniqID>,
    ) -> UniqID {
        let mut tree = self.tree_internal.write().unwrap();

        let mut parent_names = vec![];
        let mut data_transitive = tree.data_transitive.clone();
        let (name, tags) = crate::utils::extract_tags(name.into());
        let id = UniqID::new();
        if let Some(parent_task) = parent.and_then(|pid| tree.tasks_internal.get(&pid)) {
            parent_names = parent_task.parent_names.clone();
            parent_names.push(parent_task.name.clone());
            data_transitive.merge(&parent_task.data_transitive);
            let parent_id = parent_task.id;

            tree.parent_to_children
                .entry(parent_id)
                .or_insert_with(BTreeSet::new)
                .insert(id);
            tree.child_to_parents
                .entry(id)
                .or_insert_with(BTreeSet::new)
                .insert(parent_id);
        } else {
            tree.root_tasks.insert(id);
        }

        let task_internal = TaskInternal {
            status: TaskStatus::Running,
            name,
            parent_names,
            id,
            started_at: SystemTime::now(),
            data: Data::empty(),
            data_transitive,
            tags,
            progress: None,
        };

        tree.tasks_internal.insert(id, task_internal);
        tree.report_start.push(id);

        id
    }

    pub fn mark_done(&self, id: UniqID, error_message: Option<String>) {
        let mut tree = self.tree_internal.write().unwrap();
        if let Some(task_internal) = tree.tasks_internal.get_mut(&id) {
            task_internal.mark_done(error_message);
            tree.mark_for_gc(id);
            tree.report_end.push(id);
        }
    }

    pub fn add_data<S: Into<String>, D: Into<DataValue>>(&self, id: UniqID, key: S, value: D) {
        let mut tree = self.tree_internal.write().unwrap();
        if let Some(task_internal) = tree.tasks_internal.get_mut(&id) {
            task_internal.data.add(key, value);
        }
    }

    pub(crate) fn add_data_transitive_for_task<S: Into<String>, D: Into<DataValue>>(
        &self,
        id: UniqID,
        key: S,
        value: D,
    ) {
        let mut tree = self.tree_internal.write().unwrap();
        if let Some(task_internal) = tree.tasks_internal.get_mut(&id) {
            task_internal.data_transitive.add(key, value);
        }
    }

    /// Add transitive data to the task tree. This transitive data will be
    /// added to every task created in this task tree
    pub fn add_data_transitive<S: Into<String>, D: Into<DataValue>>(&self, key: S, value: D) {
        let mut tree = self.tree_internal.write().unwrap();
        tree.data_transitive.add(key, value);
    }

    pub fn task_progress(&self, id: UniqID, done: i64, total: i64) {
        let mut tree = self.tree_internal.write().unwrap();
        if let Some(task_internal) = tree.tasks_internal.get_mut(&id) {
            task_internal.progress = Some((done, total));
        }
    }

    fn get_cloned_task(&self, id: UniqID) -> Option<TaskInternal> {
        let tree = self.tree_internal.read().unwrap();
        tree.get_task(id).ok().cloned()
    }

    /// If force_flush set to true, this function will block the thread until everything
    /// is reported. Useful for cases when the process exits before all async events
    /// are reported and stuff is missing from stdout.
    pub fn maybe_force_flush(&self) {
        if self.force_flush.load(Ordering::SeqCst) {
            self.report_all();
        }
    }

    pub fn report_all(&self) {
        let mut tree = self.tree_internal.write().unwrap();
        let (start_tasks, end_tasks, reporters) = tree.get_tasks_and_reporters();
        drop(tree);
        for reporter in reporters {
            for task in &start_tasks {
                reporter.task_start(task.clone());
            }
            for task in &end_tasks {
                reporter.task_end(task.clone());
            }
        }
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
                if elapsed > Duration::from_millis(self.remove_task_after_done_ms) {
                    will_delete.push(*id);
                }
            }
        }

        for id in will_delete {
            self.tasks_internal.remove(&id);
            self.parent_to_children.remove(&id);
            self.root_tasks.remove(&id);
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

    #[allow(clippy::type_complexity)]
    fn get_tasks_and_reporters(
        &mut self,
    ) -> (
        Vec<Arc<TaskInternal>>,
        Vec<Arc<TaskInternal>>,
        Vec<Arc<dyn Reporter>>,
    ) {
        let mut start_ids = vec![];
        std::mem::swap(&mut start_ids, &mut self.report_start);
        let mut end_ids = vec![];
        std::mem::swap(&mut end_ids, &mut self.report_end);

        let mut start_tasks = vec![];
        let mut end_tasks = vec![];

        for id in start_ids {
            if let Ok(task_internal) = self.get_task(id) {
                start_tasks.push(Arc::new(task_internal.clone()));
            }
        }
        for id in end_ids {
            if let Ok(task_internal) = self.get_task(id) {
                end_tasks.push(Arc::new(task_internal.clone()));
            }
        }

        let reporters = self.reporters.clone();

        (start_tasks, end_tasks, reporters)
    }
}

impl TaskInternal {
    pub(crate) fn mark_done(&mut self, error_message: Option<String>) {
        let task_status = match error_message {
            None => TaskResult::Success,
            Some(msg) => TaskResult::Failure(msg),
        };
        self.status = TaskStatus::Finished(task_status, SystemTime::now());
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
