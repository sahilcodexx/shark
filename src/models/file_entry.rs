//! File entry data structure and metadata for the Shark file manager.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// File entry representing a file or directory with cached metadata.
#[derive(Debug, Clone)]
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
    pub content_type: String,
    pub icon_name: String,
    pub permissions: u32,
    pub owner: String,
    pub group: String,
}

impl FileEntry {
    /// Create a FileEntry from GIO FileInfo.
    pub fn from_file_info(path: PathBuf, info: &gio::FileInfo) -> Self {
        let name = info.display_name().to_string();
        let file_type = info.file_type();
        let is_directory = file_type == gio::FileType::Directory;
        let is_symlink = info.is_symlink();
        let is_hidden = info.is_hidden() || name.starts_with('.');
        let size = if is_directory { 0 } else { info.size() as u64 };

        let modified = info.modification_date_time().map(|dt| {
            let unix = dt.to_unix();
            if unix >= 0 {
                SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(unix as u64)
            } else {
                SystemTime::UNIX_EPOCH
            }
        });

        let created_secs = info.attribute_uint64(gio::FILE_ATTRIBUTE_TIME_CREATED);
        let created = if created_secs > 0 {
            Some(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(created_secs))
        } else {
            None
        };

        let content_type = info.content_type().map(|c| c.to_string()).unwrap_or_else(|| {
            if is_directory {
                "inode/directory".to_string()
            } else {
                "application/octet-stream".to_string()
            }
        });

        let name_lower = name.to_lowercase();
        let icon_name = Self::determine_icon(&content_type, is_directory, is_symlink, &name);

        let permissions = info.attribute_uint32(gio::FILE_ATTRIBUTE_UNIX_MODE);
        let owner = info.attribute_string(gio::FILE_ATTRIBUTE_OWNER_USER).unwrap_or_default().to_string();
        let group = info.attribute_string(gio::FILE_ATTRIBUTE_OWNER_GROUP).unwrap_or_default().to_string();

        Self {
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
        }
    }

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
            "inode/directory".to_string()
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

        let name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string());

        let is_hidden = name.starts_with('.');
        let name_lower = name.to_lowercase();
        let size = if is_directory { 0 } else { meta.len() };
        let modified = meta.modified().ok();
        let created = meta.created().ok();

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let content_type = if is_directory {
            "inode/directory".to_string()
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

    /// Guess MIME content type from file extension.
    pub fn guess_content_type(ext: &str) -> String {
        match ext.to_lowercase().as_str() {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "mp4" | "mkv" | "webm" | "avi" | "mov" => "video/mp4",
            "mp3" | "flac" | "ogg" | "wav" | "m4a" => "audio/mpeg",
            "pdf" => "application/pdf",
            "txt" | "md" | "rst" | "log" => "text/plain",
            "rs" => "text/x-rust",
            "py" => "text/x-python",
            "c" | "h" => "text/x-c",
            "cpp" | "hpp" | "cc" => "text/x-c++",
            "js" | "mjs" | "ts" => "application/javascript",
            "html" | "htm" => "text/html",
            "css" => "text/css",
            "json" => "application/json",
            "toml" => "application/toml",
            "yaml" | "yml" => "application/yaml",
            "xml" => "application/xml",
            "zip" => "application/zip",
            "tar" | "gz" | "xz" | "bz2" | "zst" | "7z" => "application/x-archive",
            "sh" | "bash" | "zsh" => "application/x-shellscript",
            "iso" => "application/x-cd-image",
            _ => "application/octet-stream",
        }.to_string()
    }

    /// Determine icon name for this entry.
    pub fn determine_icon(content_type: &str, is_dir: bool, _is_symlink: bool, filename: &str) -> String {
        if is_dir {
            // Check special folder names
            match filename.to_lowercase().as_str() {
                "desktop" => "user-desktop",
                "documents" => "folder-documents",
                "downloads" => "folder-download",
                "music" => "folder-music",
                "pictures" => "folder-pictures",
                "videos" => "folder-videos",
                _ => "folder",
            }.to_string()
        } else if content_type.starts_with("image/") {
            "image-x-generic".to_string()
        } else if content_type.starts_with("video/") {
            "video-x-generic".to_string()
        } else if content_type.starts_with("audio/") {
            "audio-x-generic".to_string()
        } else if content_type == "application/pdf" {
            "application-pdf".to_string()
        } else if content_type.contains("archive") || content_type.contains("zip") || content_type.contains("tar") {
            "package-x-generic".to_string()
        } else if content_type.starts_with("text/x-") || content_type.contains("javascript") || content_type.contains("json") {
            "text-x-script".to_string()
        } else if content_type.starts_with("text/") {
            "text-x-generic".to_string()
        } else if content_type.contains("executable") || content_type.contains("shellscript") {
            "application-x-executable".to_string()
        } else {
            "text-x-generic".to_string()
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
