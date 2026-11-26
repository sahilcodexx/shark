use shark::filesystem::*;
use shark::filesystem::operations::folder_size;
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

#[test]
fn rename_preserves_existing_destination_even_if_symlink_is_dangling() {
    let temp = TempTestDir::new("rename_conflict");
    let source = temp.path.join("source.txt");
    let destination = temp.path.join("target.txt");
    std::fs::write(&source, "keep source").unwrap();
    std::os::unix::fs::symlink(temp.path.join("missing"), &destination).unwrap();

    assert!(rename_path(&source, "target.txt").is_err());
    assert!(destination.is_symlink());
    assert_eq!(std::fs::read_to_string(&source).unwrap(), "keep source");
}

#[test]
fn folder_size_counts_nested_files_without_following_links() {
    let temp = TempTestDir::new("properties_size");
    let child = temp.path.join("nested");
    std::fs::create_dir(&child).unwrap();
    std::fs::write(temp.path.join("one"), b"123").unwrap();
    std::fs::write(child.join("two"), b"12345").unwrap();
    std::os::unix::fs::symlink(&temp.path, child.join("loop")).unwrap();
    assert_eq!(folder_size(&temp.path, &AtomicBool::new(false)).unwrap(), 8);
}

#[test]
fn transfers_preserve_existing_entries_and_reject_self_copy() {
    use shark::filesystem::operations::transfer_into;

    let temp = TempTestDir::new("transfer_into");
    let source = temp.path.join("example");
    let destination = temp.path.join("destination");
    fs::create_dir(&source).unwrap();
    fs::create_dir(&destination).unwrap();
    fs::write(source.join("data"), b"payload").unwrap();
    std::os::unix::fs::symlink("data", source.join("link")).unwrap();
    assert!(transfer_into(&source, &source, false).is_err());
    transfer_into(&source, &destination, false).unwrap();
    assert_eq!(fs::read_link(destination.join("example/link")).unwrap(), PathBuf::from("data"));
    assert!(transfer_into(&source, &destination, true).is_err());
    assert!(source.is_dir());
    fs::remove_dir_all(destination.join("example")).unwrap();
    transfer_into(&source, &destination, true).unwrap();
    assert!(!source.exists());
    assert_eq!(fs::read(destination.join("example/data")).unwrap(), b"payload");
}
