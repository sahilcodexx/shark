//! Directory navigation sidebar for Shark file manager.

use crate::config::Config;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

type NavigateCallback = Rc<RefCell<Option<Box<dyn Fn(PathBuf)>>>>;
type ActionCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;

#[derive(Clone)]
pub struct Sidebar {
    pub container: gtk4::Box,
    places_list: gtk4::ListBox,
    bookmarks_list: gtk4::ListBox,
    drives_list: gtk4::ListBox,
    config: Rc<RefCell<Config>>,
    on_navigate: NavigateCallback,
    on_bookmark_changed: ActionCallback,
}

impl Sidebar {
    pub fn new(config: Rc<RefCell<Config>>) -> Self {
        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .width_request(200)
            .css_classes(["sidebar"])
            .build();

        let scrolled = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .build();

        let inner_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(16)
            .margin_start(10)
            .margin_end(10)
            .margin_top(12)
            .margin_bottom(12)
            .build();

        // 1. Places Header & List
        let places_header = gtk4::Label::builder()
            .label("PLACES")
            .halign(gtk4::Align::Start)
            .css_classes(["sidebar-section-header"])
            .build();

        let places_list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::Single)
            .css_classes(["sidebar-list"])
            .build();

        // 2. Bookmarks Header & List
        let bookmarks_header_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        let bookmarks_header = gtk4::Label::builder()
            .label("BOOKMARKS")
            .halign(gtk4::Align::Start)
            .hexpand(true)
            .css_classes(["sidebar-section-header"])
            .build();

        bookmarks_header_box.append(&bookmarks_header);

