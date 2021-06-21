use crate::task_internal::{TaskInternal, TaskResult, TaskStatus};
use crate::task_tree::TaskTree;
use crate::uniq_id::UniqID;
use anyhow::{Context, Result};
use colored::Colorize;
use crossterm::{cursor, style, terminal, ExecutableCommand};
use std::io::stdout;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct TermStatus(Arc<RwLock<TermStatusInternal>>);

impl TermStatus {
    pub fn new(task: &crate::task::Task) -> Self {
        Self(Arc::new(RwLock::new(TermStatusInternal::new(task))))
    }
}

#[derive(Clone)]
pub struct TermStatusInternal {
    current_height: usize,
    task_tree: TaskTree,
}

impl TermStatusInternal {
    pub fn new(task: &crate::task::Task) -> Self {
        let t = Self {
            current_height: 0,
            task_tree: task.task_tree.clone(),
        };

        let mut t2 = t.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                t2.print().await.ok();
            }
        });
        t
    }
    // async fn spawn<F, FT, T>(&self, name: &str, f: F) -> Result<T>
    // where
    //     F: FnOnce(Task) -> FT,
    //     FT: Future<Output = Result<T>> + Send + 'static,
    //     T: Send + 'static,
    // {
    //     self.spawn_internal(name, f, None).await
    // }

    // async fn spawn_internal<F, FT, T>(&self, name: &str, f: F, parent: Option<UniqID>) -> Result<T>
    // where
    //     F: FnOnce(Task) -> FT,
    //     FT: Future<Output = Result<T>> + Send + 'static,
    //     T: Send + 'static,
    // {
    //     let mut status = self.0.write().await;
    //     let task_internal = TaskInternal::new(name);
    //     let id = task_internal.id;

    //     if let Some(parent) = parent {
    //         let parent = status
    //             .tasks_internal
    //             .get_mut(&parent)
    //             .context("parent must be there")?;
    //         parent.subtasks.insert(id);
    //     }

    //     let task = task_internal.task(self);
    //     status.tasks_internal.insert(id, task_internal);
    //     drop(status);
    //     let result = tokio::spawn(f(task)).await?;
    //     let mut status = self.0.write().await;

    //     let task_internal = status
    //         .tasks_internal
    //         .get_mut(&id)
    //         .context("Task must be present in the tree")?;

    //     let tast_status = match result.is_ok() {
    //         true => TaskResult::Success,
    //         false => TaskResult::Failure,
    //     };
    //     task_internal.status = TaskStatus::Finished(tast_status, SystemTime::now());

    //     result.with_context(|| format!("Failed to execute task `{}`", name))
    // }

    async fn print(&mut self) -> Result<()> {
        let tree = self.task_tree.0.read().await;

        let child_to_parents = tree.child_to_parents();
        let parent_to_children = tree.parent_to_children();
        type Depth = usize;
        let mut stack: Vec<(UniqID, Depth)> = tree
            .root_tasks()
            .iter()
            .filter(|id| !child_to_parents.contains_key(id))
            .map(|id| (*id, 0))
            .collect();

        let mut rows = vec![];
        while let Some((id, depth)) = stack.pop() {
            let task = tree.get_task(id).context("must be present")?;
            rows.push(self.task_row(task, depth)?);

            for subtask_id in parent_to_children.get(&id).into_iter().flatten() {
                stack.push((*subtask_id, depth + 1));
            }
        }

        let height = rows.len();

        self.clear(self.current_height)?;
        self.current_height = height;

        let mut stdout = stdout();
        for row in rows {
            stdout.execute(style::Print(row))?;
        }

        Ok(())
    }

    fn task_row(&self, task_internal: &TaskInternal, depth: usize) -> Result<String> {
        let indent = "  ".repeat(depth);
        let status = match task_internal.status {
            TaskStatus::Running => "[ RUNS ]".black().on_yellow(),
            TaskStatus::Finished(TaskResult::Success, _) => "[  OK  ]".black().on_green(),
            TaskStatus::Finished(TaskResult::Failure, _) => "[ FAIL ]".on_red(),
        };

        let duration = match task_internal.status {
            TaskStatus::Finished(_, finished_at) => {
                finished_at.duration_since(task_internal.started_at)
            }
            _ => task_internal.started_at.elapsed(),
        }?;

        let indent_len = depth * 2;
        Ok(format!(
            "{}{} {}{}{:?}\n",
            indent,
            status,
            task_internal.name,
            " ".repeat(50 - indent_len), // spacer
            duration,
        ))
        //
    }

    fn clear(&self, height: usize) -> Result<()> {
        let mut stdout = stdout();

        for _ in 0..height {
            stdout.execute(cursor::MoveUp(1))?;
            stdout.execute(terminal::Clear(terminal::ClearType::CurrentLine))?;
        }

        Ok(())
    }
}
