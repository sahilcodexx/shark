//! GObject wrapper for FileEntry to enable GTK4 ListStore integration.

use crate::models::file_entry::FileEntry;
use glib::subclass::prelude::*;
use std::cell::RefCell;
use std::path::PathBuf;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct FileObject {
        pub entry: RefCell<Option<FileEntry>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FileObject {
        const NAME: &'static str = "SharkFileObject";
        type Type = super::FileObject;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for FileObject {}
}

glib::wrapper! {
    pub struct FileObject(ObjectSubclass<imp::FileObject>);
}

impl FileObject {
    /// Create a new FileObject from a FileEntry.
    pub fn new(entry: FileEntry) -> Self {
        let obj: Self = glib::Object::builder().build();
        *obj.imp().entry.borrow_mut() = Some(entry);
        obj
    }

    /// Access the underlying FileEntry.
    pub fn entry(&self) -> FileEntry {
        self.imp().entry.borrow().as_ref().cloned().expect("Entry must be set")
    }

    /// Borrow the underlying FileEntry to read fields without cloning.
    pub fn with_entry<R, F: FnOnce(&FileEntry) -> R>(&self, f: F) -> R {
        let borrow = self.imp().entry.borrow();
        f(borrow.as_ref().expect("Entry must be set"))
    }

    /// File display name.
    pub fn name(&self) -> String {
        self.with_entry(|e| e.name.clone())
    }

    /// Precomputed lowercase name for fast zero-allocation sorting and filtering.
    pub fn name_lower(&self) -> String {
        self.with_entry(|e| e.name_lower.clone())
    }

    /// File path.
    pub fn path(&self) -> PathBuf {
        self.with_entry(|e| e.path.clone())
    }

    /// Whether this is a directory.
    pub fn is_directory(&self) -> bool {
        self.with_entry(|e| e.is_directory)
    }

    /// Whether this file is hidden.
    pub fn is_hidden(&self) -> bool {
        self.with_entry(|e| e.is_hidden)
    }

    /// File size in bytes.
    pub fn size(&self) -> u64 {
        self.with_entry(|e| e.size)
    }

    /// Icon name.
    pub fn icon_name(&self) -> String {
        self.with_entry(|e| e.icon_name.clone())
    }

    /// Content type.
    pub fn content_type(&self) -> String {
        self.with_entry(|e| e.content_type.clone())
    }
}
