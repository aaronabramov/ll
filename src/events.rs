use crate::event_data::{DataValue, EventData};
use crate::level::Level;
use crate::types;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

pub struct OngoingEvent {
    pub(crate) event: Arc<Mutex<Event>>,
}

/// Public event that is passed to a logger call closure
/// `l.event("my_event", |ongoing_event| { ... });`
///
/// This struct has an API that can modify the underlying `Event` object, which is not
/// public.
impl OngoingEvent {
    /// Add a piece of data associated with the event.
    /// This data will be logged together with the event.
    /// ```
    /// let l = ll::Logger::new();
    /// l.event("some_event", |e| {
    ///     e.add_data("random_counter", 5);
    ///     // it also supports hashtag, which can be handled by provided Drain
    ///     // implementations during logging.
    ///     e.add_data("dont_print_this_data #dontprint", true);
    ///     Ok(())
    /// }).unwrap();
    /// ```
    pub fn add_data<K, V>(&self, key: K, value: V) -> &Self
    where
        K: Into<String>,
        V: Into<DataValue>,
    {
        self.event.lock().unwrap().data.add(key.into(), value);
        self
    }

    /// Sets an error message for the event as a separate field.
    /// ```
    /// let l = ll::Logger::new();
    /// l.event("some_event", |e| {
    ///     e.set_error_msg("It crashed!!!");
    ///     Ok(())
    /// }).unwrap();
    /// ```
    pub fn set_error_msg<S: Into<String>>(&self, msg: S) {
        let mut e = self.event.lock().expect("poisoned lock");
        e.error_msg = Some(msg.into());
        e.is_error = true;
    }

    /// Discard an event  manually. After calling this function will not be logged.
    pub fn discard(&self) {
        self.event.lock().unwrap().discarded = true;
    }
}

impl std::convert::From<Arc<Mutex<Event>>> for OngoingEvent {
    fn from(event: Arc<Mutex<Event>>) -> OngoingEvent {
        OngoingEvent { event }
    }
}

/// Underlying struct for the event. This struct is only interacted with publicly
/// after the logging is done. This struct is what gets passed to every `Drain`
/// implementation.
#[derive(Debug)]
pub struct Event {
    /// Name of the event, which is its main identifier
    pub name: String,

    /// Any data associated with the event in the shape of
    /// simple key/value pairs
    pub data: EventData,
    /// Whether this event has been logged or discarded already
    pub discarded: bool,
    /// how long it took to run the block of code that was
    /// wrapped into logger event call.
    pub duration: Option<Duration>,
    /// Error message that was a result of running a code block
    /// wrapped in a logging call. Must be set manually.
    pub error_msg: Option<String>,
    /// Whether it's an error or not.
    pub is_error: bool,
    /// Logging level of this event. If event is `Trace` and the logger is
    /// set up to log only `Info`, the event won't be logged
    pub level: Level,
    /// SystemTime of when the event was created
    pub started_at: SystemTime,
    /// Tags provided in the event name string
    /// for example
    /// l.event("my_event #dontprint", |_| { ... });
    /// will have tags as `BTreeSet<"dontprint">`
    pub tags: types::Tags,
}
