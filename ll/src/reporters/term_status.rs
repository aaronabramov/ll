use super::Level;
use crate::task_tree::{TaskInternal, TaskResult, TaskStatus, TaskTree, TASK_TREE};
use crate::uniq_id::UniqID;
use anyhow::{Context, Result};
use colored::Colorize;
use crossterm::{cursor, style, terminal};
use std::io::Write;
use std::sync::Arc;
use std::sync::RwLock;

const NOSTATUS_TAG: &str = "nostatus";

lazy_static::lazy_static! {
    pub static ref TERM_STATUS: TermStatus = TermStatus::new(TASK_TREE.clone());
}

pub async fn show() {
    TERM_STATUS.show();
}

pub async fn hide() {
    TERM_STATUS.hide();
}

#[derive(Clone)]
pub struct TermStatus(Arc<RwLock<TermStatusInternal>>);

impl TermStatus {
    fn new(task_tree: Arc<TaskTree>) -> Self {
        Self(Arc::new(RwLock::new(TermStatusInternal::new(task_tree))))
    }

    pub fn show(&self) {
        let mut lock = self.0.write().unwrap();
        if lock.enabled {
            return;
        } else {
            lock.enabled = true;
        }
        drop(lock);

        let t = self.clone();
        std::thread::spawn(move || {
            loop {
                // This is dumb, but it lets regular `println!` macros and such
                // time to acquire a global mutex to print whatever they want to
                // print. Without it this fuction will release and acquire the
                // lock right away without letting anything print at all.
                std::thread::sleep(std::time::Duration::from_millis(1));
                let stdout = std::io::stdout();
                let stderr = std::io::stderr();

                // Get both locks for stdout and stderr so nothing can print to
                // it while the status tree is displayed. If something prints
                // while the tree si there everything will get messed up, output
                // will be lost and parts of tree will end up as random noise.
                let stdout_lock = stdout.lock();
                let mut stderr_lock = stderr.lock();

                let mut internal = t.0.write().unwrap();
                if internal.enabled {
                    internal.print(&mut stderr_lock).ok();
                } else {
                    break;
                }
                // STDIO is locked the whole time.
                // WARN: If there a heavy IO
                // happening this will obviously slow things down quite a bit.
                std::thread::sleep(std::time::Duration::from_millis(50));

                if internal.enabled {
                    internal.clear(&mut stderr_lock).ok();
                } else {
                    break;
                }

                drop(stdout_lock);
                drop(stderr_lock);
            }
        });
    }

    pub fn hide(&self) {
        self.0.write().unwrap().enabled = false;
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
    task_tree: Arc<TaskTree>,
    pub max_log_level: Level,
    enabled: bool,
}

impl TermStatusInternal {
    fn new(task_tree: Arc<TaskTree>) -> Self {
        Self {
            current_height: 0,
            task_tree,
            max_log_level: Level::default(),
            enabled: false,
        }
    }

    fn print(&mut self, stdio: &mut impl Write) -> Result<()> {
        let rows = self.make_status_rows()?;
        let height = rows.len();

        if let (0, 0) = (height, self.current_height) {
            return Ok(());
        }

        self.current_height = height;

        crossterm::execute!(stdio, style::Print("\n")).ok();
        crossterm::execute!(stdio, style::Print(rows.join("\n"))).ok();
        crossterm::execute!(stdio, style::Print("\n")).ok();

        Ok(())
    }

    fn make_status_rows(&self) -> Result<Vec<String>> {
        let tree = self.task_tree.tree_internal.read().unwrap();
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

            let dontprint = !self.should_print(task);

            let children_iter = parent_to_children.get(&id).into_iter().flatten().peekable();
            let mut append_to_stack = vec![];

            let last_visible_child = children_iter
                .clone()
                .filter(|id| tree.get_task(**id).map_or(false, |t| self.should_print(t)))
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

    fn should_print(&self, task: &TaskInternal) -> bool {
        let level = super::utils::parse_level(task);
        !task.tags.contains(NOSTATUS_TAG) && (level <= self.max_log_level)
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
            TaskStatus::Finished(TaskResult::Failure(_), _) => " x ".white().on_red(),
        };

        let progress = make_progress(task_internal);

        let duration = match task_internal.status {
            TaskStatus::Finished(_, finished_at) => {
                finished_at.duration_since(task_internal.started_at)
            }
            _ => task_internal.started_at.elapsed(),
        }?;

        let secs = duration.as_secs();
        let millis = (duration.as_millis() % 1000) / 100;
        let ts = format!(" [{}.{}s] ", secs, millis).dimmed();

        Ok(format!(
            "{}{}{}{}{}",
            indent, status, ts, progress, task_internal.name
        ))
    }

    fn clear(&self, stdio: &mut impl Write) -> Result<()> {
        if self.current_height != 0 {
            for _ in 0..(self.current_height + 1) {
                crossterm::execute!(stdio, terminal::Clear(terminal::ClearType::CurrentLine)).ok();
                crossterm::execute!(stdio, cursor::MoveUp(1)).ok();
            }
        }

        Ok(())
    }
}

fn make_progress(task: &TaskInternal) -> String {
    const PROGRESS_BAR_LEN: i64 = 30;

    if let Some((done, total)) = &task.progress {
        let pct_done = (done * 100) / total;
        let done_blocks_len = std::cmp::min((PROGRESS_BAR_LEN * pct_done) / 100, PROGRESS_BAR_LEN);
        let todo_blocks_len = PROGRESS_BAR_LEN - done_blocks_len;
        let done_blocks = " ".repeat(done_blocks_len as usize).on_bright_green();
        let todo_blocks = ".".repeat(todo_blocks_len as usize).on_black();
        format!(" [{}{}] {}/{} ", done_blocks, todo_blocks, done, total)
    } else {
        String::new()
    }
}
