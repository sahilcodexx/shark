//! File display area with virtualized Grid and List views, sorting, filtering and selection.

use crate::models::{FileEntry, FileObject};
use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct FileView {
    pub container: gtk4::Stack,
    pub stack: gtk4::Stack,
    pub list_store: gio::ListStore,
    _filter_model: gtk4::FilterListModel,
    _sort_model: gtk4::SortListModel,
    pub selection_model: gtk4::MultiSelection,
    custom_filter: gtk4::CustomFilter,
    custom_sorter: gtk4::CustomSorter,
    show_hidden_state: Rc<RefCell<bool>>,
    search_query_state: Rc<RefCell<String>>,
    sort_by_state: Rc<RefCell<String>>,
    sort_reversed_state: Rc<RefCell<bool>>,
    grid_view: gtk4::GridView,
    list_view: gtk4::ListView,
}

impl Default for FileView {
    fn default() -> Self {
        Self::new()
    }
}

impl FileView {
    pub fn new() -> Self {
        let list_store = gio::ListStore::new::<FileObject>();

        let show_hidden_state = Rc::new(RefCell::new(false));
        let search_query_state = Rc::new(RefCell::new(String::new()));

        let sh_clone = show_hidden_state.clone();
        let sq_clone = search_query_state.clone();

        let custom_filter = gtk4::CustomFilter::new(move |obj| {
            let file_obj = obj.downcast_ref::<FileObject>().unwrap();
            let show_hidden = *sh_clone.borrow();
            let query = sq_clone.borrow().clone();

            if !show_hidden && file_obj.is_hidden() {
                return false;
            }

            if !query.is_empty() {
                let name = file_obj.name().to_lowercase();
                if !name.contains(&query) {
                    return false;
                }
            }

            true
        });

        let filter_model = gtk4::FilterListModel::new(Some(list_store.clone()), Some(custom_filter.clone()));

        let sort_by_state = Rc::new(RefCell::new("name".to_string()));
        let sort_reversed_state = Rc::new(RefCell::new(false));

        let sb_clone = sort_by_state.clone();
        let sr_clone = sort_reversed_state.clone();

        let custom_sorter = gtk4::CustomSorter::new(move |obj_a, obj_b| {
            let a = obj_a.downcast_ref::<FileObject>().unwrap();
            let b = obj_b.downcast_ref::<FileObject>().unwrap();

            // Directories always come first
            match (a.is_directory(), b.is_directory()) {
                (true, false) => return gtk4::Ordering::Smaller,
                (false, true) => return gtk4::Ordering::Larger,
                _ => {}
            }

            let sort_by = sb_clone.borrow().clone();
            let reversed = *sr_clone.borrow();

            let order = match sort_by.as_str() {
                "size" => a.size().cmp(&b.size()),
                "modified" => {
                    let time_a = a.with_entry(|e| e.modified);
                    let time_b = b.with_entry(|e| e.modified);
                    time_a.cmp(&time_b)
                }
                "type" => {
                    let type_a = a.content_type();
                    let type_b = b.content_type();
                    type_a.cmp(&type_b)
                }
                _ => {
                    // Default: name (case-insensitive)
                    let name_a = a.name().to_lowercase();
                    let name_b = b.name().to_lowercase();
                    name_a.cmp(&name_b)
                }
            };

            let final_order = if reversed {
                order.reverse()
            } else {
                order
            };

            match final_order {
                std::cmp::Ordering::Less => gtk4::Ordering::Smaller,
                std::cmp::Ordering::Equal => gtk4::Ordering::Equal,
                std::cmp::Ordering::Greater => gtk4::Ordering::Larger,
            }
        });

        let sort_model = gtk4::SortListModel::new(Some(filter_model.clone()), Some(custom_sorter.clone()));
        let selection_model = gtk4::MultiSelection::new(Some(sort_model.clone()));

        // 1. Grid View
        let grid_factory = gtk4::SignalListItemFactory::new();
        grid_factory.connect_setup(|_, list_item| {
            let item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();

            let card = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Vertical)
                .spacing(2)
                .halign(gtk4::Align::Center)
                .valign(gtk4::Align::Start)
                .css_classes(["file-grid-item"])
                .build();

            let icon = gtk4::Image::builder()
                .pixel_size(76)
                .width_request(96)
                .height_request(76)
                .halign(gtk4::Align::Center)
                .build();

            let label = gtk4::Label::builder()
                .halign(gtk4::Align::Center)
                .justify(gtk4::Justification::Center)
                .wrap(true)
                .wrap_mode(gtk4::pango::WrapMode::WordChar)
                .width_chars(13)
                .max_width_chars(15)
                .lines(2)
                .ellipsize(gtk4::pango::EllipsizeMode::End)
                .css_classes(["file-grid-label"])
                .build();

            card.append(&icon);
            card.append(&label);
            let detail = gtk4::Label::builder()
                .halign(gtk4::Align::Center)
                .justify(gtk4::Justification::Center)
                .css_classes(["file-grid-detail"])
                .build();
            card.append(&detail);
            item.set_child(Some(&card));
        });

