use super::Drain;
use crate::Event;
use chrono::prelude::*;
use chrono::{DateTime, Local, Utc};
use colored::*;

pub const DONTPRINT_TAG: &str = "dontprint";

struct StdoutDrain {
    pub timestamp_format: Option<TimestampFormat>,
}

#[derive(Clone, Copy)]
pub enum TimestampFormat {
    UTC,
    Local,
    None,
    Redacted,
}

impl Drain for StdoutDrain {
    fn log_event(&self, event: &Event) {
        if event.tags.contains(DONTPRINT_TAG) {
            return;
        }

        let timestamp_format = self.timestamp_format.unwrap_or(TimestampFormat::UTC);
        let result = make_string(event, timestamp_format);

        eprint!("{}\n", result);
    }
}

pub fn make_string(event: &Event, timestamp_format: TimestampFormat) -> String {
    let timestamp = match timestamp_format {
        TimestampFormat::None => format!(""),
        TimestampFormat::Redacted => "[<REDACTED>] ".to_string(), // for testing
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
        Some(d) => format!("{}{:<60}|{:>6}ms", timestamp, event_name, d.as_millis()),
        None => format!("{}{}", timestamp, event_name),
    };

    for (k, entry) in &event.data.map {
        if entry.1.contains(DONTPRINT_TAG) {
            continue;
        }

        result.push_str(&format!("\n  |      {}: {}", k, entry.0).dimmed());
    }

    if let Some(error) = &event.error_msg {
        result.push_str("\n");
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
