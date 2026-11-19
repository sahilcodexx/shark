//! Utility functions for the Shark file manager.

use md5::{Digest, Md5};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Convert bytes to human-readable size string.
pub fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    const TB: f64 = 1024.0 * 1024.0 * 1024.0 * 1024.0;

    let b = bytes as f64;
    if b >= TB {
        format!("{:.1} TB", b / TB)
    } else if b >= GB {
        format!("{:.1} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else if bytes == 1 {
        "1 byte".to_string()
    } else {
        format!("{} bytes", bytes)
    }
}

/// Format SystemTime to readable string.
pub fn format_date(time: Option<SystemTime>) -> String {
    let Some(time) = time else {
        return "Unknown".to_string();
    };
    let dt: chrono::DateTime<chrono::Local> = time.into();
    dt.format("%Y-%m-%d %H:%M").to_string()
}

/// Format Unix file permissions to standard rwxrwxrwx string.
pub fn format_permissions(mode: u32, is_dir: bool) -> String {
    if mode == 0 {
        return "---------".to_string();
    }

    let dir_char = if is_dir { 'd' } else { '-' };
    let user_r = if mode & 0o400 != 0 { 'r' } else { '-' };
    let user_w = if mode & 0o200 != 0 { 'w' } else { '-' };
    let user_x = if mode & 0o100 != 0 { 'x' } else { '-' };

    let group_r = if mode & 0o040 != 0 { 'r' } else { '-' };
    let group_w = if mode & 0o020 != 0 { 'w' } else { '-' };
    let group_x = if mode & 0o010 != 0 { 'x' } else { '-' };

    let other_r = if mode & 0o004 != 0 { 'r' } else { '-' };
    let other_w = if mode & 0o002 != 0 { 'w' } else { '-' };
    let other_x = if mode & 0o001 != 0 { 'x' } else { '-' };

    format!(
        "{}{}{}{}{}{}{}{}{}{}",
        dir_char, user_r, user_w, user_x, group_r, group_w, group_x, other_r, other_w, other_x
    )
}

/// Get user-friendly description for MIME content type.
pub fn get_content_type_description(mime: &str, is_dir: bool) -> String {
    if is_dir {
        return "Folder".to_string();
    }

    match mime {
        "text/plain" => "Plain Text Document".to_string(),
        "text/x-rust" => "Rust Source File".to_string(),
        "text/x-python" => "Python Script".to_string(),
        "text/x-c" => "C Source Code".to_string(),
        "text/x-c++" => "C++ Source Code".to_string(),
        "application/javascript" => "JavaScript File".to_string(),
        "text/html" => "HTML Document".to_string(),
        "text/css" => "CSS Stylesheet".to_string(),
        "application/json" => "JSON Document".to_string(),
        "application/toml" => "TOML Configuration".to_string(),
        "application/yaml" => "YAML Document".to_string(),
        "application/xml" => "XML Document".to_string(),
        "application/pdf" => "PDF Document".to_string(),
        "application/zip" => "Zip Archive".to_string(),
        "application/x-archive" => "Compressed Archive".to_string(),
        "application/x-shellscript" => "Shell Script".to_string(),
        "application/x-executable" => "Executable Program".to_string(),
        "image/png" => "PNG Image".to_string(),
        "image/jpeg" => "JPEG Image".to_string(),
        "image/gif" => "GIF Image".to_string(),
        "image/svg+xml" => "SVG Vector Graphic".to_string(),
        "image/webp" => "WebP Image".to_string(),
        "video/mp4" => "MP4 Video".to_string(),
        "audio/mpeg" => "MP3 Audio".to_string(),
        _ => {
            if mime.starts_with("image/") {
                "Image".to_string()
            } else if mime.starts_with("video/") {
                "Video".to_string()
            } else if mime.starts_with("audio/") {
                "Audio".to_string()
            } else if mime.starts_with("text/") {
                "Text Document".to_string()
            } else {
                "File".to_string()
            }
        }
    }
}

/// Get the file extension from a path.
pub fn get_extension(path: &Path) -> Option<String> {
    path.extension().and_then(|e| e.to_str()).map(|e| e.to_string())
}

/// Format free disk space for a given path using POSIX statvfs.
pub fn get_disk_free_space<P: AsRef<Path>>(path: P) -> Option<String> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let cpath = CString::new(path.as_ref().as_os_str().as_bytes()).ok()?;
    unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(cpath.as_ptr(), &mut stat) == 0 {
            let free_bytes = stat.f_bavail as u64 * stat.f_frsize as u64;
            return Some(format_size(free_bytes));
        }
    }
    None
}

