#![allow(clippy::new_without_default)]

pub mod drains;
mod level;

use crate::drains::Drain;
use anyhow::Result;
use level::Level;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime};

pub type Tags = BTreeSet<String>;

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
        let mut event = e.event.lock().expect("poisoned lock");
        event.duration = event.started_at.elapsed().ok();
        drop(event);
        self.log(e);
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

    pub fn log(&self, e: OngoingEvent) {
        let event = e.event.into_inner().expect("poisoned mutex");

        let drains = self.drains.read().expect("poisoned lock");
        for drain in drains.iter() {
            drain.log_event(&event);
        }
    }
}

pub struct OngoingEvent {
    event: Mutex<Event>,
}

impl std::convert::From<Event> for OngoingEvent {
    fn from(e: Event) -> OngoingEvent {
        OngoingEvent {
            event: Mutex::new(e),
        }
    }
}

#[derive(Debug)]
pub struct Event {
    pub name: String,

    pub data: EventData,
    pub discarded: bool,
    pub duration: Option<Duration>,
    pub error_msg: Option<String>,
    pub is_error: bool,
    pub level: Level,
    pub started_at: SystemTime,
    pub tags: Tags,
}

#[derive(Debug, Clone)]
pub enum DataValue {
    String(String),
    Int(i64),
    Float(f64),
    None,
}

impl std::fmt::Display for DataValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let result = match self {
            DataValue::String(string) => string.to_owned(),
            DataValue::Int(i) => format!("{}", i),
            DataValue::Float(f) => format!("{}", f),
            DataValue::None => format!(""),
        };
        write!(f, "{}", result)
    }
}

#[derive(Debug, Clone)]
pub struct DataEntry(pub DataValue, pub BTreeSet<String>);

#[derive(Debug, Clone)]
pub struct EventData {
    pub map: BTreeMap<String, DataEntry>,
}

impl EventData {
    pub fn empty() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }
}
