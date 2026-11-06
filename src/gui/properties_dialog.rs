//! File and directory properties dialog for Shark.

use crate::models::FileEntry;
use gtk4::prelude::*;

/// Display file properties modal dialog.
pub fn show_properties_dialog(parent: &impl IsA<gtk4::Window>, entry: &FileEntry) {
    let window = gtk4::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(format!("Properties: {}", entry.name))
        .default_width(420)
        .resizable(false)
        .build();

    let root_box = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
    root_box.set_margin_start(24);
    root_box.set_margin_end(24);
    root_box.set_margin_top(20);
    root_box.set_margin_bottom(20);

    // Header with large icon and file name
    let header_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 16);
    let icon = gtk4::Image::from_icon_name(&entry.icon_name);
    icon.set_pixel_size(56);

    let title_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    let name_label = gtk4::Label::builder()
        .label(&entry.name)
        .selectable(true)
        .halign(gtk4::Align::Start)
        .css_classes(["title-2"])
        .ellipsize(gtk4::pango::EllipsizeMode::End)
        .max_width_chars(25)
        .build();

    let type_sub = gtk4::Label::builder()
        .label(entry.type_description())
        .halign(gtk4::Align::Start)
        .css_classes(["dim-label"])
        .build();

    title_box.append(&name_label);
    title_box.append(&type_sub);

    header_box.append(&icon);
    header_box.append(&title_box);

    let separator = gtk4::Separator::new(gtk4::Orientation::Horizontal);

    // Properties grid
    let grid = gtk4::Grid::builder()
        .row_spacing(10)
        .column_spacing(16)
        .build();

    let add_row = |grid: &gtk4::Grid, row: i32, key: &str, val: &str| {
        let key_label = gtk4::Label::builder()
            .label(key)
            .halign(gtk4::Align::End)
            .css_classes(["dim-label"])
            .build();
        let val_label = gtk4::Label::builder()
            .label(val)
            .selectable(true)
            .halign(gtk4::Align::Start)
            .hexpand(true)
            .ellipsize(gtk4::pango::EllipsizeMode::Middle)
            .max_width_chars(32)
            .build();

        grid.attach(&key_label, 0, row, 1, 1);
        grid.attach(&val_label, 1, row, 1, 1);
    };

    let parent_path = entry.path.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "/".to_string());
    let size_str = if entry.is_directory {
        "Folder".to_string()
    } else {
        format!("{} ({} bytes)", entry.formatted_size(), entry.size)
    };

    add_row(&grid, 0, "Type:", &entry.type_description());
    add_row(&grid, 1, "MIME Type:", &entry.content_type);
    add_row(&grid, 2, "Location:", &parent_path);
    add_row(&grid, 3, "Size:", &size_str);
    add_row(&grid, 4, "Modified:", &entry.formatted_modified());

    if let Some(created) = entry.created {
        add_row(&grid, 5, "Created:", &crate::utils::format_date(Some(created)));
    }

    add_row(&grid, 6, "Permissions:", &format!("{} ({:o})", entry.formatted_permissions(), entry.permissions & 0o777));

    if !entry.owner.is_empty() {
        let owner_str = if !entry.group.is_empty() {
            format!("{}:{}", entry.owner, entry.group)
        } else {
            entry.owner.clone()
        };
        add_row(&grid, 7, "Owner:", &owner_str);
    }

    let close_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    close_box.set_halign(gtk4::Align::End);
    let close_btn = gtk4::Button::with_label("Close");

    let win_c = window.clone();
    close_btn.connect_clicked(move |_| {
        win_c.close();
    });
    close_box.append(&close_btn);

    root_box.append(&header_box);
    root_box.append(&separator);
    root_box.append(&grid);
    root_box.append(&close_box);

    window.set_child(Some(&root_box));
    window.present();
}
