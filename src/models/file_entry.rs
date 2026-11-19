//! File entry data structure and metadata for the Shark file manager.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// File entry representing a file or directory with cached metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub name: String,
    pub name_lower: String,
    pub path: PathBuf,
    pub is_directory: bool,
    pub is_symlink: bool,
    pub is_hidden: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub created: Option<SystemTime>,
    pub content_type: &'static str,
    pub icon_name: &'static str,
    pub permissions: u32,
    pub owner: String,
    pub group: String,
}

impl FileEntry {
    /// Create a FileEntry directly from a native DirEntry using cached kernel d_type.
    pub fn from_dir_entry(entry: &std::fs::DirEntry) -> std::io::Result<Self> {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let is_hidden = name.starts_with('.');
        let name_lower = name.to_lowercase();

        let file_type = entry.file_type()?;
        let is_symlink = file_type.is_symlink();
        let is_directory = if is_symlink {
            entry.metadata().map(|m| m.is_dir()).unwrap_or(false)
        } else {
            file_type.is_dir()
        };

        let (size, modified, created, permissions) = if let Ok(meta) = entry.metadata() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                let sz = if is_directory { 0 } else { meta.len() };
                let mod_time = meta.modified().ok();
                let cr_time = meta.created().ok();
                (sz, mod_time, cr_time, meta.mode())
            }
            #[cfg(not(unix))]
            {
                let sz = if is_directory { 0 } else { meta.len() };
                let mod_time = meta.modified().ok();
                let cr_time = meta.created().ok();
                (sz, mod_time, cr_time, 0o644)
            }
        } else {
            (0, None, None, 0o644)
        };

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let content_type = if is_directory {
            "inode/directory"
        } else {
            Self::guess_content_type(ext)
        };

        let icon_name = Self::determine_icon(&content_type, is_directory, is_symlink, &name);

        Ok(Self {
            name,
            name_lower,
            path,
            is_directory,
            is_symlink,
            is_hidden,
            size,
            modified,
            created,
            content_type,
            icon_name,
            permissions,
            owner: String::new(),
            group: String::new(),
        })
    }

    /// Create a FileEntry from standard std::fs metadata.
    pub fn from_path<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let symlink_meta = std::fs::symlink_metadata(&path)?;
        let is_symlink = symlink_meta.file_type().is_symlink();

        let meta = std::fs::metadata(&path).unwrap_or_else(|_| symlink_meta.clone());
        let is_directory = meta.is_dir();

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string());

        let is_hidden = name.starts_with('.');
        let name_lower = name.to_lowercase();
        let size = if is_directory { 0 } else { meta.len() };
        let modified = meta.modified().ok();
        let created = meta.created().ok();

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let content_type = if is_directory {
            "inode/directory"
        } else {
            Self::guess_content_type(ext)
        };

        let icon_name = Self::determine_icon(&content_type, is_directory, is_symlink, &name);

        #[cfg(unix)]
        let (permissions, owner, group) = {
            use std::os::unix::fs::MetadataExt;
            (meta.mode(), meta.uid().to_string(), meta.gid().to_string())
        };
        #[cfg(not(unix))]
        let (permissions, owner, group) = (0, String::new(), String::new());

        Ok(Self {
            name,
            name_lower,
            path,
            is_directory,
            is_symlink,
            is_hidden,
            size,
            modified,
            created,
            content_type,
            icon_name,
            permissions,
            owner,
            group,
        })
    }

    /// Guess MIME content type from file extension as a zero-allocation static string.
    pub fn guess_content_type(ext: &str) -> &'static str {
        match ext.to_lowercase().as_str() {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "ico" | "bmp" => "image/x-icon",
            "mp4" | "mkv" | "webm" | "avi" | "mov" => "video/mp4",
            "mp3" | "flac" | "ogg" | "wav" | "m4a" => "audio/mpeg",
            "pdf" => "application/pdf",
            "txt" | "rst" => "text/plain",
            "log" => "text/x-log",
            "md" | "markdown" => "text/markdown",
            "csv" | "tsv" | "xls" | "xlsx" => "text/csv",
            "html" | "htm" => "text/html",
            "xml" | "netxml" | "kml" => "application/xml",
            "cap" | "pcap" | "pcapng" => "application/vnd.tcpdump.pcap",
            "rs" => "text/x-rust",
            "py" => "text/x-python",
            "c" | "h" => "text/x-c",
            "cpp" | "hpp" | "cc" => "text/x-c++",
            "js" | "mjs" | "ts" | "tsx" | "jsx" => "application/javascript",
            "css" | "scss" | "sass" | "less" => "text/css",
            "json" => "application/json",
            "toml" => "application/toml",
            "yaml" | "yml" => "application/yaml",
            "sql" | "sqlite" | "db" => "application/x-sqlite3",
            "zip" => "application/zip",
            "tar" | "gz" | "xz" | "bz2" | "zst" | "7z" | "rar" => "application/x-archive",
            "sh" | "bash" | "zsh" => "application/x-shellscript",
            "iso" => "application/x-cd-image",
            _ => "application/octet-stream",
        }
    }

    /// Determine SVG icon name in assets/icons/ for this entry.
    pub fn determine_icon(content_type: &str, is_dir: bool, _is_symlink: bool, filename: &str) -> &'static str {
        if is_dir {
            "folder"
        } else if content_type == "application/pdf" || filename.to_lowercase().ends_with(".pdf") {
            "pdf"
        } else if content_type.starts_with("image/") {
            "image"
        } else if content_type.starts_with("video/") {
            "video"
        } else if content_type.starts_with("audio/") {
            "audio"
        } else if content_type.contains("archive") || content_type.contains("zip") || content_type.contains("tar") {
            "archive"
        } else if content_type == "text/csv" || filename.to_lowercase().ends_with(".csv") || filename.to_lowercase().ends_with(".tsv") {
            "table"
        } else if content_type == "text/html" || filename.to_lowercase().ends_with(".html") || filename.to_lowercase().ends_with(".htm") {
            "globe"
        } else if content_type == "application/xml" || filename.to_lowercase().ends_with(".xml") || filename.to_lowercase().ends_with(".netxml") {
            "globe"
        } else if content_type == "application/vnd.tcpdump.pcap" || filename.to_lowercase().ends_with(".cap") || filename.to_lowercase().ends_with(".pcap") {
            "cpu"
        } else if content_type == "text/x-log" || filename.to_lowercase().ends_with(".log") {
            "terminal"
        } else if content_type == "application/x-sqlite3" || filename.to_lowercase().ends_with(".sql") || filename.to_lowercase().ends_with(".db") {
            "database"
        } else if content_type == "text/markdown" || filename.to_lowercase().ends_with(".md") {
            "book"
        } else if content_type.starts_with("text/x-") || content_type.contains("javascript") || content_type.contains("json") || content_type.contains("toml") || content_type.contains("yaml") {
            "code"
        } else if content_type.starts_with("text/") || filename.to_lowercase().ends_with(".txt") {
            "file-text"
        } else {
            "file"
        }
    }

    /// Color accent for file icon representation (ARGB hex).
    pub fn icon_color_hex(&self) -> u32 {
        if self.is_directory {
            0x38bdf8ff // Sky / Cyan
        } else {
            match self.icon_name {
                "table" => 0x34d399ff,     // Emerald green for CSV / TSV / Excel
                "globe" => 0xfb923cff,     // Orange for HTML / XML / Web
                "cpu" => 0x818cf8ff,       // Indigo for CAP / PCAP / Binaries
                "terminal" => 0x06b6d4ff,  // Cyan for Logs / Scripts
                "code" => 0x60a5faff,      // Blue for Rust / Py / JS / TS / JSON
                "book" => 0xa78bfaff,      // Purple for Markdown / Docs
                "file-text" => 0x94a3b8ff, // Slate gray for Text / Notes
                "pdf" => 0xf87171ff,       // Red for PDF
                "image" => 0xf472b6ff,     // Pink for Images
                "video" => 0xc084fcff,     // Purple for Videos
                "audio" => 0xe879f9ff,     // Fuchsia for Audio
                "archive" => 0xfbbf24ff,   // Amber for ZIP / TAR / GZ
                "database" => 0x2dd4bfff,  // Teal for SQL / DB
                _ => 0x94a3b8ff,
            }
        }
    }

    /// Short format badge for grid card display (e.g. CSV, HTML, TXT, LOG).
    pub fn format_badge(&self) -> Option<String> {
        if self.is_directory {
            return None;
        }
        if let Some(ext) = self.path.extension().and_then(|e| e.to_str()) {
            if ext.len() <= 5 {
                return Some(ext.to_uppercase());
            }
        }
        match self.icon_name {
            "table" => Some("CSV".to_string()),
            "globe" => Some("WEB".to_string()),
            "cpu" => Some("BIN".to_string()),
            "terminal" => Some("LOG".to_string()),
            "code" => Some("CODE".to_string()),
            "book" => Some("DOC".to_string()),
            "file-text" => Some("TXT".to_string()),
            "pdf" => Some("PDF".to_string()),
            "archive" => Some("ZIP".to_string()),
            _ => None,
        }
    }

    /// Human-readable file size.
    pub fn formatted_size(&self) -> String {
        if self.is_directory {
            "Folder".to_string()
        } else {
            crate::utils::format_size(self.size)
        }
    }

    /// Human-readable modified date.
    pub fn formatted_modified(&self) -> String {
        crate::utils::format_date(self.modified)
    }

    /// Human-readable Unix permissions string.
    pub fn formatted_permissions(&self) -> String {
        crate::utils::format_permissions(self.permissions, self.is_directory)
    }

    /// Human-readable type description.
    pub fn type_description(&self) -> String {
        crate::utils::get_content_type_description(&self.content_type, self.is_directory)
    }
}
