use std::sync::atomic::{AtomicU64, Ordering};

lazy_static::lazy_static! {
    static ref INCREMENTAL_UNIQ_ID: AtomicU64 = AtomicU64::new(0);
}
#[derive(Clone, Copy, Hash, PartialOrd, PartialEq, Ord, Eq)]
pub struct UniqID(u64);

impl UniqID {
    pub fn new() -> Self {
        UniqID(INCREMENTAL_UNIQ_ID.fetch_add(1, Ordering::SeqCst))
    }
}
