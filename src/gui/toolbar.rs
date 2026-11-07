//! HeaderBar and navigation controls for Shark file manager.

use crate::config::Config;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

type NavigateCallback = Rc<RefCell<Option<Box<dyn Fn(PathBuf)>>>>;
type SortCallback = Rc<RefCell<Option<Box<dyn Fn(String, bool)>>>>;
type ViewCallback = Rc<RefCell<Option<Box<dyn Fn(String)>>>>;
type BoolCallback = Rc<RefCell<Option<Box<dyn Fn(bool)>>>>;
type ActionCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;

#[derive(Clone)]
pub struct Toolbar {
    pub header_bar: gtk4::HeaderBar,
    pub folder_title: gtk4::Label,
    pub back_btn: gtk4::Button,
    pub forward_btn: gtk4::Button,
    pub up_btn: gtk4::Button,
    pub refresh_btn: gtk4::Button,
    pub search_btn: gtk4::ToggleButton,
    pub new_folder_btn: gtk4::Button,
    pub view_grid_btn: gtk4::ToggleButton,
    pub view_list_btn: gtk4::ToggleButton,
    _sort_btn: gtk4::MenuButton,
    _menu_btn: gtk4::MenuButton,
    breadcrumb_box: gtk4::Box,
    location_entry: gtk4::Entry,
    path_stack: gtk4::Stack,
    location_toggle_btn: gtk4::Button,
    current_path: Rc<RefCell<PathBuf>>,
    on_navigate: NavigateCallback,
    on_sort_changed: SortCallback,
    on_view_changed: ViewCallback,
    on_show_hidden_toggled: BoolCallback,
    on_bookmark_current: ActionCallback,
    on_new_folder: ActionCallback,
    on_about: ActionCallback,
}

impl Toolbar {
    pub fn new(config: Rc<RefCell<Config>>) -> Self {
        let header_bar = gtk4::HeaderBar::builder()
            .show_title_buttons(true)
            .css_classes(["toolbar"])
            .build();

        let brand = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
        brand.set_width_request(200);
        brand.add_css_class("toolbar-brand");
        let brand_icon = gtk4::Image::from_icon_name("system-file-manager-symbolic");
        brand_icon.set_pixel_size(18);
        brand.append(&brand_icon);
        brand.append(&gtk4::Label::new(Some("Shark")));
        header_bar.pack_start(&brand);

        // --- Left Box (Nav buttons) ---
        let left_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        left_box.set_valign(gtk4::Align::Center);
        left_box.add_css_class("nav-pill");

        let back_btn = gtk4::Button::from_icon_name("go-previous-symbolic");
        back_btn.set_tooltip_text(Some("Back (Alt+Left)"));
        back_btn.set_sensitive(false);
        back_btn.add_css_class("nav-btn");

        let forward_btn = gtk4::Button::from_icon_name("go-next-symbolic");
        forward_btn.set_tooltip_text(Some("Forward (Alt+Right)"));
        forward_btn.set_sensitive(false);
        forward_btn.add_css_class("nav-btn");

        let up_btn = gtk4::Button::from_icon_name("go-up-symbolic");
        up_btn.set_tooltip_text(Some("Parent Folder (Alt+Up)"));
        up_btn.set_sensitive(false);
        up_btn.add_css_class("nav-btn");

        let refresh_btn = gtk4::Button::from_icon_name("view-refresh-symbolic");
        refresh_btn.set_tooltip_text(Some("Reload (Ctrl+R)"));
        refresh_btn.add_css_class("nav-btn");

        left_box.append(&back_btn);
        left_box.append(&forward_btn);

        header_bar.pack_start(&left_box);

        // --- Center Title Widget (Breadcrumbs & Direct Location Entry) ---
        let center_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(4)
            .valign(gtk4::Align::Center)
            .css_classes(["breadcrumb-container"])
            .build();

        let path_stack = gtk4::Stack::builder()
            .hexpand(true)
            .transition_type(gtk4::StackTransitionType::Crossfade)
            .build();

        let breadcrumb_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(2)
            .build();

        let location_entry = gtk4::Entry::builder()
            .placeholder_text("Enter location...")
            .width_chars(16)
            .hexpand(true)
            .build();

        path_stack.add_named(&breadcrumb_box, Some("breadcrumbs"));
        path_stack.add_named(&location_entry, Some("entry"));

        let location_toggle_btn = gtk4::Button::from_icon_name("document-edit-symbolic");
        location_toggle_btn.set_tooltip_text(Some("Edit Location (Ctrl+L)"));
        location_toggle_btn.add_css_class("flat");

        let path_scroll = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::External)
            .vscrollbar_policy(gtk4::PolicyType::Never)
            .min_content_width(120)
            .max_content_width(480)
            .propagate_natural_width(true)
            .child(&path_stack)
            .build();
        center_box.append(&path_scroll);
        center_box.append(&location_toggle_btn);

