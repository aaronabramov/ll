/*!
# ll - Logging Library

**ll** is a lightweight logging library. Its main focus is to provide the ability
to manually instrument portions of code to track and log its execution.

Instrumentation of the code is done by wrapping parts of code into `Tasks`.
Tasks emit a `start` event when the task is started and `end` event when it's finished.

These events are consumed by `Reporters`. Multiple reporters can be used at the same time
and they will each receive task events. Different reporters can report/log task events to
different systems/sources, e.g. print them to STDOUT, write to a database, file or
third-party system.

Tasks are organized in a task tree. Each task can spawn multiple subtasks and there's always
parent-child relationship between them.
TaskTree is the main struct that holds configuration for how to spawn/log/report tasks.

Example

```
use ll::Task;

async fn do_something() {
    ll::reporters::term_status::show();

    let root_task = Task::create_new("root_task");
    root_task.spawn("subtask_1", |task| async move {
        task.spawn_sync("subtask_2", |task| {
            // do other stuff
            Ok(())
        })?;
        Ok(())
    }).await.unwrap();
}
```

 */
#![allow(clippy::new_without_default)]

pub mod data;
pub mod level;
pub mod task;
pub mod task_tree;
pub mod uniq_id;
pub mod utils;

pub use task::Task;

pub mod reporters;
pub use task_tree::add_reporter;

#[cfg(test)]
mod tests;

pub use data::{Data, DataEntry, DataValue};
pub use reporters::term_status::TermStatus;
pub use reporters::text::StdioReporter;
pub use reporters::text::StringReporter;
pub use task_tree::ErrorFormatter;
pub use task_tree::TaskInternal;
pub use task_tree::TaskTree;
