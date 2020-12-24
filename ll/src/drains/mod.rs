pub mod stdout;

use crate::events::Event;

pub trait Drain {
    fn log_event(&self, e: &Event);
}
