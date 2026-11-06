use shark::filesystem::*;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::channel;
use std::sync::Arc;

struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("shark_scanner_test_{}_{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create test dir");
        Self { path }
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn test_scan_directory_async() {
    let temp = TempTestDir::new("scan_test");

    // Populate with 10 files and 2 folders
    for i in 0..10 {
        let _ = create_file(&temp.path, &format!("file_{}.txt", i)).unwrap();
    }
    for i in 0..2 {
        let _ = create_folder(&temp.path, &format!("folder_{}", i)).unwrap();
    }

    let (tx, rx) = channel();
    let cancel = Arc::new(AtomicBool::new(false));

    scan_directory_async(temp.path.clone(), cancel, tx);

    let mut total_found = 0;
    let mut completed = false;

    while let Ok(msg) = rx.recv() {
        match msg {
            ScanMessage::Batch(entries) => {
                total_found += entries.len();
            }
            ScanMessage::Completed { total_items, .. } => {
                assert_eq!(total_items, 12);
                completed = true;
                break;
            }
            ScanMessage::Error(e) => {
                panic!("Scanner error: {}", e);
            }
        }
    }

    assert!(completed);
    assert_eq!(total_found, 12);
}

#[test]
fn test_recursive_search_async() {
    let temp = TempTestDir::new("recursive_search_test");

    // Nested directory structure
    let sub = create_folder(&temp.path, "subdir").unwrap();
    let deep = create_folder(&sub, "deepdir").unwrap();

    let _ = create_file(&temp.path, "root_apple.txt").unwrap();
    let _ = create_file(&sub, "sub_apple.md").unwrap();
    let _ = create_file(&deep, "deep_banana.txt").unwrap();
    let _ = create_file(&deep, "deep_apple_pie.rs").unwrap();

    let (tx, rx) = channel();
    let cancel = Arc::new(AtomicBool::new(false));

    recursive_search_async(temp.path.clone(), "apple".to_string(), cancel, tx);

    let mut found_names = Vec::new();
    let mut completed = false;

    while let Ok(msg) = rx.recv() {
        match msg {
            ScanMessage::Batch(entries) => {
                for e in entries {
                    found_names.push(e.name);
                }
            }
            ScanMessage::Completed { .. } => {
                completed = true;
                break;
            }
            ScanMessage::Error(e) => {
                panic!("Search error: {}", e);
            }
        }
    }

    assert!(completed);
    assert_eq!(found_names.len(), 3);
    assert!(found_names.iter().all(|name| name.contains("apple")));
}
