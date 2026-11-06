//! Application setup and lifecycle for the Shark file manager.

use crate::gui::SharkWindow;
use gtk4::gdk;
use gtk4::prelude::*;

pub struct SharkApp {
    app: gtk4::Application,
}

impl Default for SharkApp {
    fn default() -> Self {
        Self::new()
    }
}

impl SharkApp {
    pub fn new() -> Self {
        let app = gtk4::Application::builder()
            .application_id("com.shark.filemanager")
            .flags(gio::ApplicationFlags::HANDLES_OPEN)
            .build();

        app.connect_startup(|_| {
            Self::load_styles();
        });

        app.connect_activate(|app| {
            let shark_win = SharkWindow::new(app);
            shark_win.window.present();
        });

        app.connect_open(|app, files, _| {
            if let Some(file) = files.first()
                && let Some(path) = file.path() {
                    let shark_win = SharkWindow::new(app);
                    shark_win.navigate_to(path);
                    shark_win.window.present();
                    return;
                }
            let shark_win = SharkWindow::new(app);
            shark_win.window.present();
        });

        SharkApp { app }
    }

    /// Load the application CSS stylesheet.
    fn load_styles() {
        let provider = gtk4::CssProvider::new();

        // Embed the stylesheet directly to guarantee it is always available
        const STYLE_CSS: &str = include_str!("../assets/styles/style.css");
        provider.load_from_data(STYLE_CSS);

        if let Some(display) = gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }

    pub fn run(&self) -> glib::ExitCode {
        self.app.run()
    }
}
