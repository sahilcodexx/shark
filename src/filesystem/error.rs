//! Filesystem error types for the Shark file manager.

use std::path::PathBuf;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum FilesystemError {
    NotFound(PathBuf),
    PermissionDenied(PathBuf),
    AlreadyExists(PathBuf),
    NotADirectory(PathBuf),
    Io(String),
    Cancelled,
}

impl std::fmt::Display for FilesystemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(p) => write!(f, "File or directory not found: {:?}", p),
            Self::PermissionDenied(p) => write!(f, "Permission denied: {:?}", p),
            Self::AlreadyExists(p) => write!(f, "Destination already exists: {:?}", p),
            Self::NotADirectory(p) => write!(f, "Not a directory: {:?}", p),
            Self::Io(msg) => write!(f, "I/O error: {}", msg),
            Self::Cancelled => write!(f, "Operation was cancelled"),
        }
    }
}

impl std::error::Error for FilesystemError {}

impl From<std::io::Error> for FilesystemError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => Self::NotFound(PathBuf::new()),
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied(PathBuf::new()),
            std::io::ErrorKind::AlreadyExists => Self::AlreadyExists(PathBuf::new()),
            _ => Self::Io(err.to_string()),
        }
    }
}

impl From<glib::Error> for FilesystemError {
    fn from(err: glib::Error) -> Self {
        Self::Io(err.to_string())
    }
}
