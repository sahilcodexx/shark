//! Modal dialogs (Rename, New Folder, New File, Delete Confirmation, Conflict, About) for Shark.

use crate::filesystem::ConflictResolution;
use gtk4::prelude::*;
use std::rc::Rc;

/// Show rename dialog pre-filled with the current name.
pub fn show_rename_dialog<F: Fn(String) + 'static>(
    parent: &impl IsA<gtk4::Window>,
    current_name: &str,
    on_confirm: F,
) {
    let window = gtk4::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title("Rename")
        .default_width(380)
        .resizable(false)
        .build();

    let root_box = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
    root_box.set_margin_start(20);
    root_box.set_margin_end(20);
    root_box.set_margin_top(20);
    root_box.set_margin_bottom(20);

    let title_label = gtk4::Label::builder()
        .label("Enter new name:")
        .halign(gtk4::Align::Start)
        .css_classes(["heading"])
        .build();

    let entry = gtk4::Entry::builder()
        .text(current_name)
        .hexpand(true)
        .activates_default(true)
        .build();

    // Select text excluding extension if present
    if let Some(pos) = current_name.rfind('.') {
        if pos > 0 {
            entry.select_region(0, pos as i32);
        }
    } else {
        entry.select_region(0, -1);
    }

    let error_label = gtk4::Label::builder()
        .halign(gtk4::Align::Start)
        .visible(false)
        .css_classes(["error-text"])
        .build();

    let button_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    button_box.set_halign(gtk4::Align::End);

    let cancel_btn = gtk4::Button::with_label("Cancel");
    let rename_btn = gtk4::Button::with_label("Rename");
    rename_btn.add_css_class("suggested-action");

    button_box.append(&cancel_btn);
    button_box.append(&rename_btn);

    root_box.append(&title_label);
    root_box.append(&entry);
    root_box.append(&error_label);
    root_box.append(&button_box);

    window.set_child(Some(&root_box));

    let window_clone = window.clone();
    cancel_btn.connect_clicked(move |_| {
        window_clone.close();
    });

    let on_confirm = Rc::new(on_confirm);
    let on_confirm_clone = on_confirm.clone();
    let window_clone = window.clone();
    let entry_clone = entry.clone();
    let error_clone = error_label.clone();

    let do_rename = move || {
        let text = entry_clone.text().to_string();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            error_clone.set_text("Name cannot be empty");
            error_clone.set_visible(true);
            return;
        }
        if trimmed.contains('/') || trimmed.contains('\0') {
            error_clone.set_text("Name contains invalid characters");
            error_clone.set_visible(true);
            return;
        }

        window_clone.close();
        on_confirm_clone(trimmed.to_string());
    };

    let do_rename_clone = Rc::new(do_rename);
    let do_rename_ref = do_rename_clone.clone();
    rename_btn.connect_clicked(move |_| {
        do_rename_ref();
    });

    let do_rename_ref2 = do_rename_clone.clone();
    entry.connect_activate(move |_| {
        do_rename_ref2();
    });

    window.present();
    entry.grab_focus();
}

/// Show dialog to create a new folder.
pub fn show_create_folder_dialog<F: Fn(String) + 'static>(
    parent: &impl IsA<gtk4::Window>,
    on_confirm: F,
) {
    let window = gtk4::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title("New Folder")
        .default_width(380)
        .resizable(false)
        .build();

    let root_box = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
    root_box.set_margin_start(20);
    root_box.set_margin_end(20);
    root_box.set_margin_top(20);
    root_box.set_margin_bottom(20);

    let title_label = gtk4::Label::builder()
        .label("Folder name:")
        .halign(gtk4::Align::Start)
        .css_classes(["heading"])
        .build();

    let entry = gtk4::Entry::builder()
        .text("New Folder")
        .hexpand(true)
        .activates_default(true)
        .build();
    entry.select_region(0, -1);

    let error_label = gtk4::Label::builder()
        .halign(gtk4::Align::Start)
        .visible(false)
        .css_classes(["error-text"])
        .build();

    let button_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    button_box.set_halign(gtk4::Align::End);

    let cancel_btn = gtk4::Button::with_label("Cancel");
    let create_btn = gtk4::Button::with_label("Create");
    create_btn.add_css_class("suggested-action");

    button_box.append(&cancel_btn);
    button_box.append(&create_btn);

    root_box.append(&title_label);
    root_box.append(&entry);
    root_box.append(&error_label);
    root_box.append(&button_box);

    window.set_child(Some(&root_box));

    let window_clone = window.clone();
    cancel_btn.connect_clicked(move |_| {
        window_clone.close();
    });

    let on_confirm = Rc::new(on_confirm);
    let on_confirm_clone = on_confirm.clone();
    let window_clone = window.clone();
    let entry_clone = entry.clone();
    let error_clone = error_label.clone();

    let do_create = move || {
        let text = entry_clone.text().to_string();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            error_clone.set_text("Folder name cannot be empty");
            error_clone.set_visible(true);
            return;
        }
        if trimmed.contains('/') || trimmed.contains('\0') {
            error_clone.set_text("Name contains invalid characters");
            error_clone.set_visible(true);
            return;
        }

        window_clone.close();
        on_confirm_clone(trimmed.to_string());
    };

    let do_create = Rc::new(do_create);
    let do_create_ref = do_create.clone();
    create_btn.connect_clicked(move |_| {
        do_create_ref();
    });

    let do_create_ref2 = do_create.clone();
    entry.connect_activate(move |_| {
        do_create_ref2();
    });

    window.present();
    entry.grab_focus();
}

