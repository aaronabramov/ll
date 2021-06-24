use super::DONTPRINT_TAG;
use crate::task_tree::{TaskInternal, TaskResult, TaskStatus, TaskTree, TASK_TREE};
use crate::uniq_id::UniqID;
use anyhow::{Context, Result};
use colored::Colorize;
use crossterm::{cursor, style, terminal, ExecutableCommand};
use std::io::stdout;
use std::sync::Arc;
use std::sync::RwLock;

lazy_static::lazy_static! {
    pub static ref TERM_STATUS: TermStatus = TermStatus::new(TASK_TREE.clone());
}

pub async fn show() {
    TERM_STATUS.show().await;
}

pub async fn hide() {
    TERM_STATUS.hide().await;
}

#[derive(Clone)]
pub struct TermStatus(Arc<RwLock<TermStatusInternal>>);

impl TermStatus {
    fn new(task_tree: TaskTree) -> Self {
        Self(Arc::new(RwLock::new(TermStatusInternal::new(task_tree))))
    }

    pub async fn show(&self) {
        self.0.write().unwrap().enabled = true;

        let t = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
                let _lock = stdio::LOCK.lock().expect("poisoned lock");
                let mut internal = t.0.write().unwrap();
                if internal.enabled {
                    internal.clear().ok();
                    internal.print().ok();
                } else {
                    break;
                }
            }
        });
    }

    pub async fn hide(&self) {
        self.0.write().unwrap().enabled = false;
    }

    pub async fn clean(&self) -> Result<()> {
        self.0.read().unwrap().clear()?;
        Ok(())
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
    enabled: bool,
}

impl TermStatusInternal {
    fn new(task_tree: TaskTree) -> Self {
        Self {
            current_height: 0,
            task_tree,
            enabled: false,
        }
    }

    fn print(&mut self) -> Result<()> {
        let rows = self.make_status_rows()?;

        let height = rows.len();

        let stdout = stdout();
        let mut lock = stdout.lock();

        self.current_height = height;

        lock.execute(style::Print("\n"))?;
        lock.execute(style::Print(rows.join("\n")))?;
        lock.execute(style::Print("\n"))?;

        Ok(())
    }

    fn make_status_rows(&self) -> Result<Vec<String>> {
        let tree = self.task_tree.0.read().unwrap();
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

            let dontprint = task.tags.contains(DONTPRINT_TAG);

            let children_iter = parent_to_children.get(&id).into_iter().flatten().peekable();
            let mut append_to_stack = vec![];

            let last_visible_child = children_iter
                .clone()
                .filter(|id| {
                    tree.get_task(**id)
                        .map_or(false, |t| !t.tags.contains(DONTPRINT_TAG))
                })
                .last();

            // we still need to DFS the ones that we don't print to make sure
            // we're not skipping their children
            for subtask_id in children_iter {
                let mut new_depth = depth.clone();
                // If we're not printing it, we're not adding the indent either
                // so this tasks children will become children of the parnet task
                if !dontprint {
                    new_depth.push(Some(subtask_id) != last_visible_child);
                }
                append_to_stack.push((*subtask_id, new_depth));
            }

            // Since we're popping, we'll be going through children in reverse order,
            // so we need to counter that.
            append_to_stack.reverse();
            stack.append(&mut append_to_stack);

            if !dontprint {
                rows.push(self.task_row(task, depth)?);
            }
        }

        Ok(rows)
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
            for has_vertical_line in depth.into_iter() {
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
            TaskStatus::Finished(TaskResult::Failure(_), _) => " x ".on_red(),
        };

        let duration = match task_internal.status {
            TaskStatus::Finished(_, finished_at) => {
                finished_at.duration_since(task_internal.started_at)
            }
            _ => task_internal.started_at.elapsed(),
        }?;

        let indent_len = indent.len();
        Ok(format!(
            "{}{} {}{}{:?}",
            indent,
            status,
            task_internal.name,
            " ".repeat(50 - indent_len), // spacer
            duration,
        ))
        //
    }

    fn clear(&self) -> Result<()> {
        let mut stdout = stdout();
        for _ in 0..=self.current_height {
            stdout.execute(terminal::Clear(terminal::ClearType::CurrentLine))?;
            stdout.execute(cursor::MoveUp(1))?;
        }

        Ok(())
    }
}

pub mod stdio {
    use super::TERM_STATUS;
    use std::io::Write;
    use std::sync::Mutex;

    lazy_static::lazy_static! {
        pub(crate) static ref LOCK: Mutex<()> = Mutex::new(());
        pub static ref STDOUT: BufferedStdout = BufferedStdout {};
    }

    pub fn stdout() -> BufferedStdout {
        STDOUT.clone()
    }

    #[macro_export]
    macro_rules! println {
        ($($t:expr),+) => {{
            use std::io::Write;
            let mut stdout = $crate::reporters::term_status::stdio::stdout();
            write!(stdout, $($t),+).unwrap();
            write!(stdout, "\n").unwrap();
        }};
    }

    #[derive(Clone)]
    pub struct BufferedStdout {}

    impl Write for BufferedStdout {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let _lock = LOCK.lock().expect("poisoned lock");
            let mut term_status = TERM_STATUS.0.write().unwrap();
            if term_status.enabled {
                term_status.clear().unwrap();
            }
            let mut stdout = std::io::stdout();
            let result = stdout.write(buf);
            if term_status.enabled {
                term_status.print().unwrap();
            }
            result
        }

        fn flush(&mut self) -> std::io::Result<()> {
            let mut stdout = std::io::stdout();
            stdout.flush()
        }
    }
}
