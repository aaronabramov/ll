mod drains;

use crate::drains::Drain;
use std::time::SystemTime;

use anyhow::Result;
pub struct Logger {
    drains: Vec<Box<dyn Drain>>,
}

impl Logger {
    pub fn with_event<E: Into<String>, F, T>(event_name: E, f: F) -> Result<T>
    where
        F: FnOnce(Event) -> Result<T>,
    {
        let e = Event::new(event_name.into());
        f(e)
    }
}

pub struct Event {
    name: String,
    timestamp: SystemTime,
}

impl Event {
    fn new(event_name: String) -> Self {
        Self {
            name: event_name,
            timestamp: SystemTime::now(),
        }
    }
}
