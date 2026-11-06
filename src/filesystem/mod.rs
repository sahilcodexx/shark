//! Filesystem module for the Shark file manager.

pub mod error;
pub mod metadata;
pub mod monitor;
pub mod operations;
pub mod scanner;

pub use monitor::DirectoryMonitor;
pub use operations::{
    copy_paths_async, create_file, create_folder, delete_paths_permanent, get_unique_path,
    move_paths_async, rename_path, trash_paths, ConflictResolution, OpProgress,
};
pub use scanner::{recursive_search_async, scan_directory_async, ScanMessage};
