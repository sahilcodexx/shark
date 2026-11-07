//! Configuration module for the Shark file manager.
//! Handles application settings and preferences persistence.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Application configuration settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// The last visited directory path
    pub last_directory: Option<PathBuf>,
    /// Show hidden files
    pub show_hidden: bool,
    /// View mode: "grid" or "list"
    pub view_mode: String,
    /// Sort column: "name", "size", "modified", "type"
    pub sort_by: String,
    /// Sort reversed (descending)
    pub sort_reversed: bool,
    /// User bookmarks
    pub bookmarks: Vec<PathBuf>,
    /// Window default width
    pub window_width: i32,
    /// Window default height
    pub window_height: i32,
    /// Background opacity mode: "solid" (plain simple background) or "glass" (translucent acrylic/glass)
    #[serde(default = "default_background_mode")]
    pub background_mode: String,
}

fn default_background_mode() -> String {
    "solid".to_string()
}

impl Default for Config {
    fn default() -> Self {
        let home = glib::home_dir();
        let default_bookmarks = vec![
            home.join("Documents"),
            home.join("Downloads"),
            home.join("Pictures"),
            home.join("Music"),
            home.join("Videos"),
        ]
        .into_iter()
        .filter(|p| p.exists())
        .collect();

        Self {
            last_directory: Some(home),
            show_hidden: false,
            view_mode: "grid".to_string(),
            sort_by: "name".to_string(),
            sort_reversed: false,
            bookmarks: default_bookmarks,
            window_width: 1024,
            window_height: 700,
            background_mode: "solid".to_string(),
        }
    }
}

impl Config {
    /// Get the configuration file path: ~/.config/shark/config.json
    fn config_path() -> PathBuf {
        let base = glib::user_config_dir();
        base.join("shark").join("config.json")
    }

    /// Load configuration from disk, or return default if missing or invalid.
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists()
            && let Ok(content) = std::fs::read_to_string(&path)
                && let Ok(config) = serde_json::from_str::<Config>(&content) {
                    return config;
                }
        Self::default()
    }

    /// Save configuration to disk.
    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }

    /// Get current starting directory (fallback to user home).
    pub fn current_directory(&self) -> PathBuf {
        if let Some(ref dir) = self.last_directory
            && dir.exists() && dir.is_dir() {
                return dir.clone();
            }
        glib::home_dir()
    }

    /// Add a path to bookmarks if not already present.
    pub fn add_bookmark<P: AsRef<Path>>(&mut self, path: P) {
        let path_buf = path.as_ref().to_path_buf();
        if !self.bookmarks.contains(&path_buf) {
            self.bookmarks.push(path_buf);
            self.save();
        }
    }

    /// Remove a path from bookmarks.
    pub fn remove_bookmark<P: AsRef<Path>>(&mut self, path: P) {
        let path_buf = path.as_ref();
        self.bookmarks.retain(|p| p != path_buf);
        self.save();
    }
}
