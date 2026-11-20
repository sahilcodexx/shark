//! Fast background thumbnail provider for GPUI Shark.
//! Uses Freedesktop thumbnail specification standards with zero-allocation memory lookup caching.

use crate::utils::{find_cached_thumbnail_path, generate_thumbnail_file, is_previewable_media};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

pub struct ThumbnailCache {
    queued: Arc<Mutex<HashSet<PathBuf>>>,
    sender: async_channel::Sender<(PathBuf, String, u64)>,
    lookup_cache: Arc<RwLock<HashMap<PathBuf, Option<PathBuf>>>>,
    ready_sender: async_channel::Sender<PathBuf>,
    ready_receiver: async_channel::Receiver<PathBuf>,
}

impl Default for ThumbnailCache {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ThumbnailCache {
    fn clone(&self) -> Self {
        Self {
            queued: self.queued.clone(),
            sender: self.sender.clone(),
            lookup_cache: self.lookup_cache.clone(),
            ready_sender: self.ready_sender.clone(),
            ready_receiver: self.ready_receiver.clone(),
        }
    }
}

impl ThumbnailCache {
    pub fn new() -> Self {
        let (sender, receiver) = async_channel::bounded::<(PathBuf, String, u64)>(256);
        let (ready_sender, ready_receiver) = async_channel::bounded::<PathBuf>(256);
        let queued = Arc::new(Mutex::new(HashSet::new()));
        let lookup_cache = Arc::new(RwLock::new(HashMap::with_capacity(512)));

        // Spawn 4 background workers with low priority for smooth parallel thumbnail performance
        for _ in 0..4 {
            let rx = receiver.clone();
            let lookup = lookup_cache.clone();
            let ready_tx = ready_sender.clone();

            std::thread::Builder::new()
                .name("shark-thumb-worker".into())
                .spawn(move || {
                    while let Ok((path, content_type, size)) = rx.recv_blocking() {
                        if let Some(target_thumb) = generate_thumbnail_file(&path, &content_type, size) {
                            if let Ok(mut lock) = lookup.write() {
                                lock.insert(path.clone(), Some(target_thumb));
                            }
                            let _ = ready_tx.try_send(path);
                        }
                    }
                })
                .ok();
        }

        Self {
            queued,
            sender,
            lookup_cache,
            ready_sender,
            ready_receiver,
        }
    }

    /// Access the receiver channel to be notified when new thumbnails finish generating.
    pub fn ready_receiver(&self) -> async_channel::Receiver<PathBuf> {
        self.ready_receiver.clone()
    }

    /// Resolve an image source for an entry. Returns Some(PathBuf) if a cached
    /// thumbnail (or native vector SVG) is ready to display.
    /// Never returns raw multi-megabyte image paths directly to prevent RAM spikes.
    pub fn get_thumbnail_path(&self, path: &Path, content_type: &str, size: u64) -> Option<PathBuf> {
        // Fast-path: Check in-memory lookup cache first (<10ns, 0 filesystem calls)
        if let Ok(lock) = self.lookup_cache.read() {
            if let Some(cached) = lock.get(path) {
                return cached.clone();
            }
        }

        // Slow-path: Check disk once and cache the result
        // 1. Direct vector SVGs are lightweight and sharp
        if path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("svg")) {
            let res = Some(path.to_path_buf());
            if let Ok(mut lock) = self.lookup_cache.write() {
                lock.insert(path.to_path_buf(), res.clone());
            }
            return res;
        }

        // 2. Check existing Freedesktop thumbnail cache
        if let Some(thumb) = find_cached_thumbnail_path(path) {
            let res = Some(thumb);
            if let Ok(mut lock) = self.lookup_cache.write() {
                lock.insert(path.to_path_buf(), res.clone());
            }
            return res;
        }

        // 3. Mark as None in lookup cache so we don't repeat filesystem stat calls on future frames
        if let Ok(mut lock) = self.lookup_cache.write() {
            lock.insert(path.to_path_buf(), None);
        }

        // 4. For previewable media (images/videos) under 100MB, enqueue background thumbnail generation
        if is_previewable_media(content_type, path) && size <= 100 * 1024 * 1024 {
            let mut queued = self.queued.lock().unwrap();
            if queued.insert(path.to_path_buf()) {
                let _ = self.sender.try_send((path.to_path_buf(), content_type.to_string(), size));
            }
        }

        None
    }
}
