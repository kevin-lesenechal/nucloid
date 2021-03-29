use core::sync::atomic::{AtomicU32, Ordering};

static CRITICAL_REGION_DEPTH: AtomicU32 = AtomicU32::new(0);

pub fn push_critical_region() {
    CRITICAL_REGION_DEPTH.fetch_add(1, Ordering::SeqCst);
}

pub fn pop_critical_region() {
    CRITICAL_REGION_DEPTH.fetch_sub(1, Ordering::SeqCst);
}
