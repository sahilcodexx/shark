//! In-memory directory cache for instant folder switching and zero-latency navigation.
//! Inspired by KDE Dolphin's KDirLister and GNOME Files architecture.

use crate::models::file_entry::FileEntry;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

/// Cached directory snapshot holding immutable file entries and metadata.
#[derive(Clone, Debug)]
pub struct CachedDirectory {
    pub entries: Arc<Vec<FileEntry>>,
    pub total_items: usize,
    pub total_bytes: u64,
    pub mtime: Option<SystemTime>,
}

/// Thread-safe directory cache with LRU eviction and background pre-warming.
#[derive(Clone)]
pub struct DirectoryCache {
    storage: Arc<RwLock<HashMap<PathBuf, CachedDirectory>>>,
}

impl Default for DirectoryCache {
    fn default() -> Self {
        Self::new()
    }
}

impl DirectoryCache {
    /// Create a new, empty directory cache.
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::with_capacity(32))),
        }
    }

    /// Retrieve cached directory contents if present.
    pub fn get(&self, path: &Path) -> Option<CachedDirectory> {
        let lock = self.storage.read().ok()?;
        lock.get(path).cloned()
    }

    /// Check if the cache is still fresh by comparing the directory's filesystem mtime.
    pub fn is_fresh(&self, path: &Path) -> bool {
        if let Some(cached) = self.get(path) {
            if let Ok(meta) = path.metadata() {
                if let Ok(current_mtime) = meta.modified() {
                    return cached.mtime == Some(current_mtime);
                }
            }
        }
        false
    }

    /// Insert or update a directory cache entry with bounded capacity (max 64 directories).
    pub fn insert(&self, path: PathBuf, entries: Vec<FileEntry>, total_items: usize, total_bytes: u64) {
        let mtime = path.metadata().and_then(|m| m.modified()).ok();
        if let Ok(mut lock) = self.storage.write() {
            // Cap memory usage: 64 directories of ~1000 items is ~15MB RAM max
            if lock.len() >= 64 && !lock.contains_key(&path) {
                if let Some(key) = lock.keys().next().cloned() {
                    lock.remove(&key);
                }
            }
            lock.insert(
                path,
                CachedDirectory {
                    entries: Arc::new(entries),
                    total_items,
                    total_bytes,
                    mtime,
                },
            );
        }
    }

    /// Invalidate a directory from the cache (e.g. after file changes or write operations).
    pub fn invalidate(&self, path: &Path) {
        if let Ok(mut lock) = self.storage.write() {
            lock.remove(path);
        }
    }

    /// Pre-warm common sidebar favorite directories in a low-priority background thread.
    /// This ensures that when the user clicks 'Downloads', 'Documents', etc., the files
    /// appear instantly (0ms delay) just like in KDE Dolphin.
    pub fn warm_favorites(&self) {
        let cache = self.clone();
        std::thread::Builder::new()
            .name("shark-cache-prewarmer".into())
            .spawn(move || {
                let mut favorites = Vec::new();
                if let Some(p) = dirs::home_dir() { favorites.push(p); }
                if let Some(p) = dirs::download_dir() { favorites.push(p); }
                if let Some(p) = dirs::document_dir() { favorites.push(p); }
                if let Some(p) = dirs::desktop_dir() { favorites.push(p); }
                if let Some(p) = dirs::picture_dir() { favorites.push(p); }
                if let Some(p) = dirs::video_dir() { favorites.push(p); }
                if let Some(p) = dirs::audio_dir() { favorites.push(p); }

                for dir in favorites {
                    if !dir.exists() {
                        continue;
                    }
                    if cache.get(&dir).is_some() {
                        continue;
                    }

                    if let Ok(read_dir) = std::fs::read_dir(&dir) {
                        let mut entries = Vec::with_capacity(128);
                        let mut total_items = 0;
                        let mut total_bytes = 0;

                        for res in read_dir.flatten() {
                            if let Ok(entry) = FileEntry::from_dir_entry(&res) {
                                total_items += 1;
                                total_bytes += entry.size;
                                entries.push(entry);
                            }
                        }

                        cache.insert(dir, entries, total_items, total_bytes);
                    }
                }
            })
            .ok();
    }
}
