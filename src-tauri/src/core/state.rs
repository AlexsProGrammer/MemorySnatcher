use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessingStateSnapshot {
    pub is_paused: bool,
    pub is_stopped: bool,
}

static IS_PAUSED: AtomicBool = AtomicBool::new(false);
static IS_STOPPED: AtomicBool = AtomicBool::new(false);

pub fn snapshot() -> ProcessingStateSnapshot {
    ProcessingStateSnapshot {
        is_paused: IS_PAUSED.load(Ordering::SeqCst),
        is_stopped: IS_STOPPED.load(Ordering::SeqCst),
    }
}

pub fn set_paused(value: bool) {
    IS_PAUSED.store(value, Ordering::SeqCst);
}

pub fn set_stopped(value: bool) {
    IS_STOPPED.store(value, Ordering::SeqCst);
}

pub fn reset() {
    set_paused(false);
    set_stopped(false);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_reflects_atomic_updates() {
        reset();
        assert_eq!(
            snapshot(),
            ProcessingStateSnapshot {
                is_paused: false,
                is_stopped: false,
            }
        );

        set_paused(true);
        assert_eq!(
            snapshot(),
            ProcessingStateSnapshot {
                is_paused: true,
                is_stopped: false,
            }
        );

        set_stopped(true);
        assert_eq!(
            snapshot(),
            ProcessingStateSnapshot {
                is_paused: true,
                is_stopped: true,
            }
        );

        reset();
    }
}