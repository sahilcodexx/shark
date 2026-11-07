//! Keyboard-first recursive file search, independent of the browsing view.
use crate::{
    filesystem::{ScanMessage, recursive_search_async},
    models::{FileEntry, FileObject},
};
use gtk4::{gdk, prelude::*};
use std::{
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

pub struct SearchPalette {
    pub window: gtk4::Window,
    pub entry: gtk4::SearchEntry,
    store: gio::ListStore,
    selection: gtk4::SingleSelection,
    scope: gtk4::Label,
    status: gtk4::Label,
    root: RefCell<PathBuf>,
    show_hidden: RefCell<bool>,
    cancel: RefCell<Arc<AtomicBool>>,
    debounce: RefCell<Option<glib::SourceId>>,
}

impl SearchPalette {
    pub fn new(parent: &gtk4::ApplicationWindow) -> Rc<Self> {
        let window = gtk4::Window::builder()
            .transient_for(parent)
            .destroy_with_parent(true)
            .modal(true)
            .decorated(false)
            .resizable(false)
            .default_width(600)
            .default_height(440)
            .title("Search files")
            .css_classes(["search-palette"])
            .build();
        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        let input_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        input_row.add_css_class("palette-input");
        let entry = gtk4::SearchEntry::builder()
            .placeholder_text("Search files and folders…")
            .hexpand(true)
            .build();
        let close = gtk4::Button::from_icon_name("window-close-symbolic");
        close.set_tooltip_text(Some("Close search (Esc)"));
        close.add_css_class("flat");
        input_row.append(&entry);
        input_row.append(&close);
        let scope = gtk4::Label::builder()
            .xalign(0.0)
            .ellipsize(gtk4::pango::EllipsizeMode::Middle)
            .css_classes(["palette-scope"])
            .build();
        let store = gio::ListStore::new::<FileObject>();
        let selection = gtk4::SingleSelection::new(Some(store.clone()));
        let factory = gtk4::SignalListItemFactory::new();
        factory.connect_setup(|_, item| {
            let item = item.downcast_ref::<gtk4::ListItem>().unwrap();
            let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
            row.add_css_class("palette-result");
            let icon = gtk4::Image::builder().pixel_size(24).build();
            let labels = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
            labels.set_hexpand(true);
            let name = gtk4::Label::builder()
                .xalign(0.0)
                .ellipsize(gtk4::pango::EllipsizeMode::End)
                .build();
            let path = gtk4::Label::builder()
                .xalign(0.0)
                .ellipsize(gtk4::pango::EllipsizeMode::Middle)
                .css_classes(["palette-result-path"])
                .build();
            labels.append(&name);
            labels.append(&path);
            row.append(&icon);
            row.append(&labels);
            item.set_child(Some(&row));
        });
        let folder_texture = gtk4::gdk_pixbuf::Pixbuf::from_read(std::io::Cursor::new(
            include_bytes!("../../assets/icons/folder.svg").as_slice(),
        )).ok().map(|pixbuf| gtk4::gdk::Texture::for_pixbuf(&pixbuf));
        let doc_texture = gtk4::gdk_pixbuf::Pixbuf::from_read(std::io::Cursor::new(
            include_bytes!("../../assets/icons/document.svg").as_slice(),
        )).ok().map(|pixbuf| gtk4::gdk::Texture::for_pixbuf(&pixbuf));
        factory.connect_bind(move |_, item| {
            let item = item.downcast_ref::<gtk4::ListItem>().unwrap();
            let file = item.item().and_downcast::<FileObject>().unwrap();
            let row = item.child().and_downcast::<gtk4::Box>().unwrap();
            let icon = row.first_child().and_downcast::<gtk4::Image>().unwrap();
            let labels = row.last_child().and_downcast::<gtk4::Box>().unwrap();
            let name = labels.first_child().and_downcast::<gtk4::Label>().unwrap();
            let path = labels.last_child().and_downcast::<gtk4::Label>().unwrap();

            if file.is_directory() && let Some(texture) = &folder_texture {
                icon.set_paintable(Some(texture));
            } else if file.with_entry(|e| {
                let ct = &e.content_type;
                ct.starts_with("text/") || ct.contains("json") || ct.contains("toml")
                    || ct.contains("xml") || ct.contains("markdown")
                    || ct.contains("pdf") || ct == "application/octet-stream"
            }) && let Some(texture) = &doc_texture {
                icon.set_paintable(Some(texture));
            } else {
                icon.set_icon_name(Some(&file.icon_name()));
            }

            name.set_text(&file.name());
            file.with_entry(|file| {
                path.set_text(&file.path.parent().unwrap_or(&file.path).to_string_lossy());
                row.set_tooltip_text(Some(&file.path.to_string_lossy()));
            });
        });
        let results = gtk4::ListView::new(Some(selection.clone()), Some(factory));
        results.add_css_class("palette-results");
        let scroll = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .child(&results)
            .build();
        let status = gtk4::Label::builder()
            .label("Type a filename to search")
            .xalign(0.0)
            .wrap(true)
            .css_classes(["palette-status"])
            .build();
        let hint = gtk4::Label::builder()
            .label("↑ ↓  Select     Enter  Open     Esc  Close")
            .xalign(0.0)
            .css_classes(["palette-hint"])
            .build();
        content.append(&input_row);
        content.append(&scope);
        content.append(&scroll);
        content.append(&status);
        content.append(&hint);
        window.set_child(Some(&content));
        let palette = Rc::new(Self {
            window,
            entry,
            store,
            selection,
            scope,
            status,
            root: RefCell::new(PathBuf::new()),
            show_hidden: RefCell::new(false),
            cancel: RefCell::new(Arc::new(AtomicBool::new(false))),
            debounce: RefCell::new(None),
        });
        let weak = Rc::downgrade(&palette);
        palette.entry.connect_changed(move |_| {
            if let Some(palette) = weak.upgrade() {
                palette.queue_search();
            }
        });
        let weak = Rc::downgrade(&palette);
        close.connect_clicked(move |_| {
            if let Some(palette) = weak.upgrade() {
                palette.hide();
            }
        });
        let weak = Rc::downgrade(&palette);
        palette.window.connect_close_request(move |_| {
            if let Some(palette) = weak.upgrade() {
                palette.hide();
            }
            glib::Propagation::Stop
        });
        let weak = Rc::downgrade(&palette);
        let keys = gtk4::EventControllerKey::new();
        keys.set_propagation_phase(gtk4::PropagationPhase::Capture);
        let results_keys = results.clone();
        keys.connect_key_pressed(move |_, key, _, _| {
            let Some(palette) = weak.upgrade() else {
                return glib::Propagation::Proceed;
            };
            match key {
                gdk::Key::Escape => palette.hide(),
                gdk::Key::Up | gdk::Key::Down => {
                    let count = palette.store.n_items();
                    if count > 0 {
                        let selected = palette.selection.selected().min(count - 1);
                        palette.selection.set_selected(if key == gdk::Key::Up {
                            selected.saturating_sub(1)
                        } else {
                            (selected + 1).min(count - 1)
                        });
                        let _ = results_keys.activate_action(
                            "list.scroll-to-item",
                            Some(&palette.selection.selected().to_variant()),
                        );
                    }
                }
                _ => return glib::Propagation::Proceed,
            }
            glib::Propagation::Stop
        });
        palette.window.add_controller(keys);
        let weak = Rc::downgrade(&palette);
        palette.window.connect_destroy(move |_| {
            if let Some(palette) = weak.upgrade() {
                palette.stop();
            }
        });
        // List activation and Enter share the same action via the entry signal.
        let weak = Rc::downgrade(&palette);
        results.connect_activate(move |_, _| {
            if let Some(palette) = weak.upgrade() {
                palette.entry.emit_activate();
            }
        });
        palette
    }

    pub fn connect_activate(self: &Rc<Self>, callback: impl Fn(FileEntry) + 'static) {
        let weak = Rc::downgrade(self);
        self.entry.connect_activate(move |_| {
            if let Some(palette) = weak.upgrade()
                && let Some(file) = palette
                    .selection
                    .selected_item()
                    .and_downcast::<FileObject>()
            {
                palette.hide();
                callback(file.entry());
            }
        });
    }

    pub fn show(&self, root: PathBuf, show_hidden: bool) {
        self.stop();
        self.scope
            .set_text(&format!("Search in {} and subfolders", root.display()));
        self.scope.set_tooltip_text(Some(&root.to_string_lossy()));
        *self.root.borrow_mut() = root;
        *self.show_hidden.borrow_mut() = show_hidden;
        self.entry.set_text("");
        self.store.remove_all();
        self.status.set_text("Type a filename to search");
        self.window.present();
        self.entry.grab_focus();
    }

    pub fn hide(&self) {
        self.stop();
        self.window.set_visible(false);
    }

    fn stop(&self) {
        self.cancel.borrow().store(true, Ordering::Relaxed);
        if let Some(source) = self.debounce.borrow_mut().take() {
            source.remove();
        }
    }

    fn queue_search(self: &Rc<Self>) {
        self.stop();
        self.store.remove_all();
        let query = self.entry.text().trim().to_string();
        if query.is_empty() {
            self.status.set_text("Type a filename to search");
            return;
        }
        self.status.set_text("Searching…");
        let weak = Rc::downgrade(self);
        *self.debounce.borrow_mut() = Some(glib::timeout_add_local_once(
            Duration::from_millis(200),
            move || {
                if let Some(palette) = weak.upgrade() {
                    palette.debounce.borrow_mut().take();
                    palette.search(query);
                }
            },
        ));
    }

    fn search(self: &Rc<Self>, query: String) {
        let cancel = Arc::new(AtomicBool::new(false));
        *self.cancel.borrow_mut() = cancel.clone();
        let root = self.root.borrow().clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        recursive_search_async(root.clone(), query, cancel.clone(), sender);
        let weak = Rc::downgrade(self);
        glib::timeout_add_local(Duration::from_millis(30), move || {
            let Some(palette) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            if cancel.load(Ordering::Relaxed) {
                return glib::ControlFlow::Break;
            }
            // Bound each UI tick so a large result stream cannot starve input.
            for _ in 0..8 {
                match receiver.try_recv() {
                    Ok(ScanMessage::Batch(entries)) => {
                        let objects: Vec<_> = entries
                            .into_iter()
                            .filter(|file| {
                                *palette.show_hidden.borrow()
                                    || !file
                                        .path
                                        .strip_prefix(&root)
                                        .unwrap_or(&file.path)
                                        .components()
                                        .any(|part| {
                                            part.as_os_str().as_encoded_bytes().starts_with(b".")
                                        })
                            })
                            .map(FileObject::new)
                            .collect();
                        palette.store.extend_from_slice(&objects);
                        palette
                            .status
                            .set_text(&format!("Searching… {} matches", palette.store.n_items()));
                    }
                    Ok(ScanMessage::Completed { .. }) => {
                        let count = palette.store.n_items();
                        palette.status.set_text(&if count == 0 {
                            "No matches. Try a different filename.".to_string()
                        } else {
                            format!("{} match{}", count, if count == 1 { "" } else { "es" })
                        });
                        return glib::ControlFlow::Break;
                    }
                    Ok(ScanMessage::Error(error)) => {
                        palette.status.set_text(&error);
                        return glib::ControlFlow::Break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        return glib::ControlFlow::Break;
                    }
                }
            }
            glib::ControlFlow::Continue
        });
    }
}

impl Drop for SearchPalette {
    fn drop(&mut self) {
        self.stop();
    }
}
