//! File operations (copy, move, trash, delete, rename, create) for Shark.

use gio::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

/// How to resolve filename collision at destination.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    Skip,
    Replace,
    KeepBoth,
    Cancel,
}

#[allow(dead_code)]
/// Operation progress notification.
#[derive(Debug, Clone)]
pub enum OpProgress {
    Progress {
        items_done: usize,
        total_items: usize,
        bytes_done: u64,
        total_bytes: u64,
        current_item: String,
    },
    Finished {
        success_count: usize,
        error_count: usize,
    },
    Error(String),
}

/// Generate a unique destination path if a conflict exists (e.g. "report (1).pdf").
pub fn get_unique_path(parent: &Path, name: &str) -> PathBuf {
    let target = parent.join(name);
    if !target.exists() {
        return target;
    }

    let p = Path::new(name);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or(name);
    let ext = p.extension().and_then(|e| e.to_str());

    let mut counter = 1;
    loop {
        let new_name = match ext {
            Some(e) => format!("{} ({}).{}", stem, counter, e),
            None => format!("{} ({})", stem, counter),
        };
        let candidate = parent.join(new_name);
        if !candidate.exists() {
            return candidate;
        }
        counter += 1;
    }
}

/// Move files or directories to system Trash using GIO.
pub fn trash_paths(paths: &[PathBuf]) -> Result<usize, String> {
    let mut trashed = 0;
    for path in paths {
        let gfile = gio::File::for_path(path);
        if let Err(e) = gfile.trash(gio::Cancellable::NONE) {
            return Err(format!("Failed to trash {:?}: {}", path, e));
        }
        trashed += 1;
    }
    Ok(trashed)
}

/// Permanently delete files or directories without moving to trash.
pub fn delete_paths_permanent(paths: &[PathBuf]) -> Result<usize, String> {
    let mut deleted = 0;
    for path in paths {
        if path.is_dir() && !path.is_symlink() {
            std::fs::remove_dir_all(path)
                .map_err(|e| format!("Failed to delete directory {:?}: {}", path, e))?;
        } else {
            std::fs::remove_file(path)
                .map_err(|e| format!("Failed to delete file {:?}: {}", path, e))?;
        }
        deleted += 1;
    }
    Ok(deleted)
}

/// Rename a file or directory.
pub fn rename_path(src: &Path, new_name: &str) -> Result<PathBuf, String> {
    if new_name.trim().is_empty() {
        return Err("Filename cannot be empty".to_string());
    }
    if new_name.contains('/') || new_name.contains('\0') {
        return Err("Filename contains invalid characters".to_string());
    }

    let parent = src.parent().ok_or_else(|| "Source has no parent".to_string())?;
    let dst = parent.join(new_name);

    if dst.exists() && dst != src {
        return Err(format!("An item named '{}' already exists", new_name));
    }

    std::fs::rename(src, &dst).map_err(|e| format!("Rename failed: {}", e))?;
    Ok(dst)
}

/// Create a new folder.
pub fn create_folder(parent: &Path, name: &str) -> Result<PathBuf, String> {
    if name.trim().is_empty() {
        return Err("Folder name cannot be empty".to_string());
    }
    if name.contains('/') || name.contains('\0') {
        return Err("Folder name contains invalid characters".to_string());
    }

    let target = parent.join(name);
    if target.exists() {
        return Err(format!("A folder named '{}' already exists", name));
    }

    std::fs::create_dir_all(&target).map_err(|e| format!("Failed to create folder: {}", e))?;
    Ok(target)
}

/// Create a new empty file.
pub fn create_file(parent: &Path, name: &str) -> Result<PathBuf, String> {
    if name.trim().is_empty() {
        return Err("File name cannot be empty".to_string());
    }
    if name.contains('/') || name.contains('\0') {
        return Err("File name contains invalid characters".to_string());
    }

    let target = parent.join(name);
    if target.exists() {
        return Err(format!("A file named '{}' already exists", name));
    }

    std::fs::File::create(&target).map_err(|e| format!("Failed to create file: {}", e))?;
    Ok(target)
}