/// Check if a file is an image based on MIME type or file extension.
pub fn is_image_entry(content_type: &str, path: &Path) -> bool {
    if content_type.starts_with("image/") {
        return true;
    }
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        matches!(
            ext.to_lowercase().as_str(),
            "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" | "bmp" | "ico" | "tiff" | "tif" | "avif"
        )
    } else {
        false
    }
}

/// Check if a file is a video based on MIME type or file extension.
pub fn is_video_entry(content_type: &str, path: &Path) -> bool {
    if content_type.starts_with("video/") {
        return true;
    }
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        matches!(
            ext.to_lowercase().as_str(),
            "mp4" | "mkv" | "webm" | "avi" | "mov" | "flv" | "wmv" | "m4v" | "3gp" | "ts" | "mpg" | "mpeg"
        )
    } else {
        false
    }
}

/// Check if an entry is previewable media (image or video).
pub fn is_previewable_media(content_type: &str, path: &Path) -> bool {
    is_image_entry(content_type, path) || is_video_entry(content_type, path)
}

/// RFC 2396 percent-encode path for Freedesktop URI standard.
pub fn percent_encode_path(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.as_bytes() {
        match *b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(*b as char);
            }
            byte => {
                use std::fmt::Write;
                let _ = write!(out, "%{:02X}", byte);
            }
        }
    }
    out
}

/// Find a cached Freedesktop thumbnail on disk in ~/.cache/thumbnails/.
/// Supports both RFC 2396 percent-encoded URIs (Dolphin/GNOME) and raw URIs.
pub fn find_cached_thumbnail_path(path: &Path) -> Option<PathBuf> {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let path_str = canonical.to_string_lossy();
    let home = dirs::home_dir()?;
    let thumbs_root = home.join(".cache/thumbnails");

    // 1. Raw URI (file:///path)
    let raw_uri = format!("file://{}", path_str);
    let hash_raw = format!("{:x}", Md5::digest(raw_uri.as_bytes()));

    // 2. Encoded URI (file:///path%20with%20spaces - Freedesktop standard)
    let enc_uri = format!("file://{}", percent_encode_path(&path_str));
    let hash_enc = format!("{:x}", Md5::digest(enc_uri.as_bytes()));

    for folder in ["normal", "large", "x-large"] {
        let dir = thumbs_root.join(folder);
        let p_enc = dir.join(format!("{}.png", hash_enc));
        if p_enc.exists() {
            return Some(p_enc);
        }
        let p_raw = dir.join(format!("{}.png", hash_raw));
        if p_raw.exists() {
            return Some(p_raw);
        }
    }
    None
}

/// Generate a thumbnail into ~/.cache/thumbnails/normal/ in a background worker.
pub fn generate_thumbnail_file(path: &Path, content_type: &str, _size: u64) -> Option<PathBuf> {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let path_str = canonical.to_string_lossy();
    let enc_uri = format!("file://{}", percent_encode_path(&path_str));
    let hash = format!("{:x}", Md5::digest(enc_uri.as_bytes()));
    let home = dirs::home_dir()?;
    let normal_cache = home.join(".cache/thumbnails/normal");
    let _ = std::fs::create_dir_all(&normal_cache);
    let target_thumb = normal_cache.join(format!("{}.png", hash));

    if target_thumb.exists() {
        return Some(target_thumb);
    }

    // 1. Video thumbnail via ffmpegthumbnailer or ffmpeg
    if is_video_entry(content_type, path) {
        let thumb_ok = std::process::Command::new("ffmpegthumbnailer")
            .arg("-i")
            .arg(path)
            .arg("-o")
            .arg(&target_thumb)
            .arg("-s")
            .arg("256")
            .arg("-c")
            .arg("png")
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if !thumb_ok || !target_thumb.exists() {
            let _ = std::process::Command::new("ffmpeg")
                .arg("-ss")
                .arg("00:00:01")
                .arg("-i")
                .arg(path)
                .arg("-vframes")
                .arg("1")
                .arg("-vf")
                .arg("scale=256:-1")
                .arg("-y")
                .arg(&target_thumb)
                .status();
        }

        if target_thumb.exists() {
            return Some(target_thumb);
        }
    }

    // 2. Image thumbnail via image crate downsampled to 256x256
    if is_image_entry(content_type, path) {
        if let Ok(reader) = image::ImageReader::open(path) {
            if let Ok(reader) = reader.with_guessed_format() {
                if let Ok(img) = reader.decode() {
                    let thumb = img.thumbnail(256, 256);
                    if thumb.save_with_format(&target_thumb, image::ImageFormat::Png).is_ok() {
                        return Some(target_thumb);
                    }
                }
            }
        }
    }

    None
}