/// Show dialog to create a new file.
pub fn show_create_file_dialog<F: Fn(String) + 'static>(
    parent: &impl IsA<gtk4::Window>,
    on_confirm: F,
) {
    let window = gtk4::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title("New File")
        .default_width(380)
        .resizable(false)
        .build();

    let root_box = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
    root_box.set_margin_start(20);
    root_box.set_margin_end(20);
    root_box.set_margin_top(20);
    root_box.set_margin_bottom(20);

    let title_label = gtk4::Label::builder()
        .label("File name:")
        .halign(gtk4::Align::Start)
        .css_classes(["heading"])
        .build();

    let entry = gtk4::Entry::builder()
        .text("New File.txt")
        .hexpand(true)
        .activates_default(true)
        .build();
    entry.select_region(0, 8);

    let error_label = gtk4::Label::builder()
        .halign(gtk4::Align::Start)
        .visible(false)
        .css_classes(["error-text"])
        .build();

    let button_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    button_box.set_halign(gtk4::Align::End);

    let cancel_btn = gtk4::Button::with_label("Cancel");
    let create_btn = gtk4::Button::with_label("Create");
    create_btn.add_css_class("suggested-action");

    button_box.append(&cancel_btn);
    button_box.append(&create_btn);

    root_box.append(&title_label);
    root_box.append(&entry);
    root_box.append(&error_label);
    root_box.append(&button_box);

    window.set_child(Some(&root_box));

    let window_clone = window.clone();
    cancel_btn.connect_clicked(move |_| {
        window_clone.close();
    });

    let on_confirm = Rc::new(on_confirm);
    let on_confirm_clone = on_confirm.clone();
    let window_clone = window.clone();
    let entry_clone = entry.clone();
    let error_clone = error_label.clone();

    let do_create = move || {
        let text = entry_clone.text().to_string();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            error_clone.set_text("File name cannot be empty");
            error_clone.set_visible(true);
            return;
        }
        if trimmed.contains('/') || trimmed.contains('\0') {
            error_clone.set_text("Name contains invalid characters");
            error_clone.set_visible(true);
            return;
        }

        window_clone.close();
        on_confirm_clone(trimmed.to_string());
    };

    let do_create = Rc::new(do_create);
    let do_create_ref = do_create.clone();
    create_btn.connect_clicked(move |_| {
        do_create_ref();
    });

    let do_create_ref2 = do_create.clone();
    entry.connect_activate(move |_| {
        do_create_ref2();
    });

    window.present();
    entry.grab_focus();
}

/// Show permanent deletion warning confirmation.
pub fn show_delete_confirmation<F: Fn() + 'static>(
    parent: &impl IsA<gtk4::Window>,
    item_count: usize,
    on_confirm: F,
) {
    let window = gtk4::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title("Permanently Delete?")
        .default_width(400)
        .resizable(false)
        .build();

    let root_box = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
    root_box.set_margin_start(20);
    root_box.set_margin_end(20);
    root_box.set_margin_top(20);
    root_box.set_margin_bottom(20);

    let message = if item_count == 1 {
        "Are you sure you want to permanently delete this item? This action cannot be undone.".to_string()
    } else {
        format!(
            "Are you sure you want to permanently delete {} items? This action cannot be undone.",
            item_count
        )
    };

    let title_label = gtk4::Label::builder()
        .label(&message)
        .wrap(true)
        .halign(gtk4::Align::Start)
        .build();

    let button_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    button_box.set_halign(gtk4::Align::End);

    let cancel_btn = gtk4::Button::with_label("Cancel");
    let delete_btn = gtk4::Button::with_label("Delete Permanently");
    delete_btn.add_css_class("destructive-action");

    button_box.append(&cancel_btn);
    button_box.append(&delete_btn);

    root_box.append(&title_label);
    root_box.append(&button_box);

    window.set_child(Some(&root_box));

    let window_clone = window.clone();
    cancel_btn.connect_clicked(move |_| {
        window_clone.close();
    });

    let window_clone = window.clone();
    delete_btn.connect_clicked(move |_| {
        window_clone.close();
        on_confirm();
    });

    window.present();
}

