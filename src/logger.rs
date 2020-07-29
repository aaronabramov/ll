use crate::drains::Drain;
use crate::event_data::EventData;
use crate::events::{Event, OngoingEvent};
use crate::level::Level;
use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

#[derive(Clone)]
pub struct Logger {
    drains: Arc<RwLock<Vec<Box<dyn Drain>>>>,
    data: Arc<RwLock<EventData>>,
}

impl Logger {
    pub fn new() -> Self {
        Logger {
            drains: Arc::new(RwLock::new(vec![])),
            data: Arc::new(RwLock::new(EventData::empty())),
        }
    }

    pub fn add_drain(&self, drain: Box<dyn Drain>) {
        self.drains.write().expect("poisoned lock").push(drain);
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
        Event {
            name: event_name.into(),
            data: EventData::empty(),
            discarded: false,
            duration: None,
            error_msg: None,
            is_error: false,
            level: Level::Info,
            started_at: SystemTime::now(),
            tags: BTreeSet::new(),
        }
    }

    pub fn log(&self, event: Event) {
        let drains = self.drains.read().expect("poisoned lock");
        for drain in drains.iter() {
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