        let bookmarks_list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::Single)
            .css_classes(["sidebar-list"])
            .build();

        // 3. Drives Header & List
        let drives_header = gtk4::Label::builder()
            .label("DRIVES")
            .halign(gtk4::Align::Start)
            .css_classes(["sidebar-section-header"])
            .build();

        let drives_list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::Single)
            .css_classes(["sidebar-list"])
            .build();

        inner_box.append(&places_header);
        inner_box.append(&places_list);
        inner_box.append(&bookmarks_header_box);
        inner_box.append(&bookmarks_list);
        inner_box.append(&drives_header);
        inner_box.append(&drives_list);

        scrolled.set_child(Some(&inner_box));
        container.append(&scrolled);

        let on_navigate = Rc::new(RefCell::new(None));
        let on_bookmark_changed = Rc::new(RefCell::new(None));

        let sidebar = Self {
            container,
            places_list,
            bookmarks_list,
            drives_list,
            config,
            on_navigate,
            on_bookmark_changed,
        };

        sidebar.populate_places();
        sidebar.populate_bookmarks();
        sidebar.populate_drives();
        sidebar.setup_row_activation();

        sidebar
    }

    /// Add a standard sidebar row.
    fn create_row(icon_name: &str, label: &str, path: PathBuf) -> gtk4::ListBoxRow {
        let row = gtk4::ListBoxRow::new();
        let box_w = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(10)
            .margin_start(8)
            .margin_end(8)
            .margin_top(6)
            .margin_bottom(6)
            .build();

        let icon = gtk4::Image::from_icon_name(icon_name);
        icon.set_pixel_size(18);

        let lbl = gtk4::Label::builder()
            .label(label)
            .halign(gtk4::Align::Start)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .build();

        box_w.append(&icon);
        box_w.append(&lbl);
        row.set_child(Some(&box_w));

        row.set_widget_name(&path.to_string_lossy());

        row
    }

    /// Populate standard places.
    fn populate_places(&self) {
        let home = glib::home_dir();

        let items = [
            ("user-home-symbolic", "Home", home.clone()),
            ("user-desktop-symbolic", "Desktop", home.join("Desktop")),
            ("folder-documents-symbolic", "Documents", home.join("Documents")),
            ("folder-download-symbolic", "Downloads", home.join("Downloads")),
            ("folder-music-symbolic", "Music", home.join("Music")),
            ("folder-pictures-symbolic", "Pictures", home.join("Pictures")),
            ("folder-videos-symbolic", "Videos", home.join("Videos")),
            ("user-trash-symbolic", "Trash", PathBuf::from("trash:///")),
            ("drive-harddisk-symbolic", "File System", PathBuf::from("/")),
        ];

        for (icon, name, path) in items {
            // Include Trash and Root, and home directories if they exist or are home itself
            if path.to_string_lossy() == "trash:///" || path == *"/" || path == home || path.exists() {
                let row = Self::create_row(icon, name, path);
                self.places_list.append(&row);
            }
        }
    }

    /// Populate user bookmarks.
    pub fn populate_bookmarks(&self) {
        // Clear existing
        while let Some(child) = self.bookmarks_list.first_child() {
            self.bookmarks_list.remove(&child);
        }

        let config = self.config.borrow();
        for bookmark in &config.bookmarks {
            if bookmark.exists() {
                let name = bookmark
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| bookmark.to_string_lossy().to_string());

                let row = Self::create_row("folder-symbolic", &name, bookmark.clone());

                // Right click context menu to remove bookmark
                let gesture = gtk4::GestureClick::builder().button(3).build();
                let bookmark_path = bookmark.clone();
                let config_c = self.config.clone();
                let list_c = self.bookmarks_list.clone();
                let row_c = row.clone();
                let on_changed_c = self.on_bookmark_changed.clone();

                gesture.connect_released(move |_, _, _, _| {
                    let popover = gtk4::Popover::new();
                    let remove_btn = gtk4::Button::with_label("Remove Bookmark");
                    remove_btn.add_css_class("flat");

                    let config_inner = config_c.clone();
                    let path_inner = bookmark_path.clone();
                    let list_inner = list_c.clone();
                    let row_inner = row_c.clone();
                    let popover_c = popover.clone();
                    let on_changed_inner = on_changed_c.clone();

                    remove_btn.connect_clicked(move |_| {
                        popover_c.popdown();
                        config_inner.borrow_mut().remove_bookmark(&path_inner);
                        list_inner.remove(&row_inner);
                        if let Some(ref cb) = *on_changed_inner.borrow() {
                            cb();
                        }
                    });

                    popover.set_child(Some(&remove_btn));
                    popover.set_parent(&row_c);
                    popover.popup();
                });

                row.add_controller(gesture);
                self.bookmarks_list.append(&row);
            }
        }
    }

    /// Populate standard drive mounts.
    fn populate_drives(&self) {
        let drives = [
            ("drive-harddisk-symbolic", "Root", PathBuf::from("/")),
            ("drive-removable-media-symbolic", "Mounts", PathBuf::from("/mnt")),
            ("drive-removable-media-symbolic", "Media", PathBuf::from("/media")),
        ];

        for (icon, name, path) in drives {
            if path.exists() {
                let row = Self::create_row(icon, name, path);
                self.drives_list.append(&row);
            }
        }
    }

    /// Set up row activation on all list boxes.
    fn setup_row_activation(&self) {
        let make_handler = |on_nav: NavigateCallback| {
            move |_: &gtk4::ListBox, row: &gtk4::ListBoxRow| {
                let path_str = row.widget_name().to_string();
                if !path_str.is_empty() {
                    let path = PathBuf::from(path_str);
                    if let Some(ref cb) = *on_nav.borrow() {
                        cb(path);
                    }
                }
            }
        };

        self.places_list.connect_row_activated(make_handler(self.on_navigate.clone()));
        self.bookmarks_list.connect_row_activated(make_handler(self.on_navigate.clone()));
        self.drives_list.connect_row_activated(make_handler(self.on_navigate.clone()));
    }

    /// Set navigation callback.
    pub fn connect_navigate<F: Fn(PathBuf) + 'static>(&self, f: F) {
        *self.on_navigate.borrow_mut() = Some(Box::new(f));
    }

    /// Set bookmark changed callback.
    pub fn connect_bookmarks_changed<F: Fn() + 'static>(&self, f: F) {
        *self.on_bookmark_changed.borrow_mut() = Some(Box::new(f));
    }

    /// Highlight current directory in sidebar if matches a place or bookmark.
    pub fn set_current_path(&self, current: &Path) {
        let current_str = current.to_string_lossy();
        let unselect_if_mismatch = |list: &gtk4::ListBox| {
            let mut child = list.first_child();
            while let Some(row) = child {
                if let Some(r) = row.downcast_ref::<gtk4::ListBoxRow>()
                    && r.widget_name().as_str() == current_str {
                        list.select_row(Some(r));
                        return;
                    }
                child = row.next_sibling();
            }
            list.unselect_all();
        };

        unselect_if_mismatch(&self.places_list);
        unselect_if_mismatch(&self.bookmarks_list);
        unselect_if_mismatch(&self.drives_list);
    }
}
