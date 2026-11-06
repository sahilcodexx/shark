use shark::NavigationHistory;
use std::path::PathBuf;

#[test]
fn test_navigation_history_initial_state() {
    let initial = PathBuf::from("/home/user");
    let history = NavigationHistory::new(initial.clone());

    assert_eq!(history.current(), initial.as_path());
    assert!(!history.can_go_back());
    assert!(!history.can_go_forward());
    assert!(history.can_go_up());
    assert_eq!(history.parent(), Some(PathBuf::from("/home")));
}

#[test]
fn test_navigation_history_back_forward() {
    let mut history = NavigationHistory::new(PathBuf::from("/home"));

    history.navigate_to(PathBuf::from("/home/user"));
    assert!(history.can_go_back());
    assert!(!history.can_go_forward());
    assert_eq!(history.current(), PathBuf::from("/home/user").as_path());

    history.navigate_to(PathBuf::from("/home/user/Documents"));
    assert_eq!(history.current(), PathBuf::from("/home/user/Documents").as_path());

    // Go back once
    let prev = history.go_back();
    assert_eq!(prev, Some(PathBuf::from("/home/user")));
    assert_eq!(history.current(), PathBuf::from("/home/user").as_path());
    assert!(history.can_go_forward());
    assert!(history.can_go_back());

    // Go back again
    let prev2 = history.go_back();
    assert_eq!(prev2, Some(PathBuf::from("/home")));
    assert_eq!(history.current(), PathBuf::from("/home").as_path());
    assert!(!history.can_go_back());
    assert!(history.can_go_forward());

    // Go forward
    let next = history.go_forward();
    assert_eq!(next, Some(PathBuf::from("/home/user")));
    assert_eq!(history.current(), PathBuf::from("/home/user").as_path());

    // Navigating to new path clears forward stack
    history.navigate_to(PathBuf::from("/tmp"));
    assert!(!history.can_go_forward());
    assert!(history.can_go_back());
    assert_eq!(history.current(), PathBuf::from("/tmp").as_path());
}

#[test]
fn test_navigation_history_root_parent() {
    let root = PathBuf::from("/");
    let history = NavigationHistory::new(root);
    assert!(!history.can_go_up());
    assert_eq!(history.parent(), None);
}
