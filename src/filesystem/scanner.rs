//! Asynchronous, incremental directory scanner for the Shark file manager.

use crate::models::file_entry::FileEntry;
use gio::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

#[allow(dead_code)]
/// Messages sent from the background scanner to the UI thread.
#[derive(Debug)]
pub enum ScanMessage {
    /// A progressive batch of loaded file entries.
    Batch(Vec<FileEntry>),
    /// Scanning has finished successfully with totals.
    Completed {
        total_items: usize,
        total_bytes: u64,
    },
    /// Scanning encountered an error.
    Error(String),
}

const ATTRIBUTES: &str = "standard::name,standard::display-name,standard::type,\
standard::size,standard::content-type,standard::is-hidden,standard::is-symlink,\
time::modified,time::created,unix::mode,owner::user,owner::group";

const BATCH_SIZE: usize = 64;

/// Asynchronously scans a directory in a background worker thread.
/// Sends entries incrementally in batches so large directories load progressively.
pub fn scan_directory_async(
    dir: PathBuf,
    cancellable: Arc<AtomicBool>,
    sender: Sender<ScanMessage>,
) {
    std::thread::spawn(move || {
        if cancellable.load(Ordering::Relaxed) {
            return;
        }

        let gfile = gio::File::for_path(&dir);
        let gio_cancellable = gio::Cancellable::new();

        // Enumerate children using GIO
        let enumerator = match gfile.enumerate_children(
            ATTRIBUTES,
            gio::FileQueryInfoFlags::NONE,
            Some(&gio_cancellable),
        ) {
            Ok(e) => e,
            Err(e) => {
                // If GIO enumeration fails, try standard std::fs fallback
                if let Err(fallback_err) = scan_with_std_fs(&dir, &cancellable, &sender) {
                    let _ = sender.send(ScanMessage::Error(format!("{}: {}", e, fallback_err)));
                }
                return;
            }
        };

        let mut current_batch = Vec::with_capacity(BATCH_SIZE);
        let mut total_items = 0;
        let mut total_bytes = 0;

        while let Ok(Some(info)) = enumerator.next_file(Some(&gio_cancellable)) {
            if cancellable.load(Ordering::Relaxed) {
                return;
            }

            let name = info.name();
            let entry_path = dir.join(name);
            let entry = FileEntry::from_file_info(entry_path, &info);

            total_items += 1;
            total_bytes += entry.size;
            current_batch.push(entry);

            if current_batch.len() >= BATCH_SIZE {
                let batch = std::mem::replace(&mut current_batch, Vec::with_capacity(BATCH_SIZE));
                if sender.send(ScanMessage::Batch(batch)).is_err() {
                    return;
                }
            }
        }

        if !current_batch.is_empty()
            && sender.send(ScanMessage::Batch(current_batch)).is_err() {
                return;
            }

        let _ = sender.send(ScanMessage::Completed {
            total_items,
            total_bytes,
        });
    });
}

/// Fallback scanner using std::fs if GIO enumeration fails.
fn scan_with_std_fs(
    dir: &Path,
    cancellable: &Arc<AtomicBool>,
    sender: &Sender<ScanMessage>,
) -> std::io::Result<()> {
    let read_dir = std::fs::read_dir(dir)?;
    let mut current_batch = Vec::with_capacity(BATCH_SIZE);
    let mut total_items = 0;
    let mut total_bytes = 0;

    for res in read_dir {
        if cancellable.load(Ordering::Relaxed) {
            return Ok(());
        }

        if let Ok(fs_entry) = res
            && let Ok(entry) = FileEntry::from_path(fs_entry.path()) {
                total_items += 1;
                total_bytes += entry.size;
                current_batch.push(entry);

                if current_batch.len() >= BATCH_SIZE {
                    let batch = std::mem::replace(&mut current_batch, Vec::with_capacity(BATCH_SIZE));
                    if sender.send(ScanMessage::Batch(batch)).is_err() {
                        return Ok(());
                    }
                }
            }
    }

    if !current_batch.is_empty()
        && sender.send(ScanMessage::Batch(current_batch)).is_err() {
            return Ok(());
        }

    let _ = sender.send(ScanMessage::Completed {
        total_items,
        total_bytes,
    });

    Ok(())
}

/// Asynchronously and recursively searches files under a root directory matching a query.
pub fn recursive_search_async(
    root: PathBuf,
    query: String,
    cancellable: Arc<AtomicBool>,
    sender: Sender<ScanMessage>,
) {
    std::thread::spawn(move || {
        let query_lower = query.trim().to_lowercase();
        if query_lower.is_empty() {
            let _ = sender.send(ScanMessage::Completed { total_items: 0, total_bytes: 0 });
            return;
        }
        let mut current_batch = Vec::with_capacity(BATCH_SIZE);
        let mut total_items = 0;
        let mut total_bytes = 0;
        let mut last_batch = std::time::Instant::now();
        let mut stack = vec![root.clone()];
        while let Some(dir) = stack.pop() {
            if cancellable.load(Ordering::Relaxed) { return; }
            let read_dir = match std::fs::read_dir(&dir) {
                Ok(entries) => entries,
                Err(error) if dir == root => {
                    let _ = sender.send(ScanMessage::Error(format!("Cannot search {}: {}", dir.display(), error)));
                    return;
                }
                // A disappearing or inaccessible subfolder must not stop the search.
                Err(_) => continue,
            };
            for item in read_dir.flatten() {
                if cancellable.load(Ordering::Relaxed) { return; }
                let path = item.path();
                // DirEntry type does not follow symlinks, preventing traversal loops.
                if item.file_type().is_ok_and(|kind| kind.is_dir()) { stack.push(path.clone()); }
                if !item.file_name().to_string_lossy().to_lowercase().contains(&query_lower) { continue; }
                // Only matches need full metadata, not every file in the tree.
                let Ok(entry) = FileEntry::from_path(&path) else { continue; };
                total_items += 1;
                total_bytes += entry.size;
                current_batch.push(entry);
                if total_items == 1 || current_batch.len() >= BATCH_SIZE
                    || last_batch.elapsed() >= std::time::Duration::from_millis(80) {
                    let batch = std::mem::replace(&mut current_batch, Vec::with_capacity(BATCH_SIZE));
                    if sender.send(ScanMessage::Batch(batch)).is_err() { return; }
                    last_batch = std::time::Instant::now();
                }
            }
            if !current_batch.is_empty() && last_batch.elapsed() >= std::time::Duration::from_millis(80) {
                let batch = std::mem::replace(&mut current_batch, Vec::with_capacity(BATCH_SIZE));
                if sender.send(ScanMessage::Batch(batch)).is_err() { return; }
                last_batch = std::time::Instant::now();
            }
        }
        if !current_batch.is_empty() && sender.send(ScanMessage::Batch(current_batch)).is_err() { return; }
        let _ = sender.send(ScanMessage::Completed { total_items, total_bytes });
    });
}
