use crate::task_tree::TaskStatus;
use crate::TaskInternal;
use anyhow::Result;
use std::fmt::Write;
use std::sync::Mutex;
use std::{
    collections::BTreeMap,
    convert::TryInto,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use super::Reporter;

#[derive(Debug, Clone, serde::Serialize)]
enum EventType {
    Start,
    End,
}

#[derive(Debug, Clone, serde::Serialize)]
struct Event {
    pub name: String,
    pub id: u64,
    pub event_type: EventType,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub data: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unix_ts_millis: Option<u64>,
}

pub struct TraceReporter {
    string: Mutex<String>,
}

impl TraceReporter {
    pub fn new() -> Self {
        Self {
            string: Mutex::new(String::new()),
        }
    }

    fn write(&self, json_line: &str) {
        let mut lock = self.string.lock().unwrap();
        writeln!(lock, "{}", json_line).ok();
    }

    pub fn take(&self) -> String {
        let mut lock = self.string.lock().unwrap();
        std::mem::take(&mut lock)
    }
}

impl Reporter for TraceReporter {
    fn task_start(&self, task: Arc<TaskInternal>) {
        let event = Event {
            name: task.full_name(),
            id: task.id.as_u64(),
            parent_id: task.parent_id.map(|id| id.as_u64()),
            event_type: EventType::Start,
            data: task
                .all_data()
                .map(|(k, v)| (k.to_string(), v.0.to_string()))
                .collect(),
            unix_ts_millis: unix_ts(task.started_at).ok(),
        };

        if let Ok(json) = serde_json::to_string(&event) {
            self.write(&json);
        }
    }

    fn task_end(&self, task: Arc<TaskInternal>) {
        let mut unix_ts_millis = None;
        if let TaskStatus::Finished(_, at) = task.status {
            unix_ts_millis = unix_ts(at).ok();
        }
        let event = Event {
            name: task.full_name(),
            id: task.id.as_u64(),
            parent_id: task.parent_id.map(|id| id.as_u64()),
            event_type: EventType::End,
            data: task
                .all_data()
                .map(|(k, v)| (k.to_string(), v.0.to_string()))
                .collect(),
            unix_ts_millis,
        };

        if let Ok(json) = serde_json::to_string(&event) {
            self.write(&json);
        }
    }

    fn task_progress(&self, _task: Arc<TaskInternal>) {}
}

fn unix_ts(ts: SystemTime) -> Result<u64> {
    let since_the_epoch = ts.duration_since(UNIX_EPOCH)?;
    Ok(since_the_epoch.as_millis().try_into()?)
}
