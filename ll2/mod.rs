mod logger;
mod uniq_id;

use anyhow::{Context, Result};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::future::Future;
use std::sync::RwLock;
use uniq_id::UniqID;

struct LoggerData {}

struct Logger(UniqID);

struct Event(UniqID);

// fn with_graph_write<F>(f: F) -> Result<()>
// where
//     F: FnOnce(&mut Graph) -> Result<()>,
// {
//     f(&mut GRAPH.write().expect("poisoned lock")).context("Failed while holding GRAPH lock")
// }

lazy_static! {
    static ref LOGGER_GRAPH: RwLock<HashMap<UniqID, LoggerData>> = RwLock::new(HashMap::new());

    // static ref GRAPH: RwLock<Graph> = RwLock::new(HashMap::new());
}

// #[derive(Debug)]
// enum Node {
//     Logger,
//     Event,
// }
// #[derive(Clone, Copy, Debug)]
// pub struct Logger(u64);

// #[derive(Clone, Copy, Debug)]
// pub struct Event(u64);

// impl Logger {
//     fn new() -> Result<Self> {
//         let id = get_uniq_id();
//         with_graph_write(|graph| {
//             //
//             let existing = graph.insert(id, Node::Logger);
//             if let Some(existing) = existing {
//                 anyhow::bail!("Can't insert a new logger into a graph because something with this ID already existts {:?}", existing);
//             }
//             Ok(())
//         })
//         .context("failed to create logger")?;
//         Ok(Logger(id))
//     }

//     pub fn event<F, T>(&self, f: F) -> Result<T>
//     where
//         F: FnOnce(Event) -> Result<T>,
//     {
//         f(Event::new()?)
//     }

//     pub async fn async_event<FN, FT, T>(&self, f: FN) -> Result<T>
//     where
//         FN: FnOnce(Event) -> FT,
//         FT: Future<Output = Result<T>>,
//     {
//         f(Event::new()?).await
//     }
// }

// impl Event {
//     fn new() -> Result<Self> {
//         let id = get_uniq_id();
//         with_graph_write(|graph| {
//             //
//             let existing = graph.insert(id, Node::Event);
//             if let Some(existing) = existing {
//                 anyhow::bail!("Can't insert a new logger into a graph because something with this ID already existts {:?}", existing);
//             }
//             Ok(())
//         })
//         .context("failed to create event")?;
//         Ok(Event(id))
//     }
// }

// pub fn create_logger() -> Result<Logger> {
//     Logger::new()
// }