        header_bar.set_title_widget(Some(&center_box));

        // --- Right Box (Search, New Folder, View Mode, Sort, Menu) ---
        let right_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        right_box.set_valign(gtk4::Align::Center);

        let search_btn = gtk4::ToggleButton::builder()
            .icon_name("edit-find-symbolic")
            .tooltip_text("Search (Ctrl+F)")
            .build();

        let new_folder_btn = gtk4::Button::builder()
            .icon_name("folder-new-symbolic")
            .tooltip_text("New Folder (Ctrl+Shift+N)")
            .build();

        // View switcher (Grid vs List)
        let view_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .valign(gtk4::Align::Center)
            .css_classes(["view-switcher"])
            .build();

        let view_grid_btn = gtk4::ToggleButton::builder()
            .icon_name("view-grid-symbolic")
            .tooltip_text("Grid View")
            .active(config.borrow().view_mode == "grid")
            .build();

        let view_list_btn = gtk4::ToggleButton::builder()
            .icon_name("view-list-symbolic")
            .tooltip_text("List View")
            .active(config.borrow().view_mode == "list")
            .build();

        view_list_btn.set_group(Some(&view_grid_btn));

        view_box.append(&view_grid_btn);
        view_box.append(&view_list_btn);

        // Sort menu button
        let sort_btn = gtk4::MenuButton::builder()
            .icon_name("view-sort-ascending-symbolic")
            .tooltip_text("Sort Options")
            .build();

        let sort_popover = gtk4::Popover::new();
        let sort_menu_box = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
        sort_menu_box.set_margin_start(8);
        sort_menu_box.set_margin_end(8);
        sort_menu_box.set_margin_top(8);
        sort_menu_box.set_margin_bottom(8);

        let sort_label = gtk4::Label::builder()
            .label("Sort by")
            .halign(gtk4::Align::Start)
            .css_classes(["heading"])
            .build();
        sort_menu_box.append(&sort_label);

        let make_sort_item = |name: &'static str, id: &'static str| {
            let btn = gtk4::Button::builder()
                .label(name)
                .halign(gtk4::Align::Fill)
                .css_classes(["flat"])
                .build();
            btn.set_widget_name(id);
            btn
        };

        let btn_name = make_sort_item("Name", "name");
        let btn_size = make_sort_item("Size", "size");
        let btn_date = make_sort_item("Modified Date", "modified");
        let btn_type = make_sort_item("File Type", "type");

        let rev_check = gtk4::CheckButton::builder()
            .label("Descending Order")
            .active(config.borrow().sort_reversed)
            .margin_top(6)
            .build();

        sort_menu_box.append(&btn_name);
        sort_menu_box.append(&btn_size);
        sort_menu_box.append(&btn_date);
        sort_menu_box.append(&btn_type);
        sort_menu_box.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
        sort_menu_box.append(&rev_check);

        sort_popover.set_child(Some(&sort_menu_box));
        sort_btn.set_popover(Some(&sort_popover));

        // Main App Menu button
        let menu_btn = gtk4::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .tooltip_text("Main Menu")
            .build();

        let app_popover = gtk4::Popover::new();
        let app_menu_box = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
        app_menu_box.set_margin_start(8);
        app_menu_box.set_margin_end(8);
        app_menu_box.set_margin_top(8);
        app_menu_box.set_margin_bottom(8);

