//! Main application window assembling HeaderBar, Sidebar, FileView, and status bar.

use crate::config::Config;
use crate::filesystem::{
    copy_paths_async, create_file, create_folder, delete_paths_permanent, move_paths_async,
    rename_path, scan_directory_async, trash_paths, ConflictResolution,
    DirectoryMonitor, OpProgress, ScanMessage,
};
use crate::gui::clipboard::AppClipboard;
use crate::gui::dialogs::{
    show_about_dialog, show_create_file_dialog, show_create_folder_dialog,
    show_delete_confirmation, show_error_dialog, show_rename_dialog,
};
use crate::gui::file_view::FileView;
use crate::gui::properties_dialog::show_properties_dialog;
use crate::gui::sidebar::Sidebar;
use crate::gui::search_palette::SearchPalette;
use crate::gui::toolbar::Toolbar;
use crate::models::FileEntry;
use crate::navigation::NavigationHistory;
use gtk4::gdk;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct SharkWindow {
    pub window: gtk4::ApplicationWindow,
    pub toolbar: Toolbar,
    pub sidebar: Sidebar,
    pub file_view: FileView,
    history: Rc<RefCell<NavigationHistory>>,
    config: Rc<RefCell<Config>>,
    clipboard: Rc<RefCell<AppClipboard>>,
    current_scan_cancel: Rc<RefCell<Arc<AtomicBool>>>,
    monitor: Rc<RefCell<Option<DirectoryMonitor>>>,
    search_palette: Rc<SearchPalette>,
    progress_revealer: gtk4::Revealer,
    progress_bar: gtk4::ProgressBar,
    progress_label: gtk4::Label,
    empty_state_box: gtk4::Box,
    empty_state_title: gtk4::Label,
    empty_state_sub: gtk4::Label,
    status_items_label: gtk4::Label,
    status_space_label: gtk4::Label,
    active_popover: Rc<RefCell<Option<gtk4::Popover>>>,
}

impl SharkWindow {
    pub fn new(app: &gtk4::Application) -> Rc<Self> {
        let config = Rc::new(RefCell::new(Config::load()));
        let initial_dir = config.borrow().current_directory();

        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("Shark File Manager")
            .default_width(config.borrow().window_width)
            .default_height(config.borrow().window_height)
            .build();

        let toolbar = Toolbar::new(config.clone());
        window.set_titlebar(Some(&toolbar.header_bar));
        let sidebar = Sidebar::new(config.clone());
        let file_view = FileView::new();

        file_view.set_show_hidden(config.borrow().show_hidden);
        file_view.set_view_mode(&config.borrow().view_mode);
        file_view.set_sort(&config.borrow().sort_by, config.borrow().sort_reversed);

        let search_palette = SearchPalette::new(&window);

        // Progress Bar Revealer (for background copy/move/trash)
        let progress_revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::SlideDown)
            .reveal_child(false)
            .build();

        let progress_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .css_classes(["progress-banner"])
            .build();

        let progress_label = gtk4::Label::builder()
            .label("Processing...")
            .halign(gtk4::Align::Start)
            .build();

        let progress_bar = gtk4::ProgressBar::builder()
            .hexpand(true)
            .show_text(false)
            .build();

        progress_box.append(&progress_label);
        progress_box.append(&progress_bar);
        progress_revealer.set_child(Some(&progress_box));

        // Empty State Box (overlay when directory has no entries)
        let empty_state_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .hexpand(true)
            .vexpand(true)
            .visible(false)
            .build();

        let empty_icon = gtk4::Image::from_icon_name("folder-symbolic");
        empty_icon.set_pixel_size(64);
        empty_icon.add_css_class("dim-label");

        let empty_state_title = gtk4::Label::builder()
            .label("Folder is Empty")
            .css_classes(["empty-state-title"])
            .build();

        let empty_state_sub = gtk4::Label::builder()
            .label("No files or folders in this directory")
            .css_classes(["empty-state-subtitle"])
            .build();

        empty_state_box.append(&empty_icon);
        empty_state_box.append(&empty_state_title);
        empty_state_box.append(&empty_state_sub);

        // Center Overlay holding FileView + EmptyState
        let view_overlay = gtk4::Overlay::new();
        view_overlay.set_child(Some(&file_view.container));
        view_overlay.add_overlay(&empty_state_box);

        // Main content column
        let content_column = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .hexpand(true)
            .vexpand(true)
            .build();

        content_column.append(&progress_revealer);
        content_column.append(&toolbar.folder_title);
        content_column.append(&view_overlay);

        // Status Bar at Bottom
        let status_bar = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(16)
            .css_classes(["status-bar"])
            .build();

        let status_items_label = gtk4::Label::builder()
            .label("0 items")
            .halign(gtk4::Align::Start)
            .hexpand(true)
            .build();

        let status_space_label = gtk4::Label::builder()
            .label("")
            .halign(gtk4::Align::End)
            .build();

        status_bar.append(&status_items_label);
        status_bar.append(&status_space_label);
        content_column.append(&status_bar);

        // Paned (Sidebar on left, Main content on right)
        let paned = gtk4::Paned::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .resize_start_child(false)
            .start_child(&sidebar.container)
            .end_child(&content_column)
            .position(240)
            .shrink_start_child(false)
            .shrink_end_child(false)
            .hexpand(true)
            .vexpand(true)
            .build();

