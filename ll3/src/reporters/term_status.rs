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

/*
 Vec of indentations. Bool represents whether a vertical line needs to be
 at every point of the indentation, e.g.

    [▶] Root Task
    │
    ├ [✓] Task 1
    │  ╰ [▶] Task 3        <-- vec[true, true] has line
    ╰ [✓] Task 1
       ╰ [⨯] Failed task   <-- vec[false, true] no line
*/
type Depth = Vec<bool>;

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
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                t2.print().await.ok();
            }
        });
        t
    }

    async fn print(&mut self) -> Result<()> {
        let tree = self.task_tree.0.read().await;

        let child_to_parents = tree.child_to_parents();
        let parent_to_children = tree.parent_to_children();

        let mut stack: Vec<(UniqID, Depth)> = tree
            .root_tasks()
            .iter()
            .filter(|id| !child_to_parents.contains_key(id))
            .map(|id| (*id, vec![]))
            .collect();

        let mut rows = vec![];
        while let Some((id, depth)) = stack.pop() {
            let task = tree.get_task(id).context("must be present")?;

            let mut children_iter = parent_to_children.get(&id).into_iter().flatten().peekable();
            let mut append_to_stack = vec![];

            while let Some(subtask_id) = children_iter.next() {
                let mut new_depth = depth.clone();
                new_depth.push(children_iter.peek().is_some());
                append_to_stack.push((*subtask_id, new_depth));
            }

            // Since we're popping, we'll be going through children in reverse order,
            // so we need to counter that.
            append_to_stack.reverse();

            stack.append(&mut append_to_stack);

            rows.push(self.task_row(task, depth)?);
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

    fn task_row(&self, task_internal: &TaskInternal, mut depth: Depth) -> Result<String> {
        /*

        [▶] Root Task
        │
        ├ [✓] Task 1
        │  ╰ [▶] Task 3
        ├ [✓] Task 1
        ╰ [⨯] Failed task
        */

        let indent = if let Some(last_indent) = depth.pop() {
            // Wrost case utf8 symbol pre level is 4 bytes
            let mut indent = String::with_capacity(4 * depth.len());
            for (i, has_vertical_line) in depth.into_iter().enumerate() {
                if has_vertical_line {
                    indent.push_str("│ ");
                } else {
                    indent.push_str("  ");
                }
            }

            if last_indent {
                indent.push_str("├ ");
            } else {
                indent.push_str("╰ ");
            }

            indent
        } else {
            String::new()
        };

        let status = match task_internal.status {
            TaskStatus::Running => " ▶ ".black().on_yellow(),
            TaskStatus::Finished(TaskResult::Success, _) => " ✓ ".black().on_green(),
            TaskStatus::Finished(TaskResult::Failure, _) => " x ".on_red(),
        };

        let duration = match task_internal.status {
            TaskStatus::Finished(_, finished_at) => {
                finished_at.duration_since(task_internal.started_at)
            }
            _ => task_internal.started_at.elapsed(),
        }?;

        let indent_len = indent.len();
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
