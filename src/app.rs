//! Application setup and run loop for Shark GPUI file manager.

use crate::assets::Assets;
use crate::config::Config;
use crate::ui::MainWindow;
use gpui::{
    point, px, size, App, AppContext, Bounds, TitlebarOptions, WindowBackgroundAppearance,
    WindowBounds, WindowDecorations, WindowOptions,
};
use std::path::PathBuf;

pub struct SharkApp;

impl SharkApp {
    pub fn run(folder: Option<PathBuf>) {
        let platform = gpui_platform::current_platform(false);
        let app = gpui::Application::with_platform(platform).with_assets(Assets);

        app.run(move |cx: &mut App| {
            let config = Config::load();
            let initial_path = folder
                .or(config.last_directory)
                .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")));

            let width = config.window_width as f32;
            let height = config.window_height as f32;

            let bounds = WindowBounds::Windowed(Bounds {
                origin: point(px(100.), px(80.)),
                size: size(px(width.max(800.)), px(height.max(550.))),
            });

            let options = WindowOptions {
                window_bounds: Some(bounds),
                window_min_size: Some(size(px(320.), px(240.))),
                window_background: WindowBackgroundAppearance::Opaque,
                app_id: Some("shark".into()),
                titlebar: Some(TitlebarOptions {
                    title: Some("Shark".into()),
                    appears_transparent: true,
                    traffic_light_position: None,
                }),
                window_decorations: Some(WindowDecorations::Server),
                ..Default::default()
            };

            cx.open_window(options, move |_, cx| {
                cx.new(|cx| MainWindow::new(initial_path, cx))
            })
            .expect("Failed to open Shark window");

            cx.activate(true);
        });
    }
}