        // Decode once; visible folders and documents share fast cached textures.
        let folder_texture = gtk4::gdk_pixbuf::Pixbuf::from_read(std::io::Cursor::new(
            include_bytes!("../../assets/icons/folder.svg").as_slice(),
        )).ok().map(|pixbuf| gtk4::gdk::Texture::for_pixbuf(&pixbuf));
        let doc_texture = gtk4::gdk_pixbuf::Pixbuf::from_read(std::io::Cursor::new(
            include_bytes!("../../assets/icons/document.svg").as_slice(),
        )).ok().map(|pixbuf| gtk4::gdk::Texture::for_pixbuf(&pixbuf));
        grid_factory.connect_bind(move |_, list_item| {
            let item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();
            let file_obj = item.item().and_downcast::<FileObject>().unwrap();
            let card = item.child().and_downcast::<gtk4::Box>().unwrap();

            let icon = card.first_child().and_downcast::<gtk4::Image>().unwrap();
            let label = icon.next_sibling().and_downcast::<gtk4::Label>().unwrap();
            let detail = card.last_child().and_downcast::<gtk4::Label>().unwrap();

            if file_obj.is_directory() && !file_obj.with_entry(|e| e.is_symlink) {
                if let Some(texture) = &folder_texture {
                    icon.set_paintable(Some(texture));
                } else {
                    icon.set_icon_name(Some(&file_obj.icon_name()));
                }
            } else if file_obj.with_entry(|e| {
                let ct = &e.content_type;
                ct.starts_with("text/") || ct.contains("json") || ct.contains("toml")
                    || ct.contains("xml") || ct.contains("markdown")
                    || ct.ends_with("/lock") || ct == "application/octet-stream"
            }) && let Some(texture) = &doc_texture {
                icon.set_paintable(Some(texture));
            } else {
                icon.set_icon_name(Some(&file_obj.icon_name()));
            }

            detail.set_text(&file_obj.with_entry(|entry| {
                if entry.is_directory {
                    if let Ok(count) = std::fs::read_dir(&entry.path).map(|r| r.count()) {
                        if count == 0 {
                            "Empty".to_string()
                        } else if count == 1 {
                            "1 item".to_string()
                        } else {
                            format!("{} items", count)
                        }
                    } else {
                        "Folder".to_string()
                    }
                } else {
                    entry.formatted_size()
                }
            }));
            label.set_text(&file_obj.name());
            card.set_tooltip_text(Some(&file_obj.name()));
        });

        let grid_view = gtk4::GridView::builder()
            .model(&selection_model)
            .factory(&grid_factory)
            .max_columns(12)
            .min_columns(1)
            .enable_rubberband(true)
            .build();

