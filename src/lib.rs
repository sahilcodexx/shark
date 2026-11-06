//! Shark file manager library.

pub mod app;
pub mod config;
pub mod filesystem;
pub mod gui;
pub mod models;
pub mod navigation;
pub mod utils;

pub use app::SharkApp;
pub use config::Config;
pub use models::{FileEntry, FileObject};
pub use navigation::NavigationHistory;
