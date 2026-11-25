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

#[test]
fn filters_round_trip_and_default_for_existing_configs() {
    use shark::config::{DateFilter, SizeFilter, TypeFilter};

    let config = Config {
        type_filter: TypeFilter::Images,
        date_filter: DateFilter::Last7Days,
        size_filter: SizeFilter::Medium,
        sort_folders_first: false,
        ..Config::default()
    };
    let mut json = serde_json::to_value(config).unwrap();
    let restored: Config = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(restored.type_filter, TypeFilter::Images);
    assert_eq!(restored.date_filter, DateFilter::Last7Days);
    assert_eq!(restored.size_filter, SizeFilter::Medium);
    assert!(!restored.sort_folders_first);

    for field in ["type_filter", "date_filter", "size_filter", "sort_folders_first"] {
        json.as_object_mut().unwrap().remove(field);
    }
    let old: Config = serde_json::from_value(json).unwrap();
    assert_eq!(old.type_filter, TypeFilter::All);
    assert_eq!(old.date_filter, DateFilter::Any);
    assert_eq!(old.size_filter, SizeFilter::Any);
    assert!(old.sort_folders_first);
}

#[test]
fn zoom_round_trips_and_defaults_for_older_config() {
    let config = Config { zoom: 180., ..Config::default() };
    let mut json = serde_json::to_value(config).unwrap();
    assert_eq!(serde_json::from_value::<Config>(json.clone()).unwrap().zoom, 180.);
    json.as_object_mut().unwrap().remove("zoom");
    assert_eq!(serde_json::from_value::<Config>(json).unwrap().zoom, Config::default().zoom);
}
