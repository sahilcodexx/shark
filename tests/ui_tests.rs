use gtk4::prelude::*;
use shark::{
    Config,
    gui::{file_view::FileView, sidebar::Sidebar, toolbar::Toolbar},
};
use std::{cell::RefCell, rc::Rc};

#[test]
#[ignore = "requires a desktop display; run with --ignored"]
fn ui_layout_and_selection() {
    gtk4::init().expect("desktop display");
    let css = gtk4::CssProvider::new();
    css.connect_parsing_error(|_, section, error| {
        panic!("CSS error at {section:?}: {error}");
    });
    css.load_from_data(include_str!("../assets/styles/style.css"));
    let config = Rc::new(RefCell::new(Config::default()));
    let toolbar = Toolbar::new(config.clone());
    toolbar.set_path(&glib::home_dir());
    assert_eq!(toolbar.folder_title.text(), "Home");
    toolbar.view_list_btn.set_active(true);
    assert!(!toolbar.view_grid_btn.is_active());
    toolbar.view_grid_btn.set_active(true);
    assert!(!toolbar.view_list_btn.is_active());

    let sidebar = Sidebar::new(config);
    sidebar.set_current_path(&glib::home_dir());
    fn count_selected(widget: &gtk4::Widget) -> usize {
        let mut count = widget
            .downcast_ref::<gtk4::ListBoxRow>()
            .is_some_and(|row| row.is_selected()) as usize;
        let mut child = widget.first_child();
        while let Some(current) = child {
            count += count_selected(&current);
            child = current.next_sibling();
        }
        count
    }
    assert_eq!(count_selected(sidebar.container.upcast_ref()), 1);
    sidebar.set_current_path(std::path::Path::new("/"));
    assert_eq!(count_selected(sidebar.container.upcast_ref()), 1);
    let view = FileView::new();
    let scroll = view
        .stack
        .child_by_name("grid")
        .unwrap()
        .downcast::<gtk4::ScrolledWindow>()
        .unwrap();
    let grid = scroll
        .child()
        .unwrap()
        .downcast::<gtk4::GridView>()
        .unwrap();
    assert_eq!(grid.min_columns(), 1);

    let list_cont = view
        .stack
        .child_by_name("list")
        .unwrap()
        .downcast::<gtk4::Box>()
        .unwrap();
    let list_scroll = list_cont
        .last_child()
        .unwrap()
        .downcast::<gtk4::ScrolledWindow>()
        .unwrap();
    assert!(list_scroll.property::<bool>("vexpand"), "list view scroll container must vexpand to fill height");
    assert!(list_scroll.property::<bool>("hexpand"), "list view scroll container must hexpand to fill width");

    // Verify zoom controls
    let initial_zoom = *view.icon_size.borrow();
    view.zoom_in();
    assert_eq!(*view.icon_size.borrow(), initial_zoom + 16);
    view.zoom_out();
    assert_eq!(*view.icon_size.borrow(), initial_zoom);

    let entries = std::fs::read_dir(env!("CARGO_MANIFEST_DIR"))
        .unwrap()
        .take(10)
        .map(|entry| shark::FileEntry::from_path(entry.unwrap().path()).unwrap())
        .collect();
    view.add_batch(entries);
    view.selection_model.select_item(0, true);
    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    content.append(&toolbar.folder_title);
    content.append(&view.container);
    let layout = gtk4::Paned::builder()
        .start_child(&sidebar.container)
        .end_child(&content)
        .position(240)
        .resize_start_child(false)
        .build();
    let window = gtk4::Window::builder()
        .default_width(1100)
        .default_height(720)
        .child(&layout)
        .build();
    window.set_titlebar(Some(&toolbar.header_bar));
    let display = gtk4::gdk::Display::default().unwrap();
    gtk4::style_context_add_provider_for_display(
        &display,
        &css,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    let settings = gtk4::Settings::default().unwrap();
    let context = glib::MainContext::default();
    let mut backgrounds = Vec::new();
    window.present();
    for theme in ["Adwaita", "Adwaita-dark"] {
        settings.set_gtk_theme_name(Some(theme));
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(250);
        while std::time::Instant::now() < deadline {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        backgrounds.push(window.style_context().lookup_color("shark_bg").unwrap());
        assert!(grid.width() > 0);
        assert!(toolbar.header_bar.measure(gtk4::Orientation::Vertical, -1).0 <= 56,
            "the toolbar must stay compact even with the hidden location entry");
        fn check_tiles(widget: &gtk4::Widget) -> usize {
            let mut tiles = 0;
            if widget.has_css_class("file-grid-item") {
                assert!(widget.width() <= 220, "selection must stay around the card");
                assert!(
                    widget.height() <= 240,
                    "tiles must not stretch down the viewport"
                );
                tiles += 1;
            }
            let mut child = widget.first_child();
            while let Some(current) = child {
                tiles += check_tiles(&current);
                child = current.next_sibling();
            }
            tiles
        }
        assert!(check_tiles(grid.upcast_ref()) > 0);
        if let Ok(directory) = std::env::var("SHARK_UI_SNAPSHOTS") {
            std::fs::create_dir_all(&directory).unwrap();
            let paintable = gtk4::WidgetPaintable::new(Some(&window));
            let snapshot = gtk4::Snapshot::new();
            paintable.snapshot(&snapshot, window.width() as f64, window.height() as f64);
            let node = snapshot.to_node().unwrap();
            let renderer = window.renderer().unwrap();
            let texture = renderer.render_texture(&node, None);
            texture
                .save_to_png(std::path::Path::new(&directory).join(format!("{theme}.png")))
                .unwrap();
        }
    }
    assert_ne!(
        backgrounds[0], backgrounds[1],
        "palette must follow the GTK theme"
    );
    window.close();

    // Search must find a nested Downloads file without changing the browsing model.
    let root = std::env::temp_dir().join(format!("shark_palette_{}", std::process::id()));
    std::fs::create_dir_all(root.join("Downloads")).unwrap();
    let resume = root.join("Downloads/Sahil-Singh-Resume.pdf");
    std::fs::write(&resume, b"resume").unwrap();
    let app = gtk4::Application::builder()
        .application_id("com.shark.SearchTest")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.register(gio::Cancellable::NONE).unwrap();
    let parent = gtk4::ApplicationWindow::builder().application(&app).build();
    let palette = shark::gui::search_palette::SearchPalette::new(&parent);
    let opened = Rc::new(RefCell::new(None));
    let opened_callback = opened.clone();
    palette.connect_activate(move |entry| *opened_callback.borrow_mut() = Some(entry.path));
    fn result_list(widget: &gtk4::Widget) -> Option<gtk4::ListView> {
        if let Some(list) = widget.downcast_ref::<gtk4::ListView>() {
            return Some(list.clone());
        }
        let mut child = widget.first_child();
        while let Some(current) = child {
            if let Some(list) = result_list(&current) {
                return Some(list);
            }
            child = current.next_sibling();
        }
        None
    }
    palette.show(root.clone(), false);
    let results = result_list(palette.window.upcast_ref())
        .unwrap()
        .model()
        .unwrap();
    let browsing_count = view.item_count();
    palette.entry.set_text("wrong-query");
    palette.entry.set_text("sahil-singh-resume.pdf");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while results.n_items() == 0 && std::time::Instant::now() < deadline {
        while context.pending() {
            context.iteration(false);
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(results.n_items(), 1);
    result_list(palette.window.upcast_ref())
        .unwrap()
        .activate_action("list.scroll-to-item", Some(&0u32.to_variant()))
        .unwrap();
    assert_eq!(view.item_count(), browsing_count);
    let file = results
        .item(0)
        .unwrap()
        .downcast::<shark::FileObject>()
        .unwrap();
    assert_eq!(file.entry().path, resume);
    if let Ok(directory) = std::env::var("SHARK_UI_SNAPSHOTS") {
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(150);
        while std::time::Instant::now() < deadline {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let paintable = gtk4::WidgetPaintable::new(Some(&palette.window));
        let snapshot = gtk4::Snapshot::new();
        paintable.snapshot(
            &snapshot,
            palette.window.width() as f64,
            palette.window.height() as f64,
        );
        let node = snapshot.to_node().unwrap();
        palette
            .window
            .renderer()
            .unwrap()
            .render_texture(&node, None)
            .save_to_png(std::path::Path::new(&directory).join("search.png"))
            .unwrap();
    }
    palette.entry.emit_activate();
    assert_eq!(*opened.borrow(), Some(resume));
    assert!(!palette.window.is_visible());
    palette.show(root.clone(), false);
    assert_eq!(results.n_items(), 0);
    palette.entry.set_text("resume");
    palette.hide();
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(300);
    while std::time::Instant::now() < deadline {
        while context.pending() {
            context.iteration(false);
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(
        results.n_items(),
        0,
        "closing search must cancel pending work"
    );
    parent.close();
    std::fs::remove_dir_all(root).unwrap();
}
