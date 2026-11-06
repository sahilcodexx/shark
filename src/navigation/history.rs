//! Navigation history tracking for the Shark file manager.

use std::path::{Path, PathBuf};

/// Manages directory navigation history (back, forward, parent).
#[derive(Debug, Clone)]
pub struct NavigationHistory {
    back_stack: Vec<PathBuf>,
    forward_stack: Vec<PathBuf>,
    current: PathBuf,
}

impl NavigationHistory {
    /// Create a new navigation history initialized with a starting path.
    pub fn new(initial: PathBuf) -> Self {
        Self {
            back_stack: Vec::new(),
            forward_stack: Vec::new(),
            current: initial,
        }
    }

    /// Get the current path.
    pub fn current(&self) -> &Path {
        &self.current
    }

    /// Navigate to a new directory.
    pub fn navigate_to(&mut self, new_path: PathBuf) {
        if new_path == self.current {
            return;
        }
        self.back_stack.push(self.current.clone());
        self.forward_stack.clear();
        self.current = new_path;
    }

    /// Check if back navigation is available.
    pub fn can_go_back(&self) -> bool {
        !self.back_stack.is_empty()
    }

    /// Check if forward navigation is available.
    pub fn can_go_forward(&self) -> bool {
        !self.forward_stack.is_empty()
    }

    /// Check if navigating to parent directory is possible.
    pub fn can_go_up(&self) -> bool {
        self.current.parent().is_some()
    }

    /// Navigate back in history.
    pub fn go_back(&mut self) -> Option<PathBuf> {
        let prev = self.back_stack.pop()?;
        self.forward_stack.push(self.current.clone());
        self.current = prev.clone();
        Some(prev)
    }

    /// Navigate forward in history.
    pub fn go_forward(&mut self) -> Option<PathBuf> {
        let next = self.forward_stack.pop()?;
        self.back_stack.push(self.current.clone());
        self.current = next.clone();
        Some(next)
    }

    /// Get parent directory of current path.
    pub fn parent(&self) -> Option<PathBuf> {
        self.current.parent().map(|p| p.to_path_buf())
    }
}
