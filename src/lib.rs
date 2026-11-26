//! Shark file manager library - High-performance GPU-accelerated file manager.

pub mod app;
mod appimage_icon;
pub mod assets;
pub mod config;
pub mod filesystem;
pub mod models;
pub mod navigation;
pub mod platform;
pub mod theme;
pub mod thumbnails;
pub mod ui;
pub mod utils;

pub use app::SharkApp;
pub use config::Config;
pub use models::FileEntry;
pub use navigation::NavigationHistory;
