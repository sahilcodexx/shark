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
        let path = std::env::temp_dir().join(format!("shark_test_{}_{}", name, std::process::id()));
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
fn test_create_folder_and_file() {
    let temp = TempTestDir::new("create_test");

    let folder_path = create_folder(&temp.path, "TestFolder").expect("Should create folder");
    assert!(folder_path.exists());
    assert!(folder_path.is_dir());

    let file_path = create_file(&folder_path, "test.txt").expect("Should create file");
    assert!(file_path.exists());
    assert!(file_path.is_file());

    // Conflict test: creating same folder again fails
    assert!(create_folder(&temp.path, "TestFolder").is_err());
    // Creating same file again fails
    assert!(create_file(&folder_path, "test.txt").is_err());
}

#[test]
fn test_rename_path() {
    let temp = TempTestDir::new("rename_test");

    let file_path = create_file(&temp.path, "original.txt").expect("Should create file");
    let renamed = rename_path(&file_path, "renamed.txt").expect("Should rename file");

    assert!(!file_path.exists());
    assert!(renamed.exists());
    assert_eq!(renamed.file_name().unwrap(), "renamed.txt");
}

#[test]
fn test_unique_path() {
    let temp = TempTestDir::new("unique_path_test");

    let initial = create_file(&temp.path, "data.txt").expect("Should create file");
    assert!(initial.exists());

    let unique1 = get_unique_path(&temp.path, "data.txt");
    assert_eq!(unique1, temp.path.join("data (1).txt"));

    let _ = create_file(&temp.path, "data (1).txt");
    let unique2 = get_unique_path(&temp.path, "data.txt");
    assert_eq!(unique2, temp.path.join("data (2).txt"));
}

#[test]
fn test_delete_permanent() {
    let temp = TempTestDir::new("delete_test");

    let file1 = create_file(&temp.path, "file1.txt").expect("create file1");
    let folder1 = create_folder(&temp.path, "folder1").expect("create folder1");
    let _ = create_file(&folder1, "nested.txt").expect("create nested file");

    let deleted = delete_paths_permanent(&[file1.clone(), folder1.clone()]).expect("delete paths");
    assert_eq!(deleted, 2);
    assert!(!file1.exists());
    assert!(!folder1.exists());
}

#[test]
fn test_copy_paths_async() {
    let temp = TempTestDir::new("copy_async_test");
    let src_dir = temp.path.join("src");
    let dst_dir = temp.path.join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dst_dir).unwrap();

    let file1 = create_file(&src_dir, "file1.txt").unwrap();
    fs::write(&file1, "Hello Shark!").unwrap();

    let (tx, rx) = channel();
    let cancel = Arc::new(AtomicBool::new(false));

    copy_paths_async(vec![file1], dst_dir.clone(), ConflictResolution::KeepBoth, cancel, tx);

    let mut finished = false;
    while let Ok(msg) = rx.recv() {
        if let OpProgress::Finished { success_count, error_count } = msg {
            assert_eq!(success_count, 1);
            assert_eq!(error_count, 0);
            finished = true;
            break;
        }
    }

    assert!(finished);
    assert!(dst_dir.join("file1.txt").exists());
    assert_eq!(fs::read_to_string(dst_dir.join("file1.txt")).unwrap(), "Hello Shark!");
}

#[test]
fn test_move_paths_async() {
    let temp = TempTestDir::new("move_async_test");
    let src_dir = temp.path.join("src");
    let dst_dir = temp.path.join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dst_dir).unwrap();

    let file1 = create_file(&src_dir, "move_me.txt").unwrap();
    fs::write(&file1, "Moving content").unwrap();

    let (tx, rx) = channel();
    let cancel = Arc::new(AtomicBool::new(false));

    move_paths_async(vec![file1.clone()], dst_dir.clone(), ConflictResolution::KeepBoth, cancel, tx);

    let mut finished = false;
    while let Ok(msg) = rx.recv() {
        if let OpProgress::Finished { success_count, error_count } = msg {
            assert_eq!(success_count, 1);
            assert_eq!(error_count, 0);
            finished = true;
            break;
        }
    }

    assert!(finished);
    assert!(!file1.exists());
    assert!(dst_dir.join("move_me.txt").exists());
}
