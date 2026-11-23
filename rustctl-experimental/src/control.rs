use std::sync::atomic::{AtomicBool, Ordering};

static BUSY: AtomicBool = AtomicBool::new(false);

pub(crate) fn is_busy() -> bool {
    BUSY.load(Ordering::SeqCst)
}

fn try_acquire() -> bool {
    BUSY.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
}

fn release() {
    BUSY.store(false, Ordering::SeqCst);
}

pub(crate) struct BusyGuard {
    _private: (),
}

impl BusyGuard {
    pub(crate) fn acquire() -> Option<Self> {
        try_acquire().then_some(BusyGuard { _private: () })
    }
}

impl Drop for BusyGuard {
    fn drop(&mut self) {
        release();
    }
}
