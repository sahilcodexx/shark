//! Filesystem directory change monitoring using GIO FileMonitor.

use gio::prelude::*;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct DirectoryMonitor {
    _monitor: gio::FileMonitor,
}

impl DirectoryMonitor {
    /// Start monitoring a directory for external file additions, deletions, or modifications.
    pub fn new<F: Fn() + 'static>(dir: &Path, on_change: F) -> Option<Self> {
        let gfile = gio::File::for_path(dir);
        let monitor = gfile
            .monitor_directory(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE)
            .ok()?;

        let on_change = Rc::new(on_change);
        let pending = Rc::new(RefCell::new(false));
        let created_at = Instant::now();

        monitor.connect_changed(move |_, file, _, _| {
            // Ignore events in the first second after monitor creation
            if created_at.elapsed() < Duration::from_millis(1500) {
                return;
            }

            // Ignore hidden dotfiles to avoid thrashing on .cache, .bash_history etc.
            if let Some(basename) = file.basename()
                && basename.to_string_lossy().starts_with('.') {
                    return;
                }

            if *pending.borrow() {
                return;
            }
            *pending.borrow_mut() = true;

            let on_change = on_change.clone();
            let pending = pending.clone();

            glib::timeout_add_local_once(Duration::from_millis(1000), move || {
                *pending.borrow_mut() = false;
                on_change();
            });
        });

        Some(Self { _monitor: monitor })
    }
}
