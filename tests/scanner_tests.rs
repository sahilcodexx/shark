use shark::filesystem::*;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::channel;

struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "shark_scanner_test_{}_{}",
            name,
            std::process::id()
        ));
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

#[test]
fn search_home_finds_downloads_and_deep_files() {
    let home = TempTestDir::new("home_search");
    let downloads = home.path.join("Downloads");
    fs::create_dir(&downloads).unwrap();
    let resume = downloads.join("Sahil-Singh-Resume.pdf");
    fs::write(&resume, b"resume").unwrap();
    let mut deep = home.path.clone();
    for _ in 0..16 {
        deep.push("nested");
    }
    fs::create_dir_all(&deep).unwrap();
    let second = deep.join("RESUME.txt");
    fs::write(&second, b"deep").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&home.path, downloads.join("loop")).unwrap();
    let (tx, rx) = channel();
    recursive_search_async(
        home.path.clone(),
        " resume ".into(),
        Arc::new(AtomicBool::new(false)),
        tx,
    );
    let mut paths = Vec::new();
    loop {
        match rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap() {
            ScanMessage::Batch(entries) => {
                paths.extend(entries.into_iter().map(|entry| entry.path))
            }
            ScanMessage::Completed { total_items, .. } => {
                assert_eq!(total_items, 2);
                break;
            }
            ScanMessage::Error(error) => panic!("{error}"),
        }
    }
    assert!(paths.contains(&resume));
    assert!(paths.contains(&second));
}

#[test]
fn search_reports_unavailable_root_and_honors_cancellation() {
    let root = TempTestDir::new("unavailable_search");
    let (tx, rx) = channel();
    recursive_search_async(
        root.path.join("missing"),
        "resume".into(),
        Arc::new(AtomicBool::new(false)),
        tx,
    );
    assert!(matches!(
        rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap(),
        ScanMessage::Error(_)
    ));
    let (tx, rx) = channel();
    recursive_search_async(
        root.path.clone(),
        "resume".into(),
        Arc::new(AtomicBool::new(true)),
        tx,
    );
    assert!(matches!(
        rx.recv_timeout(std::time::Duration::from_secs(2)),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected)
    ));
}
