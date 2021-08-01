use super::DONTPRINT_TAG;
use crate::task_tree::{TaskInternal, TaskResult, TaskStatus};
use chrono::prelude::*;
use chrono::{DateTime, Local, Utc};
use colored::*;
use std::sync::{Arc, Mutex, RwLock};

use super::Reporter;

/// Simple drain that logs everything into STDOUT
pub struct StdioReporter {
    pub timestamp_format: Option<TimestampFormat>,
    /// By default this reporter writes to STDERR,
    /// this flag will make it write to STDOUT instead
    pub use_stdout: bool,
}

// Similar to STDOUT drain, but instead logs everything into a string
// that it owns that can later be inspecetd/dumped.
#[derive(Clone)]
pub struct StringReporter {
    pub output: Arc<Mutex<String>>,
    timestamp_format: Arc<RwLock<TimestampFormat>>,
    duration_format: Arc<RwLock<DurationFormat>>,
    strip_ansi: bool,
}

impl StdioReporter {
    pub fn new() -> Self {
        Self {
            timestamp_format: None,
            use_stdout: false,
        }
    }
}

#[derive(Clone, Copy)]
#[allow(clippy::upper_case_acronyms)]
pub enum TimestampFormat {
    UTC,
    Local,
    None,
    Redacted,
}

#[derive(Clone, Copy)]
pub enum DurationFormat {
    Milliseconds,
    None,
}

#[async_trait::async_trait]
impl Reporter for StdioReporter {
    async fn task_end(&self, task_internal: Arc<TaskInternal>) {
        if task_internal.tags.contains(DONTPRINT_TAG) {
            return;
        }

        let timestamp_format = self.timestamp_format.unwrap_or(TimestampFormat::UTC);
        let result = make_string(
            &task_internal,
            timestamp_format,
            DurationFormat::Milliseconds,
        );

        if self.use_stdout {
            println!("{}", result);
        } else {
            eprintln!("{}", result);
        }
    }
}

pub fn strip_ansi(s: &str) -> String {
    String::from_utf8(
        strip_ansi_escapes::strip(s).expect("Cant strip ANSI escape characters from a string"),
    )
    .expect("not a utf8 string")
}

impl StringReporter {
    pub fn new() -> Self {
        Self {
            output: Arc::new(Mutex::new(String::new())),
            timestamp_format: Arc::new(RwLock::new(TimestampFormat::Redacted)),
            duration_format: Arc::new(RwLock::new(DurationFormat::None)),
            strip_ansi: true,
        }
    }

    pub fn set_timestamp_format(&self, format: TimestampFormat) {
        *self.timestamp_format.write().unwrap() = format;
    }

    pub fn log_duration(&self, enabled: bool) {
        *self.duration_format.write().unwrap() = if enabled {
            DurationFormat::Milliseconds
        } else {
            DurationFormat::None
        };
    }
}

#[async_trait::async_trait]
impl Reporter for StringReporter {
    async fn task_end(&self, task_internal: Arc<TaskInternal>) {
        if task_internal.tags.contains(DONTPRINT_TAG) {
            return;
        }
        let timestamp_format = *self.timestamp_format.read().unwrap();
        let duration_format = *self.duration_format.read().unwrap();
        let mut result = make_string(&task_internal, timestamp_format, duration_format);
        if self.strip_ansi {
            result = strip_ansi(&result);
        }
        let mut output = self.output.lock().expect("poisoned lock");
        output.push_str(&result);
        output.push('\n');
    }
}

impl std::fmt::Display for StringReporter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self.output.lock().expect("poisoned lock");
        write!(f, "{}", &s)
    }
}

pub fn make_string(
    task_internal: &TaskInternal,
    timestamp_format: TimestampFormat,
    duration_format: DurationFormat,
) -> String {
    let timestamp = match timestamp_format {
        TimestampFormat::None => format!(""),
        TimestampFormat::Redacted => "[ ] ".to_string(), // for testing
        TimestampFormat::Local => {
            let datetime: DateTime<Local> = task_internal.started_at.into();
            let rounded = datetime.round_subsecs(0);
            let formatted = rounded.format("%I:%M:%S%p");
            format!("[{}] ", formatted).dimmed().to_string()
        }
        TimestampFormat::UTC => {
            let datetime: DateTime<Utc> = task_internal.started_at.into();
            let rounded = datetime.round_subsecs(0);
            format!("[{:?}] ", rounded).dimmed().to_string()
        }
    };

    let name = if matches!(
        task_internal.status,
        TaskStatus::Finished(TaskResult::Failure(_), _)
    ) {
        format!("[ERR] {}", task_internal.full_name()).red()
    } else {
        task_internal.full_name().yellow()
    };

    let duration = if let TaskStatus::Finished(_, finished_at) = task_internal.status {
        finished_at.duration_since(task_internal.started_at).ok()
    } else {
        None
    };

    let mut result = match duration {
        Some(d) => format!(
            "{}{:<60}{}",
            timestamp,
            name,
            format_duration(d, duration_format)
        ),
        None => format!("{}{}", timestamp, name),
    };

    let mut data = vec![];
    for (k, entry) in task_internal.all_data() {
        if entry.1.contains(DONTPRINT_TAG) {
            continue;
        }

        data.push(format!("  |      {}: {}", k, entry.0).dimmed().to_string());
    }

    if !data.is_empty() {
        result.push('\n');
        result.push_str(&data.join("\n"));
    }

    if let TaskStatus::Finished(TaskResult::Failure(error_msg), _) = &task_internal.status {
        result.push_str("\n  |\n");
        let error_log = error_msg
            .split('\n')
            .map(|line| format!("  |  {}", line))
            .collect::<Vec<String>>()
            .join("\n");
        result.push_str(&error_log);
    }

    result
}

fn format_duration(d: std::time::Duration, format: DurationFormat) -> String {
    match format {
        DurationFormat::Milliseconds => format!("|{:>6}ms", d.as_millis()),
        DurationFormat::None => String::new(),
    }
}
