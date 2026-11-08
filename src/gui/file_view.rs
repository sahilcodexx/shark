//! File display area with virtualized Grid and List views, sorting, filtering and selection.

use crate::models::{FileEntry, FileObject};
use crate::utils::{
    generate_thumbnail_file, is_previewable_media, load_fast_cached_thumbnail,
};
use gtk4::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
    mpsc::{Sender, channel},
};
use std::time::Duration;

type SortRequestCallback = Rc<RefCell<Option<Box<dyn Fn(String, bool)>>>>;

struct ThumbnailRequest {
    path: PathBuf,
    content_type: String,
    size: u64,
    generation: u64,
}

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
    pub thumbnail_cache: Rc<RefCell<HashMap<PathBuf, Option<gtk4::gdk::Texture>>>>,
    pub icon_size: Rc<RefCell<i32>>,
    grid_view: gtk4::GridView,
    list_view: gtk4::ListView,
    header_name_label: gtk4::Label,
    header_size_label: gtk4::Label,
    header_type_label: gtk4::Label,
    header_date_label: gtk4::Label,
    on_sort_request: SortRequestCallback,
    #[allow(dead_code)]
    thumb_request_tx: Sender<ThumbnailRequest>,
    thumb_generation: Arc<AtomicU64>,
}

#[derive(Clone)]
struct BuiltinIcons {
    folder: Option<gtk4::gdk::Texture>,
    doc: Option<gtk4::gdk::Texture>,
    pdf: Option<gtk4::gdk::Texture>,
    archive: Option<gtk4::gdk::Texture>,
    code: Option<gtk4::gdk::Texture>,
    audio: Option<gtk4::gdk::Texture>,
    video: Option<gtk4::gdk::Texture>,
    image: Option<gtk4::gdk::Texture>,
}

impl BuiltinIcons {
    fn new() -> Self {
        let load_svg = |bytes: &'static [u8]| {
            gtk4::gdk_pixbuf::Pixbuf::from_read(std::io::Cursor::new(bytes))
                .ok()
                .map(|pixbuf| gtk4::gdk::Texture::for_pixbuf(&pixbuf))
        };

