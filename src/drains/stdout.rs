use super::Drain;
use crate::events::Event;
use chrono::prelude::*;
use chrono::{DateTime, Local, Utc};
use colored::*;
use std::sync::{Arc, Mutex};

pub const DONTPRINT_TAG: &str = "dontprint";

/// Simple drain that logs everything into STDOUT
pub struct StdoutDrain {
    pub timestamp_format: Option<TimestampFormat>,
}

// Similar to STDOUT drain, but instead logs everything into a string
// that it owns that can later be inspecetd/dumped.
#[derive(Clone)]
pub struct StringDrain {
    pub output: Arc<Mutex<String>>,
    strip_ansi: bool,
}

impl StdoutDrain {
    pub fn new() -> Self {
        Self {
            timestamp_format: None,
        }
    }
}

#[derive(Clone, Copy)]
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

impl Drain for StdoutDrain {
    fn log_event(&self, event: &Event) {
        if event.tags.contains(DONTPRINT_TAG) {
            return;
        }

        let timestamp_format = self.timestamp_format.unwrap_or(TimestampFormat::UTC);
        let result = make_string(event, timestamp_format, DurationFormat::Milliseconds);

        eprint!("{}", result);
    }
}

pub fn strip_ansi(s: &str) -> String {
    String::from_utf8(
        strip_ansi_escapes::strip(s).expect("Cant strip ANSI escape characters from a string"),
    )
    .expect("not a utf8 string")
}

impl StringDrain {
    pub fn new() -> Self {
        Self {
            output: Arc::new(Mutex::new(String::new())),
            strip_ansi: true,
        }
    }
}

impl Drain for StringDrain {
    fn log_event(&self, event: &Event) {
        if event.tags.contains(DONTPRINT_TAG) {
            return;
        }
        let mut result = make_string(event, TimestampFormat::Redacted, DurationFormat::None);
        if self.strip_ansi {
            result = strip_ansi(&result);
        }
        self.output.lock().expect("poisoned lock").push_str(&result);
    }
}

impl std::fmt::Display for StringDrain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self.output.lock().expect("poisoned lock");
        write!(f, "\n{}\n", &s)
    }
}

pub fn make_string(
    event: &Event,
    timestamp_format: TimestampFormat,
    duration_format: DurationFormat,
) -> String {
    let timestamp = match timestamp_format {
        TimestampFormat::None => format!(""),
        TimestampFormat::Redacted => "[ ] ".to_string(), // for testing
        TimestampFormat::Local => {
            let datetime: DateTime<Local> = event.started_at.clone().into();
            let rounded = datetime.round_subsecs(0);
            let formatted = rounded.format("%I:%M:%S%p");
            format!("[{}] ", formatted).dimmed().to_string()
        }
        TimestampFormat::UTC => {
            let datetime: DateTime<Utc> = event.started_at.clone().into();
            let rounded = datetime.round_subsecs(0);
            format!("[{:?}] ", rounded).dimmed().to_string()
        }
    };

    let event_name = if event.is_error {
        format!("[ERR] {}", event.name).red()
    } else {
        event.name.yellow()
    };

    let mut result = match event.duration {
        Some(d) => format!(
            "{}{:<60}{}",
            timestamp,
            event_name,
            format_duration(d, duration_format)
        ),
        None => format!("{}{}", timestamp, event_name),
    };

    result.push_str("\n");

    for (k, entry) in &event.data.map {
        if entry.1.contains(DONTPRINT_TAG) {
            continue;
        }

        result.push_str(&format!("  |      {}: {}\n", k, entry.0).dimmed());
    }

    if let Some(error) = &event.error_msg {
        result.push_str("  |\n");
        let error_log = error
            .split('\n')
            .map(|line| format!("  |  {}", line))
            .collect::<Vec<String>>()
            .join("\n")
            .red();
        result.push_str(&error_log);
        result.push_str("\n");
    }

    result
}

fn format_duration(d: std::time::Duration, format: DurationFormat) -> String {
    match format {
        DurationFormat::Milliseconds => format!("|{:>6}ms", d.as_millis()),
        DurationFormat::None => String::new(),
    }
}
