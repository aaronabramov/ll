#![allow(clippy::new_without_default)]

pub mod drains;
mod event_data;
mod events;
mod level;
mod logger;
mod types;

mod utils;

pub use drains::Drain;
pub use events::{Event, OngoingEvent};
pub use level::Level;
pub use logger::Logger;