#[allow(dead_code)]
/// Show conflict resolution dialog when destination file exists.
pub fn show_conflict_dialog<F: Fn(ConflictResolution) + 'static>(
    parent: &impl IsA<gtk4::Window>,
    file_name: &str,
    on_choice: F,
) {
    let window = gtk4::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title("File Conflict")
        .default_width(420)
        .resizable(false)
        .build();

    let root_box = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
    root_box.set_margin_start(20);
    root_box.set_margin_end(20);
    root_box.set_margin_top(20);
    root_box.set_margin_bottom(20);

    let label = gtk4::Label::builder()
        .label(format!(
            "An item named \"{}\" already exists in the destination. What would you like to do?",
            file_name
        ))
        .wrap(true)
        .halign(gtk4::Align::Start)
        .build();

    let button_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    button_box.set_halign(gtk4::Align::End);

    let cancel_btn = gtk4::Button::with_label("Cancel");
    let skip_btn = gtk4::Button::with_label("Skip");
    let keep_both_btn = gtk4::Button::with_label("Keep Both");
    let replace_btn = gtk4::Button::with_label("Replace");
    replace_btn.add_css_class("destructive-action");

    button_box.append(&cancel_btn);
    button_box.append(&skip_btn);
    button_box.append(&keep_both_btn);
    button_box.append(&replace_btn);

    root_box.append(&label);
    root_box.append(&button_box);

    window.set_child(Some(&root_box));

    let on_choice = Rc::new(on_choice);

    let on_choice_c = on_choice.clone();
    let win_c = window.clone();
    cancel_btn.connect_clicked(move |_| {
        win_c.close();
        on_choice_c(ConflictResolution::Cancel);
    });

    let on_choice_s = on_choice.clone();
    let win_s = window.clone();
    skip_btn.connect_clicked(move |_| {
        win_s.close();
        on_choice_s(ConflictResolution::Skip);
    });

    let on_choice_k = on_choice.clone();
    let win_k = window.clone();
    keep_both_btn.connect_clicked(move |_| {
        win_k.close();
        on_choice_k(ConflictResolution::KeepBoth);
    });

    let on_choice_r = on_choice.clone();
    let win_r = window.clone();
    replace_btn.connect_clicked(move |_| {
        win_r.close();
        on_choice_r(ConflictResolution::Replace);
    });

    window.present();
}

/// Show an error message dialog.
pub fn show_error_dialog(parent: &impl IsA<gtk4::Window>, message: &str) {
    let window = gtk4::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title("Error")
        .default_width(360)
        .resizable(false)
        .build();

    let root_box = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
    root_box.set_margin_start(20);
    root_box.set_margin_end(20);
    root_box.set_margin_top(20);
    root_box.set_margin_bottom(20);

    let label = gtk4::Label::builder()
        .label(message)
        .wrap(true)
        .halign(gtk4::Align::Start)
        .build();

    let ok_btn = gtk4::Button::builder()
        .label("OK")
        .halign(gtk4::Align::End)
        .build();

    let win_c = window.clone();
    ok_btn.connect_clicked(move |_| {
        win_c.close();
    });

    root_box.append(&label);
    root_box.append(&ok_btn);
    window.set_child(Some(&root_box));
    window.present();
}

/// Show the About Shark dialog.
pub fn show_about_dialog(parent: &impl IsA<gtk4::Window>) {
    let window = gtk4::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title("About Shark")
        .default_width(360)
        .resizable(false)
        .build();

    let root_box = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    root_box.set_margin_start(24);
    root_box.set_margin_end(24);
    root_box.set_margin_top(24);
    root_box.set_margin_bottom(24);
    root_box.set_halign(gtk4::Align::Center);

    let icon = gtk4::Image::from_icon_name("system-file-manager");
    icon.set_pixel_size(64);

    let app_name = gtk4::Label::builder()
        .label("Shark File Manager")
        .css_classes(["title-1"])
        .build();

    let version = gtk4::Label::builder()
        .label("Version 0.1.0")
        .css_classes(["dim-label"])
        .build();

    let desc = gtk4::Label::builder()
        .label("A lightweight, blazingly fast Linux file manager built with Rust and GTK4.")
        .wrap(true)
        .justify(gtk4::Justification::Center)
        .build();

    let close_btn = gtk4::Button::builder()
        .label("Close")
        .margin_top(12)
        .build();

    let win_c = window.clone();
    close_btn.connect_clicked(move |_| {
        win_c.close();
    });

    root_box.append(&icon);
    root_box.append(&app_name);
    root_box.append(&version);
    root_box.append(&desc);
    root_box.append(&close_btn);

    window.set_child(Some(&root_box));
    window.present();
}