        Self {
            folder: load_svg(include_bytes!("../../assets/icons/folder.svg")),
            doc: load_svg(include_bytes!("../../assets/icons/document.svg")),
            pdf: load_svg(include_bytes!("../../assets/icons/pdf.svg")),
            archive: load_svg(include_bytes!("../../assets/icons/archive.svg")),
            code: load_svg(include_bytes!("../../assets/icons/code.svg")),
            audio: load_svg(include_bytes!("../../assets/icons/audio.svg")),
            video: load_svg(include_bytes!("../../assets/icons/video.svg")),
            image: load_svg(include_bytes!("../../assets/icons/image.svg")),
        }
    }

    fn resolve(&self, content_type: &str, path: &std::path::Path, is_dir: bool) -> Option<gtk4::gdk::Texture> {
        if is_dir {
            return self.folder.clone();
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

        // 1. PDF
        if content_type == "application/pdf" || ext == "pdf" {
            return self.pdf.clone().or_else(|| self.doc.clone());
        }

        // 2. Archives / Compressed
        if content_type.contains("archive") || content_type.contains("zip") || content_type.contains("tar")
            || matches!(ext.as_str(), "zip" | "tar" | "gz" | "xz" | "bz2" | "7z" | "rar" | "zst" | "iso" | "deb" | "rpm")
        {
            return self.archive.clone().or_else(|| self.doc.clone());
        }

        // 3. Audio / Music
        if content_type.starts_with("audio/") || matches!(ext.as_str(), "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "opus" | "wma") {
            return self.audio.clone().or_else(|| self.doc.clone());
        }

        // 4. Video (fallback when no thumbnail)
        if content_type.starts_with("video/") || matches!(ext.as_str(), "mp4" | "mkv" | "webm" | "avi" | "mov" | "flv" | "wmv" | "m4v") {
            return self.video.clone().or_else(|| self.doc.clone());
        }

        // 5. Image (fallback when no thumbnail)
        if content_type.starts_with("image/") || matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp" | "svg" | "gif" | "bmp" | "ico" | "avif") {
            return self.image.clone().or_else(|| self.doc.clone());
        }

        // 6. Source code / config / markup
        if content_type.starts_with("text/x-")
            || content_type.contains("javascript")
            || content_type.contains("json")
            || content_type.contains("toml")
            || content_type.contains("yaml")
            || content_type.contains("xml")
            || matches!(
                ext.as_str(),
                "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "c" | "cpp" | "h" | "hpp"
                | "sh" | "bash" | "zsh" | "html" | "css" | "scss" | "json" | "toml"
                | "yaml" | "yml" | "xml" | "sql" | "lua" | "go" | "java" | "kt" | "swift"
                | "rb" | "php" | "lock"
            )
        {
            return self.code.clone().or_else(|| self.doc.clone());
        }

        // 7. General documents, text, notes, logs, csv, data
        self.doc.clone()
    }
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
            let query_borrow = sq_clone.borrow();

            if !show_hidden && file_obj.is_hidden() {
                return false;
            }

            if !query_borrow.is_empty() {
                if !file_obj.with_entry(|e| e.name_lower.contains(query_borrow.as_str())) {
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

            let sb = sb_clone.borrow();
            let reversed = *sr_clone.borrow();

            let order = match sb.as_str() {
                "size" => a.size().cmp(&b.size()),
                "modified" => {
                    let time_a = a.with_entry(|e| e.modified);
                    let time_b = b.with_entry(|e| e.modified);
                    time_a.cmp(&time_b)
                }
                "type" => {
                    a.with_entry(|ea| {
                        b.with_entry(|eb| ea.content_type.cmp(&eb.content_type))
                    })
                }
                _ => {
                    // Default: name (case-insensitive) using precomputed lowercase without allocations
                    a.with_entry(|ea| {
                        b.with_entry(|eb| ea.name_lower.cmp(&eb.name_lower))
                    })
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
                .pixel_size(86)
                .width_request(104)
                .height_request(86)
                .halign(gtk4::Align::Center)
                .build();

            let label = gtk4::Label::builder()
                .halign(gtk4::Align::Center)
                .justify(gtk4::Justification::Center)
                .wrap(true)
                .wrap_mode(gtk4::pango::WrapMode::WordChar)
                .width_chars(14)
                .max_width_chars(16)
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

        let builtin_icons = BuiltinIcons::new();
        let icons_grid = builtin_icons.clone();
        let icons_list = builtin_icons;

        let thumbnail_cache = Rc::new(RefCell::new(HashMap::<PathBuf, Option<gtk4::gdk::Texture>>::new()));
        let thumb_cache_grid = thumbnail_cache.clone();
        let thumb_cache_list = thumbnail_cache.clone();

        let icon_size = Rc::new(RefCell::new(86));
        let icon_size_grid = icon_size.clone();

        let (thumb_request_tx, thumb_rx) = channel::<ThumbnailRequest>();
        let (thumb_done_tx, thumb_done_rx) = channel::<(PathBuf, u64)>();
        let thumb_generation = Arc::new(AtomicU64::new(0));

        let gen_worker = thumb_generation.clone();
        std::thread::Builder::new()
            .name("shark-thumb-worker".to_string())
            .spawn(move || {
                while let Ok(req) = thumb_rx.recv() {
                    if req.generation != gen_worker.load(Ordering::Relaxed) {
                        continue;
                    }
                    if generate_thumbnail_file(&req.path, &req.content_type, req.size) {
                        if req.generation == gen_worker.load(Ordering::Relaxed) {
                            let _ = thumb_done_tx.send((req.path, req.generation));
                        }
                    }
                }
            })
            .expect("Failed to spawn thumbnail worker thread");

        let list_store_thumb = list_store.clone();
        let thumb_cache_async = thumbnail_cache.clone();
        let gen_listener = thumb_generation.clone();

        glib::timeout_add_local(Duration::from_millis(60), move || {
            let mut updated_indices = Vec::new();
            let current_gen = gen_listener.load(Ordering::Relaxed);

            while let Ok((path, req_gen)) = thumb_done_rx.try_recv() {
                if req_gen != current_gen {
                    continue;
                }
                if let Some(texture) = load_fast_cached_thumbnail(&path, 96, 84) {
                    thumb_cache_async.borrow_mut().insert(path.clone(), Some(texture));
                    let count = list_store_thumb.n_items();
                    for i in 0..count {
                        if let Some(obj) = list_store_thumb.item(i).and_downcast::<FileObject>() {
                            if obj.with_entry(|e| e.path == path) {
                                updated_indices.push(i);
                                break;
                            }
                        }
                    }
                }
            }

            for idx in updated_indices {
                list_store_thumb.items_changed(idx, 1, 1);
            }

            glib::ControlFlow::Continue
        });

        let thumb_req_tx_grid = thumb_request_tx.clone();
        let thumb_gen_grid = thumb_generation.clone();

        grid_factory.connect_bind(move |_, list_item| {
            let item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();
            let file_obj = item.item().and_downcast::<FileObject>().unwrap();
            let card = item.child().and_downcast::<gtk4::Box>().unwrap();

            let icon = card.first_child().and_downcast::<gtk4::Image>().unwrap();
            let label = icon.next_sibling().and_downcast::<gtk4::Label>().unwrap();
            let detail = card.last_child().and_downcast::<gtk4::Label>().unwrap();

            let cur_size = *icon_size_grid.borrow();
            icon.set_pixel_size(cur_size);
            icon.set_width_request(cur_size + 18);
            icon.set_height_request(cur_size);

            icon.remove_css_class("file-grid-thumbnail");

            if file_obj.is_directory() {
                if let Some(ref texture) = icons_grid.folder {
                    icon.set_paintable(Some(texture));
                } else {
                    icon.set_icon_name(Some(&file_obj.icon_name()));
                }
            } else if file_obj.with_entry(|e| is_previewable_media(&e.content_type, &e.path)) {
                let (path, content_type, size, icon_name) = file_obj.with_entry(|e| {
                    (e.path.clone(), e.content_type.clone(), e.size, e.icon_name.clone())
                });

                let cached = thumb_cache_grid.borrow().get(&path).cloned();
                match cached {
                    Some(Some(ref texture)) => {
                        icon.add_css_class("file-grid-thumbnail");
                        icon.set_paintable(Some(texture));
                    }
                    Some(None) => {
                        if let Some(texture) = icons_grid.resolve(&content_type, &path, false) {
                            icon.set_paintable(Some(&texture));
                        } else {
                            icon.set_icon_name(Some(&icon_name));
                        }
                    }
                    None => {
                        if let Some(tex) = load_fast_cached_thumbnail(&path, 96, 84) {
                            icon.add_css_class("file-grid-thumbnail");
                            icon.set_paintable(Some(&tex));
                            thumb_cache_grid.borrow_mut().insert(path, Some(tex));
                        } else {
                            if let Some(fallback_tex) = icons_grid.resolve(&content_type, &path, false) {
                                icon.set_paintable(Some(&fallback_tex));
                            } else {
                                icon.set_icon_name(Some(&icon_name));
                            }
                            thumb_cache_grid.borrow_mut().insert(path.clone(), None);
                            let _ = thumb_req_tx_grid.send(ThumbnailRequest {
                                path,
                                content_type,
                                size,
                                generation: thumb_gen_grid.load(Ordering::Relaxed),
                            });
                        }
                    }
                }
            } else {
                let (path, content_type, icon_name) = file_obj.with_entry(|e| {
                    (e.path.clone(), e.content_type.clone(), e.icon_name.clone())
                });
                if let Some(texture) = icons_grid.resolve(&content_type, &path, false) {
                    icon.set_paintable(Some(&texture));
                } else {
                    icon.set_icon_name(Some(&icon_name));
                }
            }

            detail.set_text(&file_obj.with_entry(|entry| entry.formatted_size()));
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

        let thumb_req_tx_list = thumb_request_tx.clone();
        let thumb_gen_list = thumb_generation.clone();

        list_factory.connect_bind(move |_, list_item| {
            let item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();
            let file_obj = item.item().and_downcast::<FileObject>().unwrap();
            let row = item.child().and_downcast::<gtk4::Box>().unwrap();

            file_obj.with_entry(|entry| {
                let mut child = row.first_child();
                if let Some(icon) = child.as_ref().and_then(|c| c.downcast_ref::<gtk4::Image>()) {
                    if entry.is_directory {
                        if let Some(ref texture) = icons_list.folder {
                            icon.set_paintable(Some(texture));
                        } else {
                            icon.set_icon_name(Some(&entry.icon_name));
                        }
                    } else if is_previewable_media(&entry.content_type, &entry.path) {
                        let cached = thumb_cache_list.borrow().get(&entry.path).cloned();
                        match cached {
                            Some(Some(ref texture)) => {
                                icon.set_paintable(Some(texture));
                            }
                            Some(None) => {
                                if let Some(texture) = icons_list.resolve(&entry.content_type, &entry.path, false) {
                                    icon.set_paintable(Some(&texture));
                                } else {
                                    icon.set_icon_name(Some(&entry.icon_name));
                                }
                            }
                            None => {
                                if let Some(tex) = load_fast_cached_thumbnail(&entry.path, 32, 32) {
                                    icon.set_paintable(Some(&tex));
                                    thumb_cache_list.borrow_mut().insert(entry.path.clone(), Some(tex));
                                } else {
                                    if let Some(fallback_tex) = icons_list.resolve(&entry.content_type, &entry.path, false) {
                                        icon.set_paintable(Some(&fallback_tex));
                                    } else {
                                        icon.set_icon_name(Some(&entry.icon_name));
                                    }
                                    thumb_cache_list.borrow_mut().insert(entry.path.clone(), None);
                                    let _ = thumb_req_tx_list.send(ThumbnailRequest {
                                        path: entry.path.clone(),
                                        content_type: entry.content_type.clone(),
                                        size: entry.size,
                                        generation: thumb_gen_list.load(Ordering::Relaxed),
                                    });
                                }
                            }
                        }
                    } else if let Some(texture) = icons_list.resolve(&entry.content_type, &entry.path, false) {
                        icon.set_paintable(Some(&texture));
                    } else {
                        icon.set_icon_name(Some(&entry.icon_name));
                    }
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
        // Column headers for List view
        let header_name_label = gtk4::Label::builder()
            .label("Name ▲")
            .halign(gtk4::Align::Start)
            .hexpand(true)
            .css_classes(["file-list-header-label"])
            .build();
        let header_name_btn = gtk4::Button::builder()
            .child(&header_name_label)
            .halign(gtk4::Align::Fill)
            .hexpand(true)
            .css_classes(["flat", "file-list-header-btn"])
            .build();

        let header_size_label = gtk4::Label::builder()
            .label("Size")
            .halign(gtk4::Align::End)
            .width_chars(10)
            .css_classes(["file-list-header-label"])
            .build();
        let header_size_btn = gtk4::Button::builder()
            .child(&header_size_label)
            .halign(gtk4::Align::End)
            .css_classes(["flat", "file-list-header-btn"])
            .build();

        let header_type_label = gtk4::Label::builder()
            .label("Type")
            .halign(gtk4::Align::Start)
            .width_chars(18)
            .css_classes(["file-list-header-label"])
            .build();
        let header_type_btn = gtk4::Button::builder()
            .child(&header_type_label)
            .halign(gtk4::Align::Start)
            .css_classes(["flat", "file-list-header-btn"])
            .build();

        let header_date_label = gtk4::Label::builder()
            .label("Modified")
            .halign(gtk4::Align::End)
            .width_chars(16)
            .css_classes(["file-list-header-label"])
            .build();
        let header_date_btn = gtk4::Button::builder()
            .child(&header_date_label)
            .halign(gtk4::Align::End)
            .css_classes(["flat", "file-list-header-btn"])
            .build();

        let icon_spacer = gtk4::Box::builder()
            .width_request(18)
            .build();

        let list_header = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .css_classes(["file-list-header"])
            .build();
        list_header.append(&icon_spacer);
        list_header.append(&header_name_btn);
        list_header.append(&header_size_btn);
        list_header.append(&header_type_btn);
        list_header.append(&header_date_btn);

        let on_sort_request: SortRequestCallback = Rc::new(RefCell::new(None));

        let sb_h = sort_by_state.clone();
        let sr_h = sort_reversed_state.clone();
        let osr_name = on_sort_request.clone();
        header_name_btn.connect_clicked(move |_| {
            let cur_col = sb_h.borrow().clone();
            let cur_rev = *sr_h.borrow();
            let new_rev = if cur_col == "name" { !cur_rev } else { false };
            if let Some(ref cb) = *osr_name.borrow() {
                cb("name".to_string(), new_rev);
            }
        });

        let sb_h2 = sort_by_state.clone();
        let sr_h2 = sort_reversed_state.clone();
        let osr_size = on_sort_request.clone();
        header_size_btn.connect_clicked(move |_| {
            let cur_col = sb_h2.borrow().clone();
            let cur_rev = *sr_h2.borrow();
            let new_rev = if cur_col == "size" { !cur_rev } else { false };
            if let Some(ref cb) = *osr_size.borrow() {
                cb("size".to_string(), new_rev);
            }
        });

        let sb_h3 = sort_by_state.clone();
        let sr_h3 = sort_reversed_state.clone();
        let osr_type = on_sort_request.clone();
        header_type_btn.connect_clicked(move |_| {
            let cur_col = sb_h3.borrow().clone();
            let cur_rev = *sr_h3.borrow();
            let new_rev = if cur_col == "type" { !cur_rev } else { false };
            if let Some(ref cb) = *osr_type.borrow() {
                cb("type".to_string(), new_rev);
            }
        });

        let sb_h4 = sort_by_state.clone();
        let sr_h4 = sort_reversed_state.clone();
        let osr_date = on_sort_request.clone();
        header_date_btn.connect_clicked(move |_| {
            let cur_col = sb_h4.borrow().clone();
            let cur_rev = *sr_h4.borrow();
            let new_rev = if cur_col == "modified" { !cur_rev } else { false };
            if let Some(ref cb) = *osr_date.borrow() {
                cb("modified".to_string(), new_rev);
            }
        });

        let list_container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .hexpand(true)
            .vexpand(true)
            .build();
        list_container.append(&list_header);
        list_container.append(&list_scroll);

        let stack = gtk4::Stack::builder()
            .hexpand(true)
            .vexpand(true)
            .build();
        stack.add_named(&grid_scroll, Some("grid"));
        stack.add_named(&list_container, Some("list"));

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
            thumbnail_cache,
            icon_size,
            grid_view,
            list_view,
            header_name_label,
            header_size_label,
            header_type_label,
            header_date_label,
            on_sort_request,
            thumb_request_tx,
            thumb_generation,
        }
    }

    fn apply_zoom_to_visible(&self, size: i32) {
        fn walk(widget: &gtk4::Widget, size: i32) {
            if widget.has_css_class("file-grid-item") {
                if let Some(card) = widget.downcast_ref::<gtk4::Box>() {
                    if let Some(icon) = card.first_child().and_then(|c| c.downcast::<gtk4::Image>().ok()) {
                        icon.set_pixel_size(size);
                        icon.set_width_request(size + 18);
                        icon.set_height_request(size);
                    }
                }
                return;
            }
            let mut child = widget.first_child();
            while let Some(current) = child {
                walk(&current, size);
                child = current.next_sibling();
            }
        }
        walk(self.grid_view.upcast_ref(), size);

        let count = self.list_store.n_items();
        if count > 0 {
            self.list_store.items_changed(0, count, count);
        }
    }

    /// Zoom in grid icons
    pub fn zoom_in(&self) {
        let cur = *self.icon_size.borrow();
        if cur < 160 {
            let new_size = (cur + 16).min(160);
            *self.icon_size.borrow_mut() = new_size;
            self.apply_zoom_to_visible(new_size);
        }
    }

    /// Zoom out grid icons
    pub fn zoom_out(&self) {
        let cur = *self.icon_size.borrow();
        if cur > 48 {
            let new_size = (cur - 16).max(48);
            *self.icon_size.borrow_mut() = new_size;
            self.apply_zoom_to_visible(new_size);
        }
    }

    /// Reset zoom to default 86px
    pub fn zoom_reset(&self) {
        *self.icon_size.borrow_mut() = 86;
        self.apply_zoom_to_visible(86);
    }

    /// Clear all items from view.
    pub fn clear(&self) {
        self.thumb_generation.fetch_add(1, Ordering::Relaxed);
        self.list_store.remove_all();
        self.thumbnail_cache.borrow_mut().clear();
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
        self.update_header_indicators(sort_by, reversed);
        self.custom_sorter.changed(gtk4::SorterChange::Different);
    }

    /// Update column header arrow indicators in list view.
    pub fn update_header_indicators(&self, sort_by: &str, reversed: bool) {
        let arrow = if reversed { " ▼" } else { " ▲" };
        self.header_name_label.set_text(&format!("Name{}", if sort_by == "name" { arrow } else { "" }));
        self.header_size_label.set_text(&format!("Size{}", if sort_by == "size" { arrow } else { "" }));
        self.header_type_label.set_text(&format!("Type{}", if sort_by == "type" { arrow } else { "" }));
        self.header_date_label.set_text(&format!("Modified{}", if sort_by == "modified" { arrow } else { "" }));
    }

    /// Connect sort request triggered from list view headers.
    pub fn connect_sort_request<F: Fn(String, bool) + 'static>(&self, f: F) {
        *self.on_sort_request.borrow_mut() = Some(Box::new(f));
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
