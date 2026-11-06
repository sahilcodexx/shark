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
