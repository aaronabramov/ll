use crate::drains::Drain;
use crate::event_data::{DataValue, EventData};
use crate::events::{Event, OngoingEvent};
use crate::level::Level;
use anyhow::{Context, Result};
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

#[derive(Clone)]
pub struct Logger {
    drains: Vec<Arc<dyn Drain>>,
    data: EventData,
    log_level: Level,
    event_name_prefix: Option<String>,
}

impl Logger {
    pub fn new() -> Self {
        Logger {
            drains: vec![],
            data: EventData::empty(),
            log_level: Level::Info,
            event_name_prefix: None,
        }
    }

    pub fn stdout() -> Self {
        let mut ll = Self::new();
        ll.add_drain(Arc::new(crate::drains::stdout::StdoutDrain::new()));
        ll
    }

    pub fn add_drain(&mut self, drain: Arc<dyn Drain>) {
        self.drains.push(drain);
    }

    pub fn set_log_level(&mut self, log_level: Level) {
        self.log_level = log_level;
    }

    pub fn set_event_name_prefix<S: Into<String>>(&mut self, prefix: S) {
        self.event_name_prefix = Some(prefix.into());
    }

    /// Add key/value paris data to the logger. Every event logged from this logger
    /// instance will have these k/v pairs associated with them.
    pub fn add_data<K: Into<String>, V: Into<DataValue>>(&mut self, key: K, value: V) {
        self.data.add(key.into(), value);
    }

    pub fn event<E: Into<String>, F, T>(&self, event_name: E, f: F) -> Result<T>
    where
        F: FnOnce(OngoingEvent) -> Result<T>,
    {
        let e = self.make_event(event_name.into());
        let result = f(e.clone().into());
        self.after_fn(result, e)
    }

    pub async fn async_event<E: Into<String>, FN, FT, T>(&self, event_name: E, f: FN) -> Result<T>
    where
        FN: FnOnce(OngoingEvent) -> FT,
        FT: Future<Output = Result<T>>,
    {
        let e = self.make_event(event_name.into());
        let result = f(e.clone().into()).await;
        self.after_fn(result, e)
    }

    fn make_event<E: Into<String>>(&self, event_name: E) -> Arc<Mutex<Event>> {
        let (mut name, tags) = crate::utils::extract_tags(event_name.into());
        if let Some(prefix) = &self.event_name_prefix {
            name = format!("{}:{}", prefix, &name);
        }
        let level = crate::utils::extract_log_level_from_tags(&tags).unwrap_or(Level::Info);
        Arc::new(Mutex::new(Event {
            name,
            data: EventData::empty(),
            discarded: false,
            duration: None,
            error_msg: None,
            is_error: false,
            level,
            started_at: SystemTime::now(),
            tags,
        }))
    }

    fn log(&self, event: &mut Event) {
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

    fn after_fn<T>(&self, mut result: Result<T>, e: Arc<Mutex<Event>>) -> Result<T> {
        let mut event = e.lock().expect("poisoned lock");
        event.duration = event.started_at.elapsed().ok();
        if result.is_err() {
            event.is_error = true;
        }
        if result.is_err() {
            let mut context = format!("[inside event] {}", &event.name);
            if !event.data.is_empty() {
                context.push_str(&format!("\n  {}", &event.data));
            }
            result = result.context(context)
        };

        self.log(&mut event);
        result
    }
}
