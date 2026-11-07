//! Utility functions for the Shark file manager.

use std::path::Path;
use std::time::SystemTime;
use gio::prelude::*;

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

    let Ok(elapsed) = time.duration_since(SystemTime::UNIX_EPOCH) else {
        return "Unknown".to_string();
    };

    let total_secs = elapsed.as_secs();
    // Use glib::DateTime for timezone-aware local time formatting
    if let Ok(dt) = glib::DateTime::from_unix_local(total_secs as i64)
        && let Ok(formatted) = dt.format("%Y-%m-%d %H:%M") {
            return formatted.to_string();
        }

    // Fallback: simple date calculation
    let days = total_secs / 86400;
    format!("{} days ago", days)
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

#[allow(dead_code)]
/// Get the file extension from a path.
pub fn get_extension(path: &Path) -> Option<String> {
    path.extension().and_then(|e| e.to_str()).map(|e| e.to_string())
}

/// Format free disk space for a given path.
pub fn get_disk_free_space<P: AsRef<Path>>(path: P) -> Option<String> {
    let file = gio::File::for_path(path);
    let info = file.query_filesystem_info("filesystem::free", gio::Cancellable::NONE).ok()?;
    let free = info.attribute_uint64("filesystem::free");
    Some(format_size(free))
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

/// Load a media thumbnail (image or video) with Freedesktop cache lookup and on-the-fly generation.
pub fn load_media_thumbnail(
    path: &Path,
    content_type: &str,
    size: u64,
    target_width: i32,
    target_height: i32,
) -> Option<gtk4::gdk::Texture> {
    if !is_previewable_media(content_type, path) {
        return None;
    }

    // 1. Try freedesktop thumbnail cache (~/.cache/thumbnails/large, normal, or x-large)
    let gfile = gio::File::for_path(path);
    let uri = gfile.uri();
    let home = glib::home_dir();
    if let Some(hash) = glib::compute_checksum_for_string(glib::ChecksumType::Md5, &uri) {
        let filename = format!("{}.png", hash);
        for folder in ["large", "normal", "x-large"] {
            let thumb_path = home
                .join(".cache/thumbnails")
                .join(folder)
                .join(&filename);
            if thumb_path.exists() {
                if let Ok(pixbuf) = gtk4::gdk_pixbuf::Pixbuf::from_file_at_scale(&thumb_path, target_width, target_height, true) {
                    return Some(gtk4::gdk::Texture::for_pixbuf(&pixbuf));
                }
            }
        }

        // 2. Video thumbnail generation via ffmpegthumbnailer or ffmpeg
        if is_video_entry(content_type, path) {
            let normal_cache = home.join(".cache/thumbnails/normal");
            let _ = std::fs::create_dir_all(&normal_cache);
            let target_thumb = normal_cache.join(&filename);

            let thumb_ok = std::process::Command::new("ffmpegthumbnailer")
                .arg("-i")
                .arg(path)
                .arg("-o")
                .arg(&target_thumb)
                .arg("-s")
                .arg("128")
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
                    .arg("scale=128:-1")
                    .arg("-y")
                    .arg(&target_thumb)
                    .status();
            }

            if target_thumb.exists() {
                if let Ok(pixbuf) = gtk4::gdk_pixbuf::Pixbuf::from_file_at_scale(&target_thumb, target_width, target_height, true) {
                    return Some(gtk4::gdk::Texture::for_pixbuf(&pixbuf));
                }
            }
        }
    }

    // 3. Direct on-the-fly image thumbnail generation for images under 25MB
    if is_image_entry(content_type, path) && size <= 25 * 1024 * 1024 {
        if let Ok(pixbuf) = gtk4::gdk_pixbuf::Pixbuf::from_file_at_scale(path, target_width, target_height, true) {
            return Some(gtk4::gdk::Texture::for_pixbuf(&pixbuf));
        }
    }

    None
}

/// Load an image thumbnail (backwards-compatible alias).
pub fn load_image_thumbnail(
    path: &Path,
    content_type: &str,
    size: u64,
    target_width: i32,
    target_height: i32,
) -> Option<gtk4::gdk::Texture> {
    load_media_thumbnail(path, content_type, size, target_width, target_height)
}
