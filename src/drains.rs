use crate::Event;
use chrono::prelude::*;

pub trait Drain {
    fn log_event(&self, e: Event);
}

struct StdoutDrain {}

impl Drain for StdoutDrain {
    fn log_event(&self, e: Event) {
        let datetime: DateTime<Local> = e.timestamp.clone().into();
        let rounded = datetime.round_subsecs(0);
        let formatted = rounded.format("%I:%M:%S%p");

        println!("[{}] {}", formatted, &e.name);
    }
}
