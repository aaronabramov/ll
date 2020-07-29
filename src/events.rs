use crate::event_data::{DataValue, EventData};
use crate::level::Level;
use crate::types;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

pub struct OngoingEvent {
    pub(crate) event: Arc<Mutex<Event>>,
}

impl OngoingEvent {
    pub fn add_data<K, V>(&self, key: K, value: V) -> &Self
    where
        K: Into<String>,
        V: Into<DataValue>,
    {
        self.event.lock().unwrap().data.add(key.into(), value);
        self
    }
}

impl std::convert::From<Arc<Mutex<Event>>> for OngoingEvent {
    fn from(event: Arc<Mutex<Event>>) -> OngoingEvent {
        OngoingEvent { event }
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
    pub tags: types::Tags,
}