        let hidden_check = gtk4::CheckButton::builder()
            .label("Show Hidden Files (Ctrl+H)")
            .active(config.borrow().show_hidden)
            .build();

        let bookmark_btn = gtk4::Button::builder()
            .label("Bookmark Current Folder")
            .css_classes(["flat"])
            .halign(gtk4::Align::Fill)
            .build();

        let about_btn = gtk4::Button::builder()
            .label("About Shark")
            .css_classes(["flat"])
            .halign(gtk4::Align::Fill)
            .build();

        up_btn.set_label("Parent Folder (Alt+Up)");
        refresh_btn.set_label("Reload (Ctrl+R)");
        app_menu_box.append(&up_btn);
        app_menu_box.append(&refresh_btn);
        app_menu_box.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
        app_menu_box.append(&hidden_check);
        app_menu_box.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
        app_menu_box.append(&bookmark_btn);
        app_menu_box.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
        app_menu_box.append(&about_btn);

        app_popover.set_child(Some(&app_menu_box));
        menu_btn.set_popover(Some(&app_popover));

        right_box.append(&search_btn);
        right_box.append(&new_folder_btn);
        right_box.append(&view_box);
        right_box.append(&sort_btn);
        right_box.append(&menu_btn);

        header_bar.pack_end(&right_box);

        let current_path = Rc::new(RefCell::new(glib::home_dir()));
        let on_navigate = Rc::new(RefCell::new(None));
        let on_sort_changed = Rc::new(RefCell::new(None));
        let on_view_changed = Rc::new(RefCell::new(None));
        let on_show_hidden_toggled = Rc::new(RefCell::new(None));
        let on_bookmark_current = Rc::new(RefCell::new(None));
        let on_new_folder = Rc::new(RefCell::new(None));
        let on_about = Rc::new(RefCell::new(None));

        let folder_title = gtk4::Label::builder()
            .halign(gtk4::Align::Fill)
            .xalign(0.0)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .hexpand(true)
            .css_classes(["folder-title"])
            .build();
        let toolbar = Self {
            folder_title,
            header_bar,
            back_btn,
            forward_btn,
            up_btn,
            refresh_btn,
            search_btn,
            new_folder_btn,
            view_grid_btn,
            view_list_btn,
            _sort_btn: sort_btn,
            _menu_btn: menu_btn,
            breadcrumb_box,
            location_entry,
            path_stack,
            location_toggle_btn,
            current_path,
            on_navigate,
            on_sort_changed,
            on_view_changed,
            on_show_hidden_toggled,
            on_bookmark_current,
            on_new_folder,
            on_about,
        };

        toolbar.setup_callbacks(
            config,
            btn_name,
            btn_size,
            btn_date,
            btn_type,
            rev_check,
            sort_popover,
            hidden_check,
            bookmark_btn,
            about_btn,
            app_popover,
        );

