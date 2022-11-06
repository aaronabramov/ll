use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

lazy_static::lazy_static! {
    static ref INCREMENTAL_UNIQ_ID: AtomicU64 = AtomicU64::new(0);
}
#[derive(Clone, Copy, Hash, PartialOrd, PartialEq, Ord, Eq, Debug)]
pub struct UniqID(u64);

impl UniqID {
    pub fn new() -> Self {
        UniqID(INCREMENTAL_UNIQ_ID.fetch_add(1, Ordering::SeqCst))
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for UniqID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
