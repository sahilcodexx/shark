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
    bookmarks_section: gtk4::Box,
    drives_list: gtk4::ListBox,
    config: Rc<RefCell<Config>>,
    on_navigate: NavigateCallback,
    on_bookmark_changed: ActionCallback,
}

impl Sidebar {
    pub fn new(config: Rc<RefCell<Config>>) -> Self {
        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .width_request(224)
            .css_classes(["sidebar"])
            .build();

        let scrolled = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .build();

        let inner_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .margin_start(12)
            .margin_end(12)
            .margin_top(10)
            .margin_bottom(12)
            .build();

        // 1. Places Header & List
        let places_header = gtk4::Label::builder()
            .label("Places")
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
            .label("Bookmarks")
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
            .label("Locations")
            .halign(gtk4::Align::Start)
            .css_classes(["sidebar-section-header"])
            .build();

        let drives_list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::Single)
            .css_classes(["sidebar-list"])
            .build();

        inner_box.append(&places_header);
        inner_box.append(&places_list);
        let bookmarks_section = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        bookmarks_section.append(&bookmarks_header_box);
        bookmarks_section.append(&bookmarks_list);
        inner_box.append(&bookmarks_section);
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
            bookmarks_section,
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
    fn create_row(icon_name: &str, label: &str, path: PathBuf, icon_class: &str) -> gtk4::ListBoxRow {
        let row = gtk4::ListBoxRow::new();
        row.add_css_class("sidebar-row");

        let box_w = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .margin_start(12)
            .margin_end(12)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let icon = gtk4::Image::from_icon_name(icon_name);
        icon.set_pixel_size(17);
        if !icon_class.is_empty() {
            icon.add_css_class(icon_class);
        }

        let lbl = gtk4::Label::builder()
            .label(label)
            .halign(gtk4::Align::Start)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .build();

        box_w.append(&icon);
        box_w.append(&lbl);
        row.set_child(Some(&box_w));

        row.set_tooltip_text(Some(&path.to_string_lossy()));
        row.set_widget_name(&path.to_string_lossy());

        row
    }

    /// Populate standard places.
    fn populate_places(&self) {
        let home = glib::home_dir();

        let items = [
            ("user-home-symbolic", "Home", home.clone(), "icon-home"),
            ("user-desktop-symbolic", "Desktop", home.join("Desktop"), "icon-desktop"),
            ("folder-documents-symbolic", "Documents", home.join("Documents"), "icon-documents"),
            ("folder-download-symbolic", "Downloads", home.join("Downloads"), "icon-downloads"),
            ("folder-music-symbolic", "Music", home.join("Music"), "icon-music"),
            ("folder-pictures-symbolic", "Pictures", home.join("Pictures"), "icon-pictures"),
            ("folder-videos-symbolic", "Videos", home.join("Videos"), "icon-videos"),
            ("user-trash-symbolic", "Trash", PathBuf::from("trash:///"), "icon-trash"),
        ];

        for (icon, name, path, icon_class) in items {
            // Include Trash and Root, and home directories if they exist or are home itself
            if path.to_string_lossy() == "trash:///" || path == *"/" || path == home || path.exists() {
                let row = Self::create_row(icon, name, path, icon_class);
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
            let home = glib::home_dir();
            let standard_place = ["", "Desktop", "Documents", "Downloads", "Music", "Pictures", "Videos"]
                .iter().any(|name| *bookmark == home.join(name));
            if bookmark.exists() && !standard_place {
                let name = bookmark
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| bookmark.to_string_lossy().to_string());

                let row = Self::create_row("folder-symbolic", &name, bookmark.clone(), "icon-bookmark");

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
        self.bookmarks_section.set_visible(self.bookmarks_list.first_child().is_some());
    }

    /// Show actual mounted local drives and update when devices change.
    fn populate_drives(&self) {
        let monitor = gio::VolumeMonitor::get();
        Self::update_drives(&self.drives_list, &monitor);
        let weak_list = self.drives_list.downgrade();
        let refresh = move |monitor: &gio::VolumeMonitor, _: &gio::Mount| {
            if let Some(list) = weak_list.upgrade() {
                Self::update_drives(&list, monitor);
            }
        };
        let handlers = RefCell::new(vec![
            monitor.connect_mount_added(refresh.clone()),
            monitor.connect_mount_removed(refresh.clone()),
            monitor.connect_mount_changed(refresh),
        ]);
        self.drives_list.connect_destroy(move |_| {
            for handler in handlers.borrow_mut().drain(..) {
                monitor.disconnect(handler);
            }
        });
    }

    fn update_drives(list: &gtk4::ListBox, monitor: &gio::VolumeMonitor) {
        let selected_path = list.selected_row().map(|row| row.widget_name());
        while let Some(child) = list.first_child() {
            list.remove(&child);
        }
        list.append(&Self::create_row(
            "drive-harddisk-symbolic", "File System", PathBuf::from("/"), "icon-drive",
        ));
        for mount in monitor.mounts() {
            if mount.is_shadowed() { continue; }
            if let Some(path) = mount.root().path() {
                if path == Path::new("/") { continue; }
                let row = Self::create_row(
                    "drive-removable-media-symbolic", &mount.name(), path, "icon-drive",
                );
                list.append(&row);
            }
        }
        let mut child = list.first_child();
        while let Some(widget) = child {
            if let Some(row) = widget.downcast_ref::<gtk4::ListBoxRow>()
                && selected_path.as_ref() == Some(&row.widget_name()) {
                list.select_row(Some(row));
            }
            child = widget.next_sibling();
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
        let mut matched = false;
        for list in [&self.places_list, &self.bookmarks_list, &self.drives_list] {
            list.unselect_all();
            let mut child = list.first_child();
            while let Some(row) = child {
                if !matched
                    && let Some(r) = row.downcast_ref::<gtk4::ListBoxRow>()
                    && r.widget_name().as_str() == current_str
                {
                    list.select_row(Some(r));
                    matched = true;
                }
                child = row.next_sibling();
            }
        }
    }
}