        toolbar
    }

    /// Set up internal signal connections.
    #[allow(clippy::too_many_arguments)]
    fn setup_callbacks(
        &self,
        config: Rc<RefCell<Config>>,
        btn_name: gtk4::Button,
        btn_size: gtk4::Button,
        btn_date: gtk4::Button,
        btn_type: gtk4::Button,
        rev_check: gtk4::CheckButton,
        sort_popover: gtk4::Popover,
        hidden_check: gtk4::CheckButton,
        bookmark_btn: gtk4::Button,
        about_btn: gtk4::Button,
        app_popover: gtk4::Popover,
    ) {
        // Location toggle button
        let path_stack = self.path_stack.clone();
        let loc_entry = self.location_entry.clone();
        let cur_path = self.current_path.clone();
        self.location_toggle_btn.connect_clicked(move |_| {
            if path_stack.visible_child_name().as_deref() == Some("entry") {
                path_stack.set_visible_child_name("breadcrumbs");
            } else {
                loc_entry.set_text(&cur_path.borrow().to_string_lossy());
                path_stack.set_visible_child_name("entry");
                loc_entry.grab_focus();
                loc_entry.select_region(0, -1);
            }
        });

        // Location entry activated (Enter pressed)
        let path_stack_c = self.path_stack.clone();
        let on_nav = self.on_navigate.clone();
        self.location_entry.connect_activate(move |entry| {
            let text = entry.text().to_string();
            let path = PathBuf::from(&text);
            if path.is_dir() {
                path_stack_c.set_visible_child_name("breadcrumbs");
                if let Some(ref cb) = *on_nav.borrow() {
                    cb(path);
                }
            } else {
                entry.add_css_class("error");
            }
        });

        self.location_entry.connect_changed(|entry| entry.remove_css_class("error"));

        // View mode toggles
        let list_btn = self.view_list_btn.clone();
        let on_vc = self.on_view_changed.clone();
        let cfg_c = config.clone();

        self.view_grid_btn.connect_toggled(move |btn| {
            if btn.is_active() {
                list_btn.set_active(false);
                cfg_c.borrow_mut().view_mode = "grid".to_string();
                cfg_c.borrow().save();
                if let Some(ref cb) = *on_vc.borrow() {
                    cb("grid".to_string());
                }
            }
        });

        let grid_btn2 = self.view_grid_btn.clone();
        let on_vc2 = self.on_view_changed.clone();
        let cfg_c2 = config.clone();

        self.view_list_btn.connect_toggled(move |btn| {
            if btn.is_active() {
                grid_btn2.set_active(false);
                cfg_c2.borrow_mut().view_mode = "list".to_string();
                cfg_c2.borrow().save();
                if let Some(ref cb) = *on_vc2.borrow() {
                    cb("list".to_string());
                }
            }
        });

        // Sort items
        let sort_buttons = [btn_name, btn_size, btn_date, btn_type];
        for btn in sort_buttons {
            let on_sc = self.on_sort_changed.clone();
            let pop = sort_popover.clone();
            let rev_c = rev_check.clone();
            let cfg = config.clone();

            btn.connect_clicked(move |b| {
                pop.popdown();
                let id_str = b.widget_name().to_string();
                let reversed = rev_c.is_active();
                cfg.borrow_mut().sort_by = id_str.clone();
                cfg.borrow_mut().sort_reversed = reversed;
                cfg.borrow().save();
                if let Some(ref cb) = *on_sc.borrow() {
                    cb(id_str, reversed);
                }
            });
        }

        let on_sc_rev = self.on_sort_changed.clone();
        let cfg_rev = config.clone();
        rev_check.connect_toggled(move |c| {
            let reversed = c.is_active();
            let current_sort = cfg_rev.borrow().sort_by.clone();
            cfg_rev.borrow_mut().sort_reversed = reversed;
            cfg_rev.borrow().save();
            if let Some(ref cb) = *on_sc_rev.borrow() {
                cb(current_sort, reversed);
            }
        });

        // Hidden files toggle
        let on_sh = self.on_show_hidden_toggled.clone();
        let cfg_sh = config.clone();
        hidden_check.connect_toggled(move |c| {
            let show = c.is_active();
            cfg_sh.borrow_mut().show_hidden = show;
            cfg_sh.borrow().save();
            if let Some(ref cb) = *on_sh.borrow() {
                cb(show);
            }
        });

        // Bookmark current
        let on_bm = self.on_bookmark_current.clone();
        let pop_bm = app_popover.clone();
        bookmark_btn.connect_clicked(move |_| {
            pop_bm.popdown();
            if let Some(ref cb) = *on_bm.borrow() {
                cb();
            }
        });

        // About
        let on_ab = self.on_about.clone();
        let pop_ab = app_popover;
        about_btn.connect_clicked(move |_| {
            pop_ab.popdown();
            if let Some(ref cb) = *on_ab.borrow() {
                cb();
            }
        });

        // New folder
        let on_nf = self.on_new_folder.clone();
        self.new_folder_btn.connect_clicked(move |_| {
            if let Some(ref cb) = *on_nf.borrow() {
                cb();
            }
        });
    }

    /// Update breadcrumb path buttons.
    pub fn set_path(&self, path: &Path) {
        self.folder_title.set_text(if path == glib::home_dir() {
            "Home"
        } else if path == Path::new("/") {
            "File System"
        } else {
            path.file_name().and_then(|s| s.to_str()).unwrap_or("Files")
        });
        *self.current_path.borrow_mut() = path.to_path_buf();

        // Clear existing breadcrumbs
        while let Some(child) = self.breadcrumb_box.first_child() {
            self.breadcrumb_box.remove(&child);
        }

        let home = glib::home_dir();
        let mut segments: Vec<(PathBuf, String)> = Vec::new();

        if path.starts_with(&home) {
            segments.push((home.clone(), "Home".to_string()));
            if let Ok(rel) = path.strip_prefix(&home) {
                let mut accum = home.clone();
                for comp in rel.components() {
                    let s = comp.as_os_str().to_string_lossy().to_string();
                    if !s.is_empty() {
                        accum.push(&s);
                        segments.push((accum.clone(), s));
                    }
                }
            }
        } else {
            let mut cur = Some(path);
            let mut raw_segs = Vec::new();
            while let Some(p) = cur {
                raw_segs.push(p.to_path_buf());
                cur = p.parent();
            }
            raw_segs.reverse();
            for seg in raw_segs {
                let name = if seg == Path::new("/") {
                    "System".to_string()
                } else {
                    seg.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Root".to_string())
                };
                segments.push((seg, name));
            }
        }

        let total = segments.len();
        for (i, (segment, label)) in segments.into_iter().enumerate() {
            let btn = gtk4::Button::builder()
                .label(&label)
                .css_classes(["breadcrumb-button"])
                .build();

            if i + 1 == total {
                btn.add_css_class("breadcrumb-active");
            }

            let on_nav = self.on_navigate.clone();
            let seg_clone = segment.clone();
            btn.connect_clicked(move |_| {
                if let Some(ref cb) = *on_nav.borrow() {
                    cb(seg_clone.clone());
                }
            });

            self.breadcrumb_box.append(&btn);

            if i + 1 < total {
                let sep = gtk4::Label::builder()
                    .label("›")
                    .css_classes(["breadcrumb-separator"])
                    .build();
                self.breadcrumb_box.append(&sep);
            }
        }

        // Ensure visible child is breadcrumbs
        self.path_stack.set_visible_child_name("breadcrumbs");
    }

    /// Focus location entry for typing (Ctrl+L).
    pub fn focus_location_entry(&self) {
        self.location_entry.set_text(&self.current_path.borrow().to_string_lossy());
        self.path_stack.set_visible_child_name("entry");
        self.location_entry.grab_focus();
        self.location_entry.select_region(0, -1);
    }

    /// Connect navigation callback.
    pub fn connect_navigate<F: Fn(PathBuf) + 'static>(&self, f: F) {
        *self.on_navigate.borrow_mut() = Some(Box::new(f));
    }

    /// Connect sort change callback.
    pub fn connect_sort_changed<F: Fn(String, bool) + 'static>(&self, f: F) {
        *self.on_sort_changed.borrow_mut() = Some(Box::new(f));
    }

    /// Connect view change callback.
    pub fn connect_view_changed<F: Fn(String) + 'static>(&self, f: F) {
        *self.on_view_changed.borrow_mut() = Some(Box::new(f));
    }

    /// Connect show hidden toggled callback.
    pub fn connect_show_hidden_toggled<F: Fn(bool) + 'static>(&self, f: F) {
        *self.on_show_hidden_toggled.borrow_mut() = Some(Box::new(f));
    }

    /// Connect bookmark current folder callback.
    pub fn connect_bookmark_current<F: Fn() + 'static>(&self, f: F) {
        *self.on_bookmark_current.borrow_mut() = Some(Box::new(f));
    }

    /// Connect new folder callback.
    pub fn connect_new_folder<F: Fn() + 'static>(&self, f: F) {
        *self.on_new_folder.borrow_mut() = Some(Box::new(f));
    }

    /// Connect about dialog callback.
    pub fn connect_about<F: Fn() + 'static>(&self, f: F) {
        *self.on_about.borrow_mut() = Some(Box::new(f));
    }
}
