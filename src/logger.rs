use crate::drains::Drain;
use crate::event_data::{DataValue, EventData};
use crate::events::{Event, OngoingEvent};
use crate::level::Level;
use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::SystemTime;

#[derive(Clone)]
pub struct Logger {
    drains: Vec<Arc<dyn Drain>>,
    data: EventData,
    log_level: Level,
}

impl Logger {
    pub fn new() -> Self {
        Logger {
            drains: vec![],
            data: EventData::empty(),
            log_level: Level::Info,
        }
    }

    pub fn add_drain(&mut self, drain: Arc<dyn Drain>) {
        self.drains.push(drain);
    }

    pub fn set_log_level(&mut self, log_level: Level) {
        self.log_level = log_level;
    }

    pub fn add_data<K: Into<String>, V: Into<DataValue>>(&mut self, key: K, value: V) {
        self.data.add(key.into(), value);
    }

    pub fn with_event<E: Into<String>, F, T>(&self, event_name: E, f: F) -> Result<T>
    where
        F: FnOnce(&OngoingEvent) -> Result<T>,
    {
        let e = self.event(event_name.into()).into();

        let result = f(&e);
        let mut event = e.event.into_inner().expect("poisoned mutex");
        event.duration = event.started_at.elapsed().ok();
        if result.is_err() {
            event.is_error = true;
        }
        let result = add_context(result, &event);
        self.log(event);
        result
    }

    pub fn event<E: Into<String>>(&self, event_name: E) -> Event {
        let (name, tags) = crate::utils::extract_tags(event_name.into());
        let level = crate::utils::extract_log_level_from_tags(&tags).unwrap_or(Level::Info);
        Event {
            name,
            data: EventData::empty(),
            discarded: false,
            duration: None,
            error_msg: None,
            is_error: false,
            level,
            started_at: SystemTime::now(),
            tags,
        }
    }

    pub fn log(&self, mut event: Event) {
        if event.discarded {
            return;
        }
        event.discarded = true;

        if event.level > self.log_level {
            return;
        }

        event.data.merge(&self.data);
        event.data.filter_for_level(self.log_level);

        for drain in self.drains.iter() {
            drain.log_event(&event);
        }
    }
}

fn add_context<T>(result: Result<T>, event: &Event) -> Result<T> {
    if result.is_err() {
        let mut context = format!("[inside event] {}", &event.name);
        if !event.data.is_empty() {
            context.push_str(&format!("\n  {}", &event.data));
        }
        result.context(context)
    } else {
        result
    }
}