        window.set_child(Some(&paned));

        let history = Rc::new(RefCell::new(NavigationHistory::new(initial_dir.clone())));
        let clipboard = Rc::new(RefCell::new(AppClipboard::new()));
        let current_scan_cancel = Rc::new(RefCell::new(Arc::new(AtomicBool::new(false))));
        let monitor = Rc::new(RefCell::new(None));
        let active_popover = Rc::new(RefCell::new(None));

        let shark = Rc::new(Self {
            window,
            toolbar,
            sidebar,
            file_view,
            history,
            config,
            clipboard,
            current_scan_cancel,
            monitor,
            search_palette,
            progress_revealer,
            progress_bar,
            progress_label,
            empty_state_box,
            empty_state_title,
            empty_state_sub,
            status_items_label,
            status_space_label,
            active_popover,
        });

        shark.apply_background_mode(&shark.config.borrow().background_mode);
        shark.setup_bindings();
        shark.setup_key_shortcuts();

        // Initial navigation
        shark.navigate_to(initial_dir);

        shark
    }

    /// Wire all UI signals, callbacks and event listeners.
    fn setup_bindings(self: &Rc<Self>) {
        // Navigation buttons
        let shark_b = self.clone();
        self.toolbar.back_btn.connect_clicked(move |_| {
            shark_b.go_back();
        });

        let shark_f = self.clone();
        self.toolbar.forward_btn.connect_clicked(move |_| {
            shark_f.go_forward();
        });

        let shark_u = self.clone();
        self.toolbar.up_btn.connect_clicked(move |_| {
            shark_u.go_up();
        });

        let shark_r = self.clone();
        self.toolbar.refresh_btn.connect_clicked(move |_| {
            shark_r.reload_current_directory();
        });

        // Breadcrumb and direct location navigation
        let shark_n = self.clone();
        self.toolbar.connect_navigate(move |path| {
            shark_n.navigate_to(path);
        });

        // Sidebar navigation
        let shark_sn = self.clone();
        self.sidebar.connect_navigate(move |path| {
            shark_sn.navigate_to(path);
        });

        // Bookmarks changed
        let shark_bc = self.clone();
        self.sidebar.connect_bookmarks_changed(move || {
            shark_bc.sidebar.populate_bookmarks();
        });

        // Bookmark current
        let shark_bmc = self.clone();
        self.toolbar.connect_bookmark_current(move || {
            let cur = shark_bmc.history.borrow().current().to_path_buf();
            shark_bmc.config.borrow_mut().add_bookmark(cur);
            shark_bmc.sidebar.populate_bookmarks();
        });

        // View mode toggle
        let shark_vm = self.clone();
        self.toolbar.connect_view_changed(move |mode| {
            shark_vm.file_view.set_view_mode(&mode);
        });

        // Sort changed
        let shark_sc = self.clone();
        self.toolbar.connect_sort_changed(move |sort_by, reversed| {
            shark_sc.file_view.set_sort(&sort_by, reversed);
        });

        // Sort requested from file list view column headers
        let shark_sr = self.clone();
        self.file_view.connect_sort_request(move |sort_by, reversed| {
            shark_sr.update_sort(&sort_by, reversed);
        });

        // Show hidden toggle
        let shark_sh = self.clone();
        self.toolbar.connect_show_hidden_toggled(move |show| {
            shark_sh.file_view.set_show_hidden(show);
            shark_sh.update_status_bar();
        });

        // New folder button
        let shark_nf = self.clone();
        self.toolbar.connect_new_folder(move || {
            shark_nf.handle_new_folder();
        });

        // About dialog
        let shark_ab = self.clone();
        self.toolbar.connect_about(move || {
            show_about_dialog(&shark_ab.window);
        });

        let weak = Rc::downgrade(self);
        self.toolbar.search_btn.connect_toggled(move |button| {
            if let Some(shark) = weak.upgrade() {
                if button.is_active() {
                    shark.search_palette.show(
                        shark.history.borrow().current().to_path_buf(),
                        shark.config.borrow().show_hidden,
                    );
                } else { shark.search_palette.hide(); }
            }
        });
        let button = self.toolbar.search_btn.clone();
        self.search_palette.window.connect_hide(move |_| button.set_active(false));
        let weak = Rc::downgrade(self);
        self.search_palette.connect_activate(move |entry| {
            if let Some(shark) = weak.upgrade() { shark.on_item_activated(entry); }
        });

        // File activation (Double-click or Enter)
        let shark_act = self.clone();
        self.file_view.connect_activate(move |entry| {
            shark_act.on_item_activated(entry);
        });

        // Context menu (Right-click)
        let shark_cm = self.clone();
        self.file_view.connect_context_menu(move |target, x, y| {
            shark_cm.show_context_menu(target, x, y);
        });

        // Selection changed -> update status bar
        let shark_sel = self.clone();
        self.file_view.selection_model.connect_selection_changed(move |_, _, _| {
            shark_sel.update_status_bar();
        });

        // Background mode changes
        let shark_bg = self.clone();
        self.toolbar.connect_background_mode_changed(move |mode| {
            shark_bg.apply_background_mode(&mode);
        });

        // Window size changed on close -> save geometry
        let config_geom = self.config.clone();
        let win_geom = self.window.clone();
        self.window.connect_close_request(move |_| {
            let mut cfg = config_geom.borrow_mut();
            cfg.window_width = win_geom.width();
            cfg.window_height = win_geom.height();
            cfg.save();
            glib::Propagation::Proceed
        });
    }

    /// Set up keyboard shortcuts.
    fn setup_key_shortcuts(self: &Rc<Self>) {
        let key_controller = gtk4::EventControllerKey::new();
        let shark_k = self.clone();

        key_controller.connect_key_pressed(move |_, key, _, state| {
            let ctrl = state.contains(gdk::ModifierType::CONTROL_MASK);
            let alt = state.contains(gdk::ModifierType::ALT_MASK);
            let shift = state.contains(gdk::ModifierType::SHIFT_MASK);

            let is_editing = GtkWindowExt::focus(&shark_k.window)
                .map(|w| w.is::<gtk4::Editable>() || w.has_css_class("entry") || w.has_css_class("searchentry"))
                .unwrap_or(false);

            match key {
                // Return / Enter: Open selected item (when not editing text)
                gdk::Key::Return | gdk::Key::KP_Enter if !is_editing && !alt => {
                    let selected = shark_k.file_view.get_selected_entries();
                    if let Some(first) = selected.into_iter().next() {
                        shark_k.on_item_activated(first);
                        return glib::Propagation::Stop;
                    }
                    glib::Propagation::Proceed
                }
                // Alt + Enter or Ctrl + I: Properties
                gdk::Key::Return | gdk::Key::KP_Enter if alt => {
                    shark_k.handle_properties();
                    glib::Propagation::Stop
                }
                gdk::Key::i | gdk::Key::I if ctrl => {
                    shark_k.handle_properties();
                    glib::Propagation::Stop
                }
                // Ctrl + 1: Grid View
                gdk::Key::_1 | gdk::Key::KP_1 if ctrl => {
                    shark_k.toolbar.view_grid_btn.set_active(true);
                    glib::Propagation::Stop
                }
                // Ctrl + 2: List View
                gdk::Key::_2 | gdk::Key::KP_2 if ctrl => {
                    shark_k.toolbar.view_list_btn.set_active(true);
                    glib::Propagation::Stop
                }
                // Ctrl + K or Ctrl + P: Toggle command/search palette
                gdk::Key::k | gdk::Key::K | gdk::Key::p | gdk::Key::P if ctrl => {
                    let is_active = shark_k.toolbar.search_btn.is_active();
                    shark_k.toolbar.search_btn.set_active(!is_active);
                    glib::Propagation::Stop
                }
                // Ctrl + H: Toggle hidden
                gdk::Key::h | gdk::Key::H if ctrl => {
                    let current = shark_k.config.borrow().show_hidden;
                    shark_k.toolbar.set_path(shark_k.history.borrow().current());
                    shark_k.config.borrow_mut().show_hidden = !current;
                    shark_k.config.borrow().save();
                    shark_k.file_view.set_show_hidden(!current);
                    shark_k.update_status_bar();
                    glib::Propagation::Stop
                }
                // Ctrl + F: Search
                gdk::Key::f | gdk::Key::F if ctrl => {
                    let is_active = shark_k.toolbar.search_btn.is_active();
                    shark_k.toolbar.search_btn.set_active(!is_active);
                    glib::Propagation::Stop
                }
                // Ctrl + L or Alt + D: Focus location entry
                gdk::Key::l | gdk::Key::L if ctrl => {
                    shark_k.toolbar.focus_location_entry();
                    glib::Propagation::Stop
                }
                gdk::Key::d | gdk::Key::D if alt => {
                    shark_k.toolbar.focus_location_entry();
                    glib::Propagation::Stop
                }
                // Ctrl + D: Bookmark current folder
                gdk::Key::d | gdk::Key::D if ctrl => {
                    let cur = shark_k.history.borrow().current().to_path_buf();
                    shark_k.config.borrow_mut().add_bookmark(&cur);
                    shark_k.sidebar.populate_bookmarks();
                    glib::Propagation::Stop
                }
                // Ctrl + C: Copy
                gdk::Key::c | gdk::Key::C if ctrl => {
                    shark_k.handle_copy();
                    glib::Propagation::Stop
                }
                // Ctrl + X: Cut
                gdk::Key::x | gdk::Key::X if ctrl => {
                    shark_k.handle_cut();
                    glib::Propagation::Stop
                }
                // Ctrl + V: Paste
                gdk::Key::v | gdk::Key::V if ctrl => {
                    shark_k.handle_paste();
                    glib::Propagation::Stop
                }
                // Ctrl + A: Select All
                gdk::Key::a | gdk::Key::A if ctrl => {
                    shark_k.file_view.select_all();
                    glib::Propagation::Stop
                }
                // Delete: Trash (or Shift+Delete: Permanent)
                gdk::Key::Delete if !is_editing => {
                    shark_k.handle_delete(shift);
                    glib::Propagation::Stop
                }
                // F2: Rename
                gdk::Key::F2 => {
                    shark_k.handle_rename();
                    glib::Propagation::Stop
                }
                // Ctrl + Shift + S: Toggle Sort Order (Ascending / Descending)
                gdk::Key::s | gdk::Key::S if ctrl && shift => {
                    let cur_sort = shark_k.config.borrow().sort_by.clone();
                    let cur_rev = shark_k.config.borrow().sort_reversed;
                    shark_k.update_sort(&cur_sort, !cur_rev);
                    glib::Propagation::Stop
                }
                // Zoom In: Ctrl + Plus / Ctrl + Equal / Ctrl + KP_Add
                gdk::Key::plus | gdk::Key::equal | gdk::Key::KP_Add if ctrl => {
                    shark_k.file_view.zoom_in();
                    glib::Propagation::Stop
                }
                // Zoom Out: Ctrl + Minus / Ctrl + Underscore / Ctrl + KP_Subtract
                gdk::Key::minus | gdk::Key::underscore | gdk::Key::KP_Subtract if ctrl => {
                    shark_k.file_view.zoom_out();
                    glib::Propagation::Stop
                }
                // Zoom Reset: Ctrl + 0
                gdk::Key::_0 | gdk::Key::KP_0 if ctrl => {
                    shark_k.file_view.zoom_reset();
                    glib::Propagation::Stop
                }
                // Back: Alt + Left, BackSpace, Ctrl + [, or Multimedia Back
                gdk::Key::Left if alt => {
                    shark_k.go_back();
                    glib::Propagation::Stop
                }
                gdk::Key::bracketleft if ctrl => {
                    shark_k.go_back();
                    glib::Propagation::Stop
                }
                gdk::Key::Back => {
                    shark_k.go_back();
                    glib::Propagation::Stop
                }
                gdk::Key::BackSpace if !is_editing && !shift => {
                    shark_k.go_back();
                    glib::Propagation::Stop
                }
                // Forward: Alt + Right, Shift + BackSpace, Ctrl + ], or Multimedia Forward
                gdk::Key::Right if alt => {
                    shark_k.go_forward();
                    glib::Propagation::Stop
                }
                gdk::Key::bracketright if ctrl => {
                    shark_k.go_forward();
                    glib::Propagation::Stop
                }
                gdk::Key::Forward => {
                    shark_k.go_forward();
                    glib::Propagation::Stop
                }
                gdk::Key::BackSpace if !is_editing && shift => {
                    shark_k.go_forward();
                    glib::Propagation::Stop
                }
                // Alt + Up: Parent folder
                gdk::Key::Up if alt => {
                    shark_k.go_up();
                    glib::Propagation::Stop
                }
                // Alt + Home: Navigate to Home
                gdk::Key::Home if alt => {
                    shark_k.navigate_to(glib::home_dir());
                    glib::Propagation::Stop
                }
                // Ctrl + R or F5: Refresh
                gdk::Key::r | gdk::Key::R if ctrl => {
                    shark_k.reload_current_directory();
                    glib::Propagation::Stop
                }
                gdk::Key::F5 => {
                    shark_k.reload_current_directory();
                    glib::Propagation::Stop
                }
                // Ctrl + Shift + N: New folder
                gdk::Key::n | gdk::Key::N if ctrl && shift => {
                    shark_k.handle_new_folder();
                    glib::Propagation::Stop
                }
                // Ctrl + N: New file
                gdk::Key::n | gdk::Key::N if ctrl && !shift => {
                    shark_k.handle_new_file();
                    glib::Propagation::Stop
                }
                // Ctrl + W or Ctrl + Q: Close window / Quit
                gdk::Key::w | gdk::Key::W | gdk::Key::q | gdk::Key::Q if ctrl => {
                    shark_k.window.close();
                    glib::Propagation::Stop
                }
                // F1: About / Help
                gdk::Key::F1 => {
                    crate::gui::dialogs::show_about_dialog(&shark_k.window);
                    glib::Propagation::Stop
                }
                // Escape: Cancel search or unselect
                gdk::Key::Escape => {
                    if shark_k.toolbar.search_btn.is_active() {
                        shark_k.toolbar.search_btn.set_active(false);
                    } else {
                        shark_k.file_view.unselect_all();
                    }
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });

        self.window.add_controller(key_controller);

        // Mouse Back/Forward navigation (Thumb buttons 8 and 9)
        let mouse_nav = gtk4::GestureClick::builder()
            .button(0)
            .build();
        let shark_mouse = self.clone();
        mouse_nav.connect_pressed(move |gesture, _n_press, _x, _y| {
            let button = gesture.current_button();
            if button == 8 {
                shark_mouse.go_back();
            } else if button == 9 {
                shark_mouse.go_forward();
            }
        });
        self.window.add_controller(mouse_nav);

        // Ctrl + Mouse Wheel Zoom
        let scroll_controller = gtk4::EventControllerScroll::new(gtk4::EventControllerScrollFlags::VERTICAL);
        let shark_scroll = self.clone();
        scroll_controller.connect_scroll(move |controller, _dx, dy| {
            let state = controller.current_event_state();
            if state.contains(gdk::ModifierType::CONTROL_MASK) {
                if dy < 0.0 {
                    shark_scroll.file_view.zoom_in();
                } else if dy > 0.0 {
                    shark_scroll.file_view.zoom_out();
                }
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        self.window.add_controller(scroll_controller);
    }

    /// Navigate to a new directory.
    pub fn navigate_to(self: &Rc<Self>, path: PathBuf) {
        if !path.exists() && path.to_string_lossy() != "trash:///" {
            show_error_dialog(&self.window, &format!("Directory does not exist: {:?}", path));
            return;
        }

        // Cancel running scan
        self.current_scan_cancel.borrow().store(true, Ordering::Relaxed);
        let new_cancel = Arc::new(AtomicBool::new(false));
        *self.current_scan_cancel.borrow_mut() = new_cancel.clone();

        self.history.borrow_mut().navigate_to(path.clone());
        self.config.borrow_mut().last_directory = Some(path.clone());
        self.config.borrow().save();

        self.toolbar.set_path(&path);
        self.sidebar.set_current_path(&path);

        // Update nav buttons
        let hist = self.history.borrow();
        self.toolbar.back_btn.set_sensitive(hist.can_go_back());
        self.toolbar.forward_btn.set_sensitive(hist.can_go_forward());
        self.toolbar.up_btn.set_sensitive(hist.can_go_up());
        drop(hist);

        self.file_view.clear();
        self.empty_state_box.set_visible(false);

        // Monitor directory changes
        let shark_m = self.clone();
        *self.monitor.borrow_mut() = DirectoryMonitor::new(&path, move || {
            shark_m.reload_current_directory();
        });

        // Start incremental directory scanning
        let (sender, receiver) = std::sync::mpsc::channel();
        let shark_rx = self.clone();

        glib::idle_add_local(move || {
            loop {
                match receiver.try_recv() {
                    Ok(ScanMessage::Batch(entries)) => {
                        shark_rx.file_view.add_batch(entries);
                        shark_rx.update_status_bar();
                    }
                    Ok(ScanMessage::Completed { total_items, .. }) => {
                        shark_rx.update_status_bar();
                        if total_items == 0 {
                            shark_rx.empty_state_title.set_text("Folder is Empty");
                            shark_rx.empty_state_sub.set_text("No files or folders in this directory");
                            shark_rx.empty_state_box.set_visible(true);
                        } else {
                            shark_rx.empty_state_box.set_visible(false);
                        }
                        return glib::ControlFlow::Break;
                    }
                    Ok(ScanMessage::Error(err)) => {
                        shark_rx.empty_state_title.set_text("Cannot Open Folder");
                        shark_rx.empty_state_sub.set_text(&err);
                        shark_rx.empty_state_box.set_visible(true);
                        return glib::ControlFlow::Break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        return glib::ControlFlow::Continue;
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        return glib::ControlFlow::Break;
                    }
                }
            }
        });

        scan_directory_async(path, new_cancel, sender);
    }

    /// Reload the current directory.
    pub fn reload_current_directory(self: &Rc<Self>) {
        let cur = self.history.borrow().current().to_path_buf();
        self.navigate_to(cur);
    }

    /// Go back in history.
    pub fn go_back(self: &Rc<Self>) {
        if let Some(prev) = self.history.borrow_mut().go_back() {
            self.navigate_internal(prev);
        }
    }

    /// Go forward in history.
    pub fn go_forward(self: &Rc<Self>) {
        if let Some(next) = self.history.borrow_mut().go_forward() {
            self.navigate_internal(next);
        }
    }

    /// Go to parent directory.
    pub fn go_up(self: &Rc<Self>) {
        if let Some(parent) = self.history.borrow().parent() {
            self.navigate_to(parent);
        }
    }

    /// Internal navigation without adding to history back stack.
    fn navigate_internal(self: &Rc<Self>, path: PathBuf) {
        self.current_scan_cancel.borrow().store(true, Ordering::Relaxed);
        let new_cancel = Arc::new(AtomicBool::new(false));
        *self.current_scan_cancel.borrow_mut() = new_cancel.clone();

        self.toolbar.set_path(&path);
        self.sidebar.set_current_path(&path);

        let hist = self.history.borrow();
        self.toolbar.back_btn.set_sensitive(hist.can_go_back());
        self.toolbar.forward_btn.set_sensitive(hist.can_go_forward());
        self.toolbar.up_btn.set_sensitive(hist.can_go_up());
        drop(hist);

        self.file_view.clear();
        self.empty_state_box.set_visible(false);

        let shark_m = self.clone();
        *self.monitor.borrow_mut() = DirectoryMonitor::new(&path, move || {
            shark_m.reload_current_directory();
        });

        let (sender, receiver) = std::sync::mpsc::channel();
        let shark_rx = self.clone();

        glib::idle_add_local(move || {
            loop {
                match receiver.try_recv() {
                    Ok(ScanMessage::Batch(entries)) => {
                        shark_rx.file_view.add_batch(entries);
                        shark_rx.update_status_bar();
                    }
                    Ok(ScanMessage::Completed { total_items, .. }) => {
                        shark_rx.update_status_bar();
                        if total_items == 0 {
                            shark_rx.empty_state_title.set_text("Folder is Empty");
                            shark_rx.empty_state_sub.set_text("No files or folders in this directory");
                            shark_rx.empty_state_box.set_visible(true);
                        } else {
                            shark_rx.empty_state_box.set_visible(false);
                        }
                        return glib::ControlFlow::Break;
                    }
                    Ok(ScanMessage::Error(err)) => {
                        shark_rx.empty_state_title.set_text("Cannot Open Folder");
                        shark_rx.empty_state_sub.set_text(&err);
                        shark_rx.empty_state_box.set_visible(true);
                        return glib::ControlFlow::Break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        return glib::ControlFlow::Continue;
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        return glib::ControlFlow::Break;
                    }
                }
            }
        });

        scan_directory_async(path, new_cancel, sender);
    }

    /// Item activated (double click or Enter).
    fn on_item_activated(self: &Rc<Self>, entry: FileEntry) {
        if entry.is_directory {
            self.navigate_to(entry.path);
        } else {
            // Open with Linux default application
            let uri = gio::File::for_path(&entry.path).uri();
            if gio::AppInfo::launch_default_for_uri(&uri, gio::AppLaunchContext::NONE).is_err() {
                // Fallback to xdg-open
                let _ = std::process::Command::new("xdg-open")
                    .arg(&entry.path)
                    .spawn();
            }
        }
    }

    /// Update status bar labels with item count, selection count, and free space.
    pub fn update_status_bar(&self) {
        let total = self.file_view.item_count();
        let selected = self.file_view.get_selected_entries();

        let items_text = if selected.is_empty() {
            format!("{} item{}", total, if total == 1 { "" } else { "s" })
        } else {
            let total_bytes: u64 = selected.iter().map(|e| e.size).sum();
            format!(
                "{} of {} selected ({})",
                selected.len(),
                total,
                crate::utils::format_size(total_bytes)
            )
        };

        self.status_items_label.set_text(&items_text);

        let cur = self.history.borrow().current().to_path_buf();
        if let Some(free_space) = crate::utils::get_disk_free_space(&cur) {
            self.status_space_label.set_text(&format!("{} free", free_space));
        } else {
            self.status_space_label.set_text("");
        }
    }

    /// Copy selected files to clipboard.
    fn handle_copy(&self) {
        let selected = self.file_view.get_selected_entries();
        if !selected.is_empty() {
            let paths = selected.into_iter().map(|e| e.path).collect();
            self.clipboard.borrow_mut().copy(paths);
        }
    }

    /// Cut selected files to clipboard.
    fn handle_cut(&self) {
        let selected = self.file_view.get_selected_entries();
        if !selected.is_empty() {
            let paths = selected.into_iter().map(|e| e.path).collect();
            self.clipboard.borrow_mut().cut(paths);
        }
    }

    /// Paste files from clipboard into current directory.
    fn handle_paste(self: &Rc<Self>) {
        let clipboard = self.clipboard.borrow();
        if clipboard.is_empty() {
            return;
        }

        let sources = clipboard.paths.clone();
        let is_cut = clipboard.is_cut;
        drop(clipboard);

        let cur_dir = self.history.borrow().current().to_path_buf();
        let cancel = Arc::new(AtomicBool::new(false));

        self.progress_revealer.set_reveal_child(true);
        self.progress_bar.set_fraction(0.0);
        self.progress_label.set_text(if is_cut { "Moving files..." } else { "Copying files..." });

        let (sender, receiver) = std::sync::mpsc::channel();
        let shark_p = self.clone();

        glib::idle_add_local(move || {
            while let Ok(msg) = receiver.try_recv() {
                match msg {
                    OpProgress::Progress {
                        items_done,
                        total_items,
                        current_item,
                        ..
                    } => {
                        let frac = if total_items > 0 {
                            items_done as f64 / total_items as f64
                        } else {
                            0.0
                        };
                        shark_p.progress_bar.set_fraction(frac);
                        shark_p.progress_label.set_text(&format!(
                            "Processing ({}/{}): {}",
                            items_done, total_items, current_item
                        ));
                    }
                    OpProgress::Finished { error_count, .. } => {
                        shark_p.progress_revealer.set_reveal_child(false);
                        if is_cut {
                            shark_p.clipboard.borrow_mut().clear();
                        }
                        if error_count > 0 {
                            show_error_dialog(
                                &shark_p.window,
                                &format!("Operation completed with {} errors", error_count),
                            );
                        }
                        shark_p.reload_current_directory();
                        return glib::ControlFlow::Break;
                    }
                    OpProgress::Error(e) => {
                        shark_p.progress_revealer.set_reveal_child(false);
                        show_error_dialog(&shark_p.window, &e);
                        shark_p.reload_current_directory();
                        return glib::ControlFlow::Break;
                    }
                }
            }
            glib::ControlFlow::Continue
        });

        if is_cut {
            move_paths_async(sources, cur_dir, ConflictResolution::KeepBoth, cancel, sender);
        } else {
            copy_paths_async(sources, cur_dir, ConflictResolution::KeepBoth, cancel, sender);
        }
    }

    /// Delete selected items (trash or permanent).
    fn handle_delete(self: &Rc<Self>, permanent: bool) {
        let selected = self.file_view.get_selected_entries();
        if selected.is_empty() {
            return;
        }

        let paths: Vec<PathBuf> = selected.into_iter().map(|e| e.path).collect();
        let shark_del = self.clone();

        if permanent {
            show_delete_confirmation(&self.window, paths.len(), move || {
                match delete_paths_permanent(&paths) {
                    Ok(_) => {
                        shark_del.reload_current_directory();
                    }
                    Err(e) => {
                        show_error_dialog(&shark_del.window, &e);
                        shark_del.reload_current_directory();
                    }
                }
            });
        } else {
            match trash_paths(&paths) {
                Ok(_) => {
                    shark_del.reload_current_directory();
                }
                Err(e) => {
                    show_error_dialog(&shark_del.window, &e);
                    shark_del.reload_current_directory();
                }
            }
        }
    }

    /// Rename selected item.
    fn handle_rename(self: &Rc<Self>) {
        let selected = self.file_view.get_selected_entries();
        if selected.len() != 1 {
            return;
        }

        let entry = &selected[0];
        let src_path = entry.path.clone();
        let shark_r = self.clone();

        show_rename_dialog(&self.window, &entry.name, move |new_name| {
            match rename_path(&src_path, &new_name) {
                Ok(_) => {
                    shark_r.reload_current_directory();
                }
                Err(e) => {
                    show_error_dialog(&shark_r.window, &e);
                }
            }
        });
    }

    /// Create new folder in current directory.
    fn handle_new_folder(self: &Rc<Self>) {
        let cur = self.history.borrow().current().to_path_buf();
        let shark_nf = self.clone();

        show_create_folder_dialog(&self.window, move |name| {
            match create_folder(&cur, &name) {
                Ok(_) => {
                    shark_nf.reload_current_directory();
                }
                Err(e) => {
                    show_error_dialog(&shark_nf.window, &e);
                }
            }
        });
    }

    /// Create new empty file in current directory.
    fn handle_new_file(self: &Rc<Self>) {
        let cur = self.history.borrow().current().to_path_buf();
        let shark_nf = self.clone();

        show_create_file_dialog(&self.window, move |name| {
            match create_file(&cur, &name) {
                Ok(_) => {
                    shark_nf.reload_current_directory();
                }
                Err(e) => {
                    show_error_dialog(&shark_nf.window, &e);
                }
            }
        });
    }

    /// Show properties dialog for single selected item.
    fn handle_properties(&self) {
        let selected = self.file_view.get_selected_entries();
        if let Some(entry) = selected.first() {
            show_properties_dialog(&self.window, entry);
        }
    }

    /// Show context menu on right click.
    fn show_context_menu(self: &Rc<Self>, target: Option<FileEntry>, x: f64, y: f64) {
        // Close any existing popover
        if let Some(old) = self.active_popover.borrow_mut().take() {
            old.popdown();
        }

        let popover = gtk4::Popover::builder()
            .has_arrow(false)
            .pointing_to(&gdk::Rectangle::new(x as i32, y as i32, 1, 1))
            .build();

        let menu_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        menu_box.set_margin_start(6);
        menu_box.set_margin_end(6);
        menu_box.set_margin_top(6);
        menu_box.set_margin_bottom(6);

        let make_item = |label: &str, icon_name: &str| {
            let btn = gtk4::Button::builder()
                .css_classes(["flat"])
                .halign(gtk4::Align::Fill)
                .build();

            let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
            let icon = gtk4::Image::from_icon_name(icon_name);
            icon.set_pixel_size(16);
            let lbl = gtk4::Label::builder()
                .label(label)
                .halign(gtk4::Align::Start)
                .hexpand(true)
                .build();

            row.append(&icon);
            row.append(&lbl);
            btn.set_child(Some(&row));
            btn
        };

        if let Some(entry) = target {
            // Context menu for a file or folder
            let open_btn = make_item("Open", "document-open-symbolic");
            let cut_btn = make_item("Cut", "edit-cut-symbolic");
            let copy_btn = make_item("Copy", "edit-copy-symbolic");
            let rename_btn = make_item("Rename...", "document-edit-symbolic");
            let trash_btn = make_item("Move to Trash", "user-trash-symbolic");
            let delete_btn = make_item("Delete Permanently...", "edit-delete-symbolic");
            let props_btn = make_item("Properties", "document-properties-symbolic");

            menu_box.append(&open_btn);
            menu_box.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
            menu_box.append(&cut_btn);
            menu_box.append(&copy_btn);
            menu_box.append(&rename_btn);
            menu_box.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
            menu_box.append(&trash_btn);
            menu_box.append(&delete_btn);
            menu_box.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
            menu_box.append(&props_btn);

            let shark_o = self.clone();
            let pop_o = popover.clone();
            let entry_o = entry.clone();
            open_btn.connect_clicked(move |_| {
                pop_o.popdown();
                shark_o.on_item_activated(entry_o.clone());
            });

            let shark_c = self.clone();
            let pop_c = popover.clone();
            cut_btn.connect_clicked(move |_| {
                pop_c.popdown();
                shark_c.handle_cut();
            });

            let shark_cp = self.clone();
            let pop_cp = popover.clone();
            copy_btn.connect_clicked(move |_| {
                pop_cp.popdown();
                shark_cp.handle_copy();
            });

            let shark_r = self.clone();
            let pop_r = popover.clone();
            rename_btn.connect_clicked(move |_| {
                pop_r.popdown();
                shark_r.handle_rename();
            });

            let shark_t = self.clone();
            let pop_t = popover.clone();
            trash_btn.connect_clicked(move |_| {
                pop_t.popdown();
                shark_t.handle_delete(false);
            });

            let shark_d = self.clone();
            let pop_d = popover.clone();
            delete_btn.connect_clicked(move |_| {
                pop_d.popdown();
                shark_d.handle_delete(true);
            });

            let shark_p = self.clone();
            let pop_p = popover.clone();
            props_btn.connect_clicked(move |_| {
                pop_p.popdown();
                shark_p.handle_properties();
            });
        } else {
            // Context menu for empty space
            let new_folder_btn = make_item("New Folder...", "folder-new-symbolic");
            let new_file_btn = make_item("New File...", "document-new-symbolic");
            let paste_btn = make_item("Paste", "edit-paste-symbolic");
            paste_btn.set_sensitive(!self.clipboard.borrow().is_empty());

            let select_all_btn = make_item("Select All", "edit-select-all-symbolic");
            let reload_btn = make_item("Reload", "view-refresh-symbolic");

            let cur_sort = self.config.borrow().sort_by.clone();
            let cur_rev = self.config.borrow().sort_reversed;

            let sort_name_btn = make_item(
                "Sort by Name",
                if cur_sort == "name" { "emblem-ok-symbolic" } else { "view-sort-ascending-symbolic" },
            );
            let sort_size_btn = make_item(
                "Sort by Size",
                if cur_sort == "size" { "emblem-ok-symbolic" } else { "view-sort-ascending-symbolic" },
            );
            let sort_date_btn = make_item(
                "Sort by Date Modified",
                if cur_sort == "modified" { "emblem-ok-symbolic" } else { "view-sort-ascending-symbolic" },
            );
            let sort_type_btn = make_item(
                "Sort by File Type",
                if cur_sort == "type" { "emblem-ok-symbolic" } else { "view-sort-ascending-symbolic" },
            );
            let sort_order_btn = make_item(
                if cur_rev { "Order: Descending (Z → A)" } else { "Order: Ascending (A → Z)" },
                if cur_rev { "view-sort-descending-symbolic" } else { "view-sort-ascending-symbolic" },
            );

            menu_box.append(&new_folder_btn);
            menu_box.append(&new_file_btn);
            menu_box.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
            menu_box.append(&paste_btn);
            menu_box.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
            menu_box.append(&sort_name_btn);
            menu_box.append(&sort_size_btn);
            menu_box.append(&sort_date_btn);
            menu_box.append(&sort_type_btn);
            menu_box.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
            menu_box.append(&sort_order_btn);
            menu_box.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
            menu_box.append(&select_all_btn);
            menu_box.append(&reload_btn);

            let shark_nf = self.clone();
            let pop_nf = popover.clone();
            new_folder_btn.connect_clicked(move |_| {
                pop_nf.popdown();
                shark_nf.handle_new_folder();
            });

            let shark_fi = self.clone();
            let pop_fi = popover.clone();
            new_file_btn.connect_clicked(move |_| {
                pop_fi.popdown();
                shark_fi.handle_new_file();
            });

            let shark_pst = self.clone();
            let pop_pst = popover.clone();
            paste_btn.connect_clicked(move |_| {
                pop_pst.popdown();
                shark_pst.handle_paste();
            });

            let shark_sn = self.clone();
            let pop_sn = popover.clone();
            sort_name_btn.connect_clicked(move |_| {
                pop_sn.popdown();
                shark_sn.update_sort("name", cur_rev);
            });

            let shark_ss = self.clone();
            let pop_ss = popover.clone();
            sort_size_btn.connect_clicked(move |_| {
                pop_ss.popdown();
                shark_ss.update_sort("size", cur_rev);
            });

            let shark_sd = self.clone();
            let pop_sd = popover.clone();
            sort_date_btn.connect_clicked(move |_| {
                pop_sd.popdown();
                shark_sd.update_sort("modified", cur_rev);
            });

            let shark_st = self.clone();
            let pop_st = popover.clone();
            sort_type_btn.connect_clicked(move |_| {
                pop_st.popdown();
                shark_st.update_sort("type", cur_rev);
            });

            let shark_so = self.clone();
            let pop_so = popover.clone();
            sort_order_btn.connect_clicked(move |_| {
                pop_so.popdown();
                shark_so.update_sort(&cur_sort, !cur_rev);
            });

            let shark_sa = self.clone();
            let pop_sa = popover.clone();
            select_all_btn.connect_clicked(move |_| {
                pop_sa.popdown();
                shark_sa.file_view.select_all();
            });

            let shark_rl = self.clone();
            let pop_rl = popover.clone();
            reload_btn.connect_clicked(move |_| {
                pop_rl.popdown();
                shark_rl.reload_current_directory();
            });
        }

        popover.set_child(Some(&menu_box));
        popover.set_parent(&self.file_view.container);
        popover.popup();

        *self.active_popover.borrow_mut() = Some(popover);
    }

    /// Update sorting parameters, persisting to config and updating both the view and toolbar.
    pub fn update_sort(self: &Rc<Self>, sort_by: &str, reversed: bool) {
        self.config.borrow_mut().sort_by = sort_by.to_string();
        self.config.borrow_mut().sort_reversed = reversed;
        self.config.borrow().save();
        self.file_view.set_sort(sort_by, reversed);
        self.toolbar.set_sort_state(sort_by, reversed);
    }

    /// Apply solid or translucent glass styling dynamically to window and popups.
    pub fn apply_background_mode(&self, mode: &str) {
        if mode == "glass" {
            self.window.remove_css_class("solid-bg");
            self.window.add_css_class("glass-bg");
            self.search_palette.window.remove_css_class("solid-bg");
            self.search_palette.window.add_css_class("glass-bg");
        } else {
            self.window.remove_css_class("glass-bg");
            self.window.add_css_class("solid-bg");
            self.search_palette.window.remove_css_class("glass-bg");
            self.search_palette.window.add_css_class("solid-bg");
        }
    }
}