/// Asynchronously copy a set of paths to a target directory.
pub fn copy_paths_async(
    sources: Vec<PathBuf>,
    destination_dir: PathBuf,
    resolution: ConflictResolution,
    cancellable: Arc<AtomicBool>,
    sender: Sender<OpProgress>,
) {
    std::thread::spawn(move || {
        let total_items = sources.len();
        let mut items_done = 0;
        let mut bytes_done = 0u64;
        let mut total_bytes = 0u64;

        for src in &sources {
            if let Ok(m) = std::fs::metadata(src) {
                total_bytes += m.len();
            }
        }

        let mut success_count = 0;
        let mut error_count = 0;

        for src in sources {
            if cancellable.load(Ordering::Relaxed) {
                let _ = sender.send(OpProgress::Error("Operation cancelled".to_string()));
                return;
            }

            let Some(file_name) = src.file_name().and_then(|n| n.to_str()) else {
                error_count += 1;
                continue;
            };

            let mut target = destination_dir.join(file_name);

            if target.exists() {
                match resolution {
                    ConflictResolution::Skip => {
                        items_done += 1;
                        continue;
                    }
                    ConflictResolution::Cancel => {
                        let _ = sender.send(OpProgress::Error("Operation cancelled by user".to_string()));
                        return;
                    }
                    ConflictResolution::KeepBoth => {
                        target = get_unique_path(&destination_dir, file_name);
                    }
                    ConflictResolution::Replace => {
                        if target.is_dir() && !target.is_symlink() {
                            let _ = std::fs::remove_dir_all(&target);
                        } else {
                            let _ = std::fs::remove_file(&target);
                        }
                    }
                }
            }

            let res = if src.is_dir() && !src.is_symlink() {
                copy_dir_recursive(&src, &target, &cancellable)
            } else {
                std::fs::copy(&src, &target).map(|b| {
                    bytes_done += b;
                })
            };

            match res {
                Ok(_) => {
                    success_count += 1;
                }
                Err(e) => {
                    error_count += 1;
                    let _ = sender.send(OpProgress::Error(format!("Failed to copy {:?}: {}", src, e)));
                }
            }

            items_done += 1;
            let _ = sender.send(OpProgress::Progress {
                items_done,
                total_items,
                bytes_done,
                total_bytes,
                current_item: file_name.to_string(),
            });
        }

        let _ = sender.send(OpProgress::Finished {
            success_count,
            error_count,
        });
    });
}

/// Asynchronously move a set of paths to a target directory.
pub fn move_paths_async(
    sources: Vec<PathBuf>,
    destination_dir: PathBuf,
    resolution: ConflictResolution,
    cancellable: Arc<AtomicBool>,
    sender: Sender<OpProgress>,
) {
    std::thread::spawn(move || {
        let total_items = sources.len();
        let mut items_done = 0;
        let mut success_count = 0;
        let mut error_count = 0;

        for src in sources {
            if cancellable.load(Ordering::Relaxed) {
                let _ = sender.send(OpProgress::Error("Operation cancelled".to_string()));
                return;
            }

            let Some(file_name) = src.file_name().and_then(|n| n.to_str()) else {
                error_count += 1;
                continue;
            };

            let mut target = destination_dir.join(file_name);

            if target.exists() {
                match resolution {
                    ConflictResolution::Skip => {
                        items_done += 1;
                        continue;
                    }
                    ConflictResolution::Cancel => {
                        let _ = sender.send(OpProgress::Error("Operation cancelled by user".to_string()));
                        return;
                    }
                    ConflictResolution::KeepBoth => {
                        target = get_unique_path(&destination_dir, file_name);
                    }
                    ConflictResolution::Replace => {
                        if target.is_dir() && !target.is_symlink() {
                            let _ = std::fs::remove_dir_all(&target);
                        } else {
                            let _ = std::fs::remove_file(&target);
                        }
                    }
                }
            }

            // Attempt native rename move first
            let res: std::io::Result<()> = std::fs::rename(&src, &target).or_else(|_| {
                // Cross-filesystem fallback: copy and remove source
                if src.is_dir() && !src.is_symlink() {
                    copy_dir_recursive(&src, &target, &cancellable)?;
                    std::fs::remove_dir_all(&src)?;
                } else {
                    std::fs::copy(&src, &target)?;
                    std::fs::remove_file(&src)?;
                }
                Ok(())
            });

            match res {
                Ok(_) => {
                    success_count += 1;
                }
                Err(e) => {
                    error_count += 1;
                    let _ = sender.send(OpProgress::Error(format!("Failed to move {:?}: {}", src, e)));
                }
            }

            items_done += 1;
            let _ = sender.send(OpProgress::Progress {
                items_done,
                total_items,
                bytes_done: 0,
                total_bytes: 0,
                current_item: file_name.to_string(),
            });
        }

        let _ = sender.send(OpProgress::Finished {
            success_count,
            error_count,
        });
    });
}

/// Recursively copy directory tree.
fn copy_dir_recursive(
    src: &Path,
    dst: &Path,
    cancellable: &Arc<AtomicBool>,
) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        if cancellable.load(Ordering::Relaxed) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "Cancelled",
            ));
        }
        let entry = entry?;
        let entry_path = entry.path();
        let target_path = dst.join(entry.file_name());

        if entry_path.is_dir() && !entry_path.is_symlink() {
            copy_dir_recursive(&entry_path, &target_path, cancellable)?;
        } else {
            std::fs::copy(&entry_path, &target_path)?;
        }
    }
    Ok(())
}
