use anyhow::Result;
use std::{
    collections::{BTreeMap, BTreeSet},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone)]
pub struct Trace {
    pub root_id: u64,
    pub tasks: BTreeMap<u64, TaskTrace>,
}

#[derive(Debug, Clone)]
pub struct TaskTrace {
    pub name: String,
    pub data: BTreeMap<String, String>,
    pub children: BTreeSet<u64>,
    pub start: SystemTime,
    pub finished_at: Option<SystemTime>,
}

impl Trace {
    pub fn to_chrome_trace(&self) -> Result<String> {
        fn unix_ts(ts: SystemTime) -> Result<u64> {
            let since_the_epoch = ts.duration_since(UNIX_EPOCH)?;
            Ok(since_the_epoch.as_secs())
        }

        #[derive(serde::Serialize)]
        struct ChromeTraceEvent {
            name: String,
            ph: &'static str,
            pid: u64,
            ts: u64,
            tid: u64,
            args: BTreeMap<String, String>,
        }

        let mut vec = vec![];

        let mut tid = 0;
        for (id, task) in &self.tasks {
            tid += 1;
            let event_name = format!("{}-{}", task.name.clone(), id);
            let beginning_event = ChromeTraceEvent {
                name: event_name.clone(),
                ph: "B",
                pid: 1,
                ts: unix_ts(task.start)?,
                tid,
                args: task.data.clone(),
            };
            vec.push(beginning_event);

            if let Some(finished_at) = task.finished_at {
                let end_event = ChromeTraceEvent {
                    name: event_name,
                    ph: "E",
                    pid: 1,
                    ts: unix_ts(finished_at)?,
                    tid,
                    args: task.data.clone(),
                };
                vec.push(end_event);
            }
        }

        Ok(serde_json::to_string_pretty(&vec)?)
    }
}
