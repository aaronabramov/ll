use rand::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};

lazy_static::lazy_static! {
    static ref INCREMENTAL_UNIQ_ID: AtomicU64 = AtomicU64::new(0);
    static ref RANDOM_UNIQ_ID: u64 = {
        let mut rng = rand::thread_rng();
        rng.gen()
    };



    // static ref GRAPH: RwLock<Graph> = RwLock::new(HashMap::new());
}
#[derive(Clone, Copy, Hash, PartialOrd, PartialEq, Ord, Eq)]
pub struct UniqID(u64, u64);

impl UniqID {
    pub fn new() -> Self {
        UniqID(
            *RANDOM_UNIQ_ID,
            INCREMENTAL_UNIQ_ID.fetch_add(1, Ordering::SeqCst),
        )
    }
}
