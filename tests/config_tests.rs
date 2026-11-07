use shark::Config;
use std::path::PathBuf;

#[test]
fn test_default_config() {
    let config = Config::default();
    assert_eq!(config.view_mode, "grid");
    assert!(!config.show_hidden);
    assert_eq!(config.sort_by, "name");
    assert!(!config.sort_reversed);
    assert_eq!(config.background_mode, "solid");
    assert!(config.window_width >= 800);
    assert!(config.window_height >= 600);
}

#[test]
fn test_config_bookmarks() {
    let mut config = Config::default();
    let sample = PathBuf::from("/tmp/my_favorite_folder");

    config.add_bookmark(&sample);
    assert!(config.bookmarks.contains(&sample));

    // Adding duplicate does not add twice
    config.add_bookmark(&sample);
    assert_eq!(config.bookmarks.iter().filter(|&p| p == &sample).count(), 1);

    config.remove_bookmark(&sample);
    assert!(!config.bookmarks.contains(&sample));
}