        // 2. List View
        let list_factory = gtk4::SignalListItemFactory::new();
        list_factory.connect_setup(|_, list_item| {
            let item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();

            let row = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Horizontal)
                .spacing(12)
                .css_classes(["file-list-row"])
                .build();

            let icon = gtk4::Image::builder()
                .pixel_size(18)
                .build();

            let name_label = gtk4::Label::builder()
                .halign(gtk4::Align::Start)
                .hexpand(true)
                .ellipsize(gtk4::pango::EllipsizeMode::End)
                .build();

            let size_label = gtk4::Label::builder()
                .halign(gtk4::Align::End)
                .width_chars(10)
                .css_classes(["dim-label"])
                .build();

            let type_label = gtk4::Label::builder()
                .halign(gtk4::Align::Start)
                .width_chars(18)
                .ellipsize(gtk4::pango::EllipsizeMode::End)
                .css_classes(["dim-label"])
                .build();

            let date_label = gtk4::Label::builder()
                .halign(gtk4::Align::End)
                .width_chars(16)
                .css_classes(["dim-label"])
                .build();

            row.append(&icon);
            row.append(&name_label);
            row.append(&size_label);
            row.append(&type_label);
            row.append(&date_label);

            item.set_child(Some(&row));
        });

        list_factory.connect_bind(|_, list_item| {
            let item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();
            let file_obj = item.item().and_downcast::<FileObject>().unwrap();
            let row = item.child().and_downcast::<gtk4::Box>().unwrap();

            let entry = file_obj.entry();

            let mut child = row.first_child();
            if let Some(icon) = child.as_ref().and_then(|c| c.downcast_ref::<gtk4::Image>()) {
                icon.set_icon_name(Some(&entry.icon_name));
            }

            child = child.and_then(|c| c.next_sibling());
            if let Some(name_lbl) = child.as_ref().and_then(|c| c.downcast_ref::<gtk4::Label>()) {
                name_lbl.set_text(&entry.name);
            }

            child = child.and_then(|c| c.next_sibling());
            if let Some(size_lbl) = child.as_ref().and_then(|c| c.downcast_ref::<gtk4::Label>()) {
                size_lbl.set_text(&entry.formatted_size());
            }

            child = child.and_then(|c| c.next_sibling());
            if let Some(type_lbl) = child.as_ref().and_then(|c| c.downcast_ref::<gtk4::Label>()) {
                type_lbl.set_text(&entry.type_description());
            }

            child = child.and_then(|c| c.next_sibling());
            if let Some(date_lbl) = child.as_ref().and_then(|c| c.downcast_ref::<gtk4::Label>()) {
                date_lbl.set_text(&entry.formatted_modified());
            }
        });

        let list_view = gtk4::ListView::builder()
            .model(&selection_model)
            .factory(&list_factory)
            .enable_rubberband(true)
            .build();

        // Scrollable views must directly own their adjustments for virtualization.
        let grid_scroll = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .child(&grid_view)
            .build();
        let list_scroll = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Automatic)
            .child(&list_view)
            .build();
        let stack = gtk4::Stack::builder()
            .hexpand(true)
            .vexpand(true)
            .build();
        stack.add_named(&grid_scroll, Some("grid"));
        stack.add_named(&list_scroll, Some("list"));

        Self {
            container: stack.clone(),
            stack,
            list_store,
            _filter_model: filter_model,
            _sort_model: sort_model,
            selection_model,
            custom_filter,
            custom_sorter,
            show_hidden_state,
            search_query_state,
            sort_by_state,
            sort_reversed_state,
            grid_view,
            list_view,
        }
    }

    /// Clear all items from view.
    pub fn clear(&self) {
        self.list_store.remove_all();
    }

    /// Append a batch of FileEntry items.
    pub fn add_batch(&self, entries: Vec<FileEntry>) {
        let objects: Vec<FileObject> = entries.into_iter().map(FileObject::new).collect();
        self.list_store.extend_from_slice(&objects);
    }

    /// Set whether hidden files are shown.
    pub fn set_show_hidden(&self, show: bool) {
        *self.show_hidden_state.borrow_mut() = show;
        self.custom_filter.changed(gtk4::FilterChange::Different);
    }

    /// Set search filter query string.
    pub fn set_search_query(&self, query: &str) {
        *self.search_query_state.borrow_mut() = query.trim().to_lowercase();
        self.custom_filter.changed(gtk4::FilterChange::Different);
    }

    /// Update sort settings.
    pub fn set_sort(&self, sort_by: &str, reversed: bool) {
        *self.sort_by_state.borrow_mut() = sort_by.to_string();
        *self.sort_reversed_state.borrow_mut() = reversed;
        self.custom_sorter.changed(gtk4::SorterChange::Different);
    }

    /// Switch view mode ("grid" or "list").
    pub fn set_view_mode(&self, mode: &str) {
        self.stack.set_visible_child_name(mode);
    }

    /// Get all currently selected FileEntry objects.
    pub fn get_selected_entries(&self) -> Vec<FileEntry> {
        let bitset = self.selection_model.selection();
        let mut entries = Vec::new();
        for i in 0..bitset.size() {
            let index = bitset.nth(i as u32);
            if let Some(obj) = self.selection_model.item(index).and_downcast::<FileObject>() {
                entries.push(obj.entry());
            }
        }
        entries
    }

    /// Select all items in current view.
    pub fn select_all(&self) {
        self.selection_model.select_all();
    }

    /// Unselect all items.
    pub fn unselect_all(&self) {
        self.selection_model.unselect_all();
    }

    /// Total count of items currently in the store.
    pub fn item_count(&self) -> u32 {
        self.selection_model.n_items()
    }

    /// Connect double-click / Enter activation handler.
    pub fn connect_activate<F: Fn(FileEntry) + 'static>(&self, f: F) {
        let f = Rc::new(f);

        let sel_m = self.selection_model.clone();
        let f_grid = f.clone();
        self.grid_view.connect_activate(move |_, pos| {
            if let Some(obj) = sel_m.item(pos).and_downcast::<FileObject>() {
                f_grid(obj.entry());
            }
        });

        let sel_m2 = self.selection_model.clone();
        let f_list = f;
        self.list_view.connect_activate(move |_, pos| {
            if let Some(obj) = sel_m2.item(pos).and_downcast::<FileObject>() {
                f_list(obj.entry());
            }
        });
    }

    /// Connect right-click / context menu popup.
    pub fn connect_context_menu<F: Fn(Option<FileEntry>, f64, f64) + 'static>(&self, f: F) {
        let f = Rc::new(f);

        // Click gesture for GridView
        let gesture_grid = gtk4::GestureClick::builder()
            .button(3) // Right mouse button
            .build();

        let sel_m = self.selection_model.clone();
        let f_grid = f.clone();
        gesture_grid.connect_released(move |gesture, _, x, y| {
            // Find item at x, y
            let selected = get_first_selected_entry(&sel_m);
            gesture.set_state(gtk4::EventSequenceState::Claimed);
            f_grid(selected, x, y);
        });
        self.grid_view.add_controller(gesture_grid);

        // Click gesture for ListView
        let gesture_list = gtk4::GestureClick::builder()
            .button(3)
            .build();

        let sel_m2 = self.selection_model.clone();
        let f_list = f;
        gesture_list.connect_released(move |gesture, _, x, y| {
            let selected = get_first_selected_entry(&sel_m2);
            gesture.set_state(gtk4::EventSequenceState::Claimed);
            f_list(selected, x, y);
        });
        self.list_view.add_controller(gesture_list);
    }
}

fn get_first_selected_entry(model: &gtk4::MultiSelection) -> Option<FileEntry> {
    let bitset = model.selection();
    if bitset.is_empty() {
        return None;
    }
    let first_idx = bitset.nth(0);
    model.item(first_idx).and_downcast::<FileObject>().map(|o| o.entry())
}
