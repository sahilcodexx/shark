//! Internal clipboard state for copy/cut/paste file operations.

use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct AppClipboard {
    pub paths: Vec<PathBuf>,
    pub is_cut: bool,
}

impl AppClipboard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn copy(&mut self, paths: Vec<PathBuf>) {
        self.paths = paths;
        self.is_cut = false;
    }

    pub fn cut(&mut self, paths: Vec<PathBuf>) {
        self.paths = paths;
        self.is_cut = true;
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub fn clear(&mut self) {
        self.paths.clear();
        self.is_cut = false;
    }
}
