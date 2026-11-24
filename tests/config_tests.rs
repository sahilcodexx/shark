use shark::Config;
use std::path::PathBuf;

#[test]
fn test_default_config() {
    let config = Config::default();
    assert_eq!(config.view_mode, "grid");
    assert!(!config.show_hidden);
    assert_eq!(config.sort_by, "name");
    assert!(!config.sort_reversed);
    assert_eq!(config.background_mode, "frosted");
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

#[test]
fn tags_round_trip_and_toggle_independently_of_filename() {
    let mut config = Config::default();
    let path = std::path::Path::new("/tmp/report.txt");
    config.toggle_tag(path, "Important");
    config.toggle_tag(path, "Work");

    let json = serde_json::to_value(&config).unwrap();
    let mut restored: Config = serde_json::from_value(json.clone()).unwrap();
    restored.toggle_tag(path, "Important");
    assert_eq!(restored.tags.get(path).unwrap(), &["Work"]);
    restored.toggle_tag(path, "Work");
    assert!(restored.tags.is_empty());

    let mut old = json;
    old.as_object_mut().unwrap().remove("tags");
    assert!(serde_json::from_value::<Config>(old).unwrap().tags.is_empty());
}
