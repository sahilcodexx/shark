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
            cx.set_prompt_builder(crate::ui::prompt::render_prompt);
            let config = Config::load();
            let initial_path = folder
                .or_else(|| config.last_directory.clone())
                .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")));

            let width = config.window_width as f32;
            let height = config.window_height as f32;

            let bounds = WindowBounds::Windowed(Bounds {
                origin: point(px(100.), px(80.)),
                size: size(px(width.max(800.)), px(height.max(550.))),
            });

            let is_frosted = config.background_mode != "solid";

            let icon = image::load_from_memory_with_format(
                include_bytes!("../assets/icons/shark.png"),
                image::ImageFormat::Png,
            )
            .map(|image| std::sync::Arc::new(image.thumbnail(128, 128).into_rgba8()))
            .inspect_err(|error| log::warn!("Could not decode the Shark window icon: {error}"))
            .ok();

            let options = WindowOptions {
                window_bounds: Some(bounds),
                window_min_size: Some(size(px(320.), px(240.))),
                window_background: if is_frosted {
                    WindowBackgroundAppearance::Transparent
                } else {
                    WindowBackgroundAppearance::Opaque
                },
                app_id: Some("shark".into()),
                icon,
                titlebar: Some(TitlebarOptions {
                    title: Some("Shark".into()),
                    appears_transparent: true,
                    traffic_light_position: None,
                }),
                window_decorations: Some(WindowDecorations::Server),
                ..Default::default()
            };

            cx.open_window(options, move |window, cx| {
                let view = cx.new(|cx| MainWindow::new(initial_path, config, cx));
                let weak = view.downgrade();
                window.on_next_frame(move |window, cx| {
                    let _ = weak.update(cx, |view, _| view.finish_startup(window));
                });
                view
            })
            .expect("Failed to open Shark window");

            cx.activate(true);
        });
    }
}
