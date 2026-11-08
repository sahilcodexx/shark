use shark::models::FileEntry;
use std::path::Path;

#[test]
fn test_format_size() {
    assert_eq!(shark::utils::format_size(0), "0 bytes");
    assert_eq!(shark::utils::format_size(1), "1 byte");
    assert_eq!(shark::utils::format_size(1023), "1023 bytes");
    assert_eq!(shark::utils::format_size(1024), "1.0 KB");
    assert_eq!(shark::utils::format_size(1024 * 1024), "1.0 MB");
    assert_eq!(shark::utils::format_size(1024 * 1024 * 1024), "1.0 GB");
}

#[test]
fn test_format_permissions() {
    assert_eq!(shark::utils::format_permissions(0o755, true), "drwxr-xr-x");
    assert_eq!(shark::utils::format_permissions(0o644, false), "-rw-r--r--");
    assert_eq!(shark::utils::format_permissions(0o777, false), "-rwxrwxrwx");
}

#[test]
fn test_determine_icon() {
    assert_eq!(FileEntry::determine_icon("inode/directory", true, false, "Documents"), "folder-documents");
    assert_eq!(FileEntry::determine_icon("inode/directory", true, false, "Downloads"), "folder-download");
    assert_eq!(FileEntry::determine_icon("inode/directory", true, false, "some_folder"), "folder");
    assert_eq!(FileEntry::determine_icon("image/png", false, false, "photo.png"), "image-x-generic");
    assert_eq!(FileEntry::determine_icon("video/mp4", false, false, "movie.mp4"), "video-x-generic");
    assert_eq!(FileEntry::determine_icon("application/pdf", false, false, "doc.pdf"), "application-pdf");
    assert_eq!(FileEntry::determine_icon("application/zip", false, false, "archive.zip"), "package-x-generic");
    assert_eq!(FileEntry::determine_icon("audio/mpeg", false, false, "song.mp3"), "audio-x-generic");
    assert_eq!(FileEntry::determine_icon("text/x-rust", false, false, "main.rs"), "text-x-script");
}

#[test]
fn test_guess_content_type() {
    assert_eq!(FileEntry::guess_content_type("pdf"), "application/pdf");
    assert_eq!(FileEntry::guess_content_type("zip"), "application/zip");
    assert_eq!(FileEntry::guess_content_type("mp3"), "audio/mpeg");
    assert_eq!(FileEntry::guess_content_type("mp4"), "video/mp4");
    assert_eq!(FileEntry::guess_content_type("rs"), "text/x-rust");
    assert_eq!(FileEntry::guess_content_type("png"), "image/png");
}

#[test]
fn test_from_path() {
    let cargo_toml = Path::new("Cargo.toml");
    let entry = FileEntry::from_path(cargo_toml).expect("Cargo.toml should exist");

    assert_eq!(entry.name, "Cargo.toml");
    assert!(!entry.is_directory);
    assert!(!entry.is_hidden);
    assert!(entry.size > 0);
    assert_eq!(entry.content_type, "application/toml");
}

#[test]
fn test_sorting_entries() {
    let mut entries = vec![
        FileEntry {
            name: "zeta.txt".to_string(),
            name_lower: "zeta.txt".to_string(),
            path: std::path::PathBuf::from("/zeta.txt"),
            is_directory: false,
            is_symlink: false,
            is_hidden: false,
            size: 500,
            modified: Some(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(100)),
            created: None,
            permissions: 0o644,
            owner: "user".to_string(),
            group: "user".to_string(),
            icon_name: "text-x-generic".to_string(),
            content_type: "text/plain".to_string(),
        },
        FileEntry {
            name: "alpha.png".to_string(),
            name_lower: "alpha.png".to_string(),
            path: std::path::PathBuf::from("/alpha.png"),
            is_directory: false,
            is_symlink: false,
            is_hidden: false,
            size: 1500,
            modified: Some(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(300)),
            created: None,
            permissions: 0o644,
            owner: "user".to_string(),
            group: "user".to_string(),
            icon_name: "image-x-generic".to_string(),
            content_type: "image/png".to_string(),
        },
        FileEntry {
            name: "Folder B".to_string(),
            name_lower: "folder b".to_string(),
            path: std::path::PathBuf::from("/Folder B"),
            is_directory: true,
            is_symlink: false,
            is_hidden: false,
            size: 4096,
            modified: Some(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(200)),
            created: None,
            permissions: 0o755,
            owner: "user".to_string(),
            group: "user".to_string(),
            icon_name: "folder".to_string(),
            content_type: "inode/directory".to_string(),
        },
    ];

    // Directories first, then name ascending
    entries.sort_by(|a, b| {
        match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });
    assert_eq!(entries[0].name, "Folder B");
    assert_eq!(entries[1].name, "alpha.png");
    assert_eq!(entries[2].name, "zeta.txt");

    // Directories first, then size descending
    entries.sort_by(|a, b| {
        match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => b.size.cmp(&a.size),
        }
    });
    assert_eq!(entries[0].name, "Folder B");
    assert_eq!(entries[1].name, "alpha.png");
    assert_eq!(entries[2].name, "zeta.txt");
}
