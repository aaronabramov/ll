pub mod stdout;

use crate::Event;

pub trait Drain {
    fn log_event(&self, e: &Event);
}
