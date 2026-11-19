//! Filesystem directory change monitoring.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub struct DirectoryMonitor {
    cancelled: Arc<AtomicBool>,
}

impl DirectoryMonitor {
    /// Start monitoring a directory for external file additions, deletions, or modifications.
    pub fn new<F: Fn() + Send + Sync + 'static>(dir: &Path, on_change: F) -> Option<Self> {
        let dir = dir.to_path_buf();
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancel_clone = cancelled.clone();

        std::thread::spawn(move || {
            let mut last_mod = std::fs::metadata(&dir).and_then(|m| m.modified()).ok();
            while !cancel_clone.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(1500));
                if cancel_clone.load(Ordering::Relaxed) {
                    break;
                }
                if let Ok(curr_mod) = std::fs::metadata(&dir).and_then(|m| m.modified()) {
                    if last_mod != Some(curr_mod) {
                        last_mod = Some(curr_mod);
                        on_change();
                    }
                }
            }
        });

        Some(Self { cancelled })
    }
}

impl Drop for DirectoryMonitor {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}
