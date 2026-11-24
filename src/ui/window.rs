//! Main application window layout and controller for Shark GPUI.

use crate::filesystem::DirectoryCache;
use crate::filesystem::operations::{open_in_terminal, open_path};
use crate::filesystem::scanner::{scan_directory_async, ScanMessage};
use crate::models::file_entry::FileEntry;
use crate::navigation::history::NavigationHistory;
use crate::theme::{
    self, button, icon, icon_button, macos_segmented_item, macos_segmented_track, theme_button, theme_chip,
    ButtonLook, RADIUS_CARD, RADIUS_ROW, RADIUS_SIDEBAR, RADIUS_THUMB, TITLE_BAR_HEIGHT,
};
use crate::thumbnails::ThumbnailCache;
use gpui::{
    actions, div, img, prelude::*, px, uniform_list, AnimationExt, AnyElement, ClickEvent, Context, CursorStyle, Div, ElementId,
    FocusHandle, Hsla, IntoElement, KeyDownEvent, MouseButton, MouseMoveEvent, Render, ResizeEdge, Role, ScrollWheelEvent, SharedString,
    Styled, UniformListScrollHandle, Window,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Grid,
    List,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Name,
    Size,
    Modified,
    Type,
}

actions!(
    shark,
    [
        Back,
        Forward,
        Up,
        ToggleInspector,
        ToggleViewMode,
        ZoomIn,
        ZoomOut,
        SelectNext,
        SelectPrev,
        OpenSelected,
    ]
);

#[derive(Clone, Debug)]
pub struct PaletteItem {
    pub section: &'static str,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon_name: &'static str,
    pub shortcut: Option<&'static str>,
    pub action: PaletteAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeFilter {
    All,
    Folders,
    Documents,
    Images,
    Media,
    Code,
    Archives,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateFilter {
    Any,
    Today,
    Last7Days,
    Last30Days,
    ThisYear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeFilter {
    Any,
    Small,  // < 1 MB
    Medium, // 1 MB - 100 MB
    Large,  // > 100 MB
}

#[derive(Clone, Debug)]
pub enum PaletteAction {
    Navigate(PathBuf),
    NewFolder,
    ToggleViewMode,
    ToggleHidden,
    ToggleInspector,
    OpenTerminal,
    OpenSettings,
    Refresh,
    OpenFile(PathBuf),
    SetTypeFilter(TypeFilter),
    SetDateFilter(DateFilter),
    SetSizeFilter(SizeFilter),
    ResetFilters,
    SetTheme(theme::ThemeMode),
}

pub struct MainWindow {
    pub current_path: PathBuf,
    pub history: NavigationHistory,
    pub entries: Vec<FileEntry>,
    pub filtered_indices: Vec<usize>,
    pub search_query: String,
    pub selected_index: Option<usize>,
    tags: std::collections::BTreeMap<PathBuf, Vec<String>>,
    active_tag: Option<&'static str>,
    scan_error: Option<String>,
    pub view_mode: ViewMode,
    pub sort_by: SortBy,
    pub sort_reversed: bool,
    pub sort_folders_first: bool,
    pub type_filter: TypeFilter,
    pub date_filter: DateFilter,
    pub size_filter: SizeFilter,
    pub show_filter_menu: bool,
    pub show_more_menu: bool,
    pub enable_thumbnails: bool,
    pub confirm_delete: bool,
    pub show_inspector: bool,
    pub zoom: f32,
    pub scroll_handle: UniformListScrollHandle,
    pub thumbnails: ThumbnailCache,
    pub dir_cache: DirectoryCache,
    pub scan_cancellable: Arc<AtomicBool>,
    pub scan_receiver: Option<Receiver<ScanMessage>>,
    pub total_items: usize,
    pub total_bytes: u64,
    pub focus_handle: FocusHandle,
    pub grid_cols: usize,
    pub show_hidden: bool,
    pub show_settings_modal: bool,
    pub show_command_palette: bool,
    pub command_palette_query: String,
    pub command_palette_selected: usize,
    pub command_palette_has_query: bool,
    pub command_palette_search_epoch: usize,
    pub search_active: bool,
    pub path_editing: bool,
    pub path_input_text: String,
    pub theme_mode: theme::ThemeMode,
    pub active_theme: theme::ThemeColors,
    pub font_size: f32,
    pub background_mode: String,
    pub blur_coordinator: Option<crate::platform::PlatformBlurCoordinator>,
    pub window_size: (i32, i32),
    appearance_subscription: Option<gpui::Subscription>,
    bounds_subscription: Option<gpui::Subscription>,
}

impl MainWindow {
    pub fn new(
        initial_path: PathBuf,
        blur_coordinator: Option<crate::platform::PlatformBlurCoordinator>,
        cx: &mut Context<Self>,
    ) -> Self {
        let cancellable = Arc::new(AtomicBool::new(false));
        let history = NavigationHistory::new(initial_path.clone());
        let dir_cache = DirectoryCache::new();
        dir_cache.warm_favorites();

        let thumbnails = ThumbnailCache::new();
        let thumb_rx = thumbnails.ready_receiver();

        // Listen for completed background thumbnails to re-render rows smoothly
        cx.spawn(async move |this, cx| {
            while let Ok(_) = thumb_rx.recv().await {
                let _ = this.update(cx, |_, cx| {
                    cx.notify();
                });
            }
        })
        .detach();

        let cfg = crate::config::Config::load();
        let is_frosted = cfg.background_mode != "solid";
        let theme_mode = match cfg.theme.as_str() {
            "light" => theme::ThemeMode::Light,
            "dark" => theme::ThemeMode::Dark,
            _ => theme::ThemeMode::System,
        };
        let initial_theme = match theme_mode {
            theme::ThemeMode::Light => theme::ThemeColors::light(is_frosted),
            _ => theme::ThemeColors::dark(is_frosted),
        };

        let mut win = Self {
            current_path: initial_path.clone(),
            history,
            entries: Vec::new(),
            filtered_indices: Vec::new(),
            search_query: String::new(),
            selected_index: None,
            tags: cfg.tags,
            active_tag: None,
            scan_error: None,
            view_mode: ViewMode::Grid,
            sort_by: SortBy::Name,
            sort_reversed: false,
            sort_folders_first: true,
            type_filter: TypeFilter::All,
            date_filter: DateFilter::Any,
            size_filter: SizeFilter::Any,
            show_filter_menu: false,
            show_more_menu: false,
            enable_thumbnails: true,
            confirm_delete: true,
            show_inspector: false,
            zoom: 110.,
            scroll_handle: UniformListScrollHandle::new(),
            thumbnails,
            dir_cache,
            scan_cancellable: cancellable,
            scan_receiver: None,
            total_items: 0,
            total_bytes: 0,
            focus_handle: cx.focus_handle(),
            grid_cols: 8,
            show_hidden: false,
            show_settings_modal: false,
            show_command_palette: false,
            command_palette_query: String::new(),
            command_palette_selected: 0,
            command_palette_has_query: false,
            command_palette_search_epoch: 0,
            search_active: false,
            path_editing: false,
            path_input_text: String::new(),
            theme_mode,
            active_theme: initial_theme,
            font_size: cfg.font_size,
            background_mode: cfg.background_mode.clone(),
            blur_coordinator,
            window_size: (cfg.window_width, cfg.window_height),
            appearance_subscription: None,
            bounds_subscription: None,
        };

        win.navigate_to(initial_path, cx);
        win
    }

    #[inline]
    pub fn entry_at(&self, idx: usize) -> Option<&FileEntry> {
        self.filtered_indices.get(idx).and_then(|&i| self.entries.get(i))
    }

    #[inline]
    pub fn filtered_count(&self) -> usize {
        self.filtered_indices.len()
    }

    /// Navigate to a new directory path and start scanning.
    pub fn navigate_to(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        // Cancel any active directory scanner
        self.scan_cancellable.store(true, Ordering::Relaxed);
        let new_cancellable = Arc::new(AtomicBool::new(false));
        self.scan_cancellable = new_cancellable.clone();

        self.history.navigate_to(path.clone());
        self.current_path = path.clone();
        self.selected_index = None;
        self.active_tag = None;
        self.scan_error = None;
        self.search_query.clear();
        self.show_filter_menu = false;
        self.show_more_menu = false;
        self.scroll_handle = UniformListScrollHandle::new();

        // 1. Instant zero-latency display from DirectoryCache (KDE Dolphin / GNOME Files style)
        if let Some(cached) = self.dir_cache.get(&path) {
            self.entries = (*cached.entries).clone();
            self.total_items = cached.total_items;
            self.total_bytes = cached.total_bytes;
            self.sort_and_filter();
            cx.notify();

            // If the folder on disk hasn't modified, we're completely done in 0ms!
            if self.dir_cache.is_fresh(&path) {
                return;
            }
        } else {
            // Uncached directory: reset entries to receive progressive stream
            self.entries.clear();
            self.filtered_indices.clear();
            self.total_items = 0;
            self.total_bytes = 0;
            cx.notify();
        }

        let (sender, receiver) = channel();
        self.scan_receiver = Some(receiver);

        // High-speed POSIX getdents64 background scanner
        scan_directory_async(path, new_cancellable, sender);

        // Start async task to receive batches reactively
        self.poll_scanner(cx);
    }

    fn poll_scanner(&mut self, cx: &mut Context<Self>) {
        let cancellable = self.scan_cancellable.clone();

        cx.spawn(async move |this, cx| {
            loop {
                if cancellable.load(Ordering::Relaxed) {
                    break;
                }

                // Check messages from scanner
                let mut batches = Vec::new();
                let mut completed = None;
                let mut error = None;

                let should_continue = this.update(cx, |win, _| {
                    if let Some(ref rx) = win.scan_receiver {
                        while let Ok(msg) = rx.try_recv() {
                            match msg {
                                ScanMessage::Batch(batch) => batches.push(batch),
                                ScanMessage::Completed { total_items, total_bytes } => {
                                    completed = Some((total_items, total_bytes));
                                }
                                ScanMessage::Error(message) => error = Some(message),
                            }
                        }
                    }
                    !cancellable.load(Ordering::Relaxed)
                });

                if should_continue.is_err() || !should_continue.unwrap() {
                    break;
                }

                let finished = completed.is_some() || error.is_some();
                if !batches.is_empty() || finished {
                    let _ = this.update(cx, |win, cx| {
                        win.scan_error = error;
                        for batch in batches {
                            win.entries.extend(batch);
                        }
                        if let Some((items, bytes)) = completed {
                            win.total_items = items;
                            win.total_bytes = bytes;
                            // Update directory cache
                            win.dir_cache.insert(
                                win.current_path.clone(),
                                win.entries.clone(),
                                items,
                                bytes,
                            );
                        }
                        win.sort_and_filter();
                        cx.notify();
                    });
                }

                if finished {
                    break;
                }

                // Smooth 8ms polling during scanning
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(8))
                    .await;
            }
        })
        .detach();
    }

    pub fn has_active_filters(&self) -> bool {
        self.type_filter != TypeFilter::All
            || self.date_filter != DateFilter::Any
            || self.size_filter != SizeFilter::Any
    }

    pub fn reset_filters(&mut self, cx: &mut Context<Self>) {
        self.type_filter = TypeFilter::All;
        self.date_filter = DateFilter::Any;
        self.size_filter = SizeFilter::Any;
        self.sort_and_filter();
        cx.notify();
    }

    /// Sort and filter entries in-place using zero-allocation index mapping.
    pub fn sort_and_filter(&mut self) {
        let sort_by = self.sort_by;
        let reversed = self.sort_reversed;
        let q = self.search_query.trim().to_lowercase();
        let show_hidden = self.show_hidden;
        let type_filter = self.type_filter;
        let date_filter = self.date_filter;
        let size_filter = self.size_filter;

        self.filtered_indices.clear();
        for (i, e) in self.entries.iter().enumerate() {
            if !show_hidden && e.is_hidden {
                continue;
            }
            if !q.is_empty() && !e.name_lower.contains(&q) {
                continue;
            }

            if let Some(tag) = self.active_tag
                && !self.tags.get(&e.path).is_some_and(|tags| tags.iter().any(|value| value == tag))
            {
                continue;
            }

            // Type filter
            match type_filter {
                TypeFilter::All => {}
                TypeFilter::Folders => {
                    if !e.is_directory { continue; }
                }
                TypeFilter::Documents => {
                    if e.is_directory || (!e.content_type.starts_with("text/") && e.content_type != "application/pdf") {
                        continue;
                    }
                }
                TypeFilter::Images => {
                    if e.is_directory || !e.content_type.starts_with("image/") {
                        continue;
                    }
                }
                TypeFilter::Media => {
                    if e.is_directory || (!e.content_type.starts_with("video/") && !e.content_type.starts_with("audio/")) {
                        continue;
                    }
                }
                TypeFilter::Code => {
                    if e.is_directory || (!e.content_type.starts_with("text/x-")
                        && !e.content_type.contains("javascript")
                        && !e.content_type.contains("json")
                        && !e.content_type.contains("toml")
                        && !e.content_type.contains("yaml")
                        && e.content_type != "text/html")
                    {
                        continue;
                    }
                }
                TypeFilter::Archives => {
                    if e.is_directory || (!e.content_type.contains("archive") && !e.content_type.contains("zip") && !e.content_type.contains("tar")) {
                        continue;
                    }
                }
            }

            // Size filter
            match size_filter {
                SizeFilter::Any => {}
                SizeFilter::Small => {
                    if e.is_directory || e.size >= 1024 * 1024 { continue; }
                }
                SizeFilter::Medium => {
                    if e.is_directory || e.size < 1024 * 1024 || e.size > 100 * 1024 * 1024 { continue; }
                }
                SizeFilter::Large => {
                    if e.is_directory || e.size <= 100 * 1024 * 1024 { continue; }
                }
            }

            // Date Modified filter
            if date_filter != DateFilter::Any {
                if let Some(mod_time) = e.modified {
                    if let Ok(elapsed) = mod_time.elapsed() {
                        let secs = elapsed.as_secs();
                        let pass = match date_filter {
                            DateFilter::Any => true,
                            DateFilter::Today => secs <= 86400,
                            DateFilter::Last7Days => secs <= 7 * 86400,
                            DateFilter::Last30Days => secs <= 30 * 86400,
                            DateFilter::ThisYear => secs <= 365 * 86400,
                        };
                        if !pass { continue; }
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            self.filtered_indices.push(i);
        }

        // Sort indices by referencing entries directly (ZERO CLONING)
        let entries = &self.entries;
        let sort_folders_first = self.sort_folders_first;

        self.filtered_indices.sort_by(|&a_idx, &b_idx| {
            let a = &entries[a_idx];
            let b = &entries[b_idx];

            if sort_folders_first && a.is_directory != b.is_directory {
                return b.is_directory.cmp(&a.is_directory);
            }

            let cmp = match sort_by {
                SortBy::Name => a.name_lower.cmp(&b.name_lower),
                SortBy::Size => a.size.cmp(&b.size),
                SortBy::Modified => a.modified.cmp(&b.modified),
                SortBy::Type => a.content_type.cmp(b.content_type),
            };

            if reversed {
                cmp.reverse()
            } else {
                cmp
            }
        });
    }

    pub fn commit_path_input(&mut self, cx: &mut Context<Self>) {
        let raw = self.path_input_text.trim();
        let expanded = if raw.starts_with("~/") || raw == "~" {
            if let Some(home) = dirs::home_dir() {
                if raw == "~" {
                    home
                } else {
                    home.join(&raw[2..])
                }
            } else {
                PathBuf::from(raw)
            }
        } else {
            PathBuf::from(raw)
        };

        if expanded.exists() {
            if expanded.is_dir() {
                self.navigate_to(expanded, cx);
            } else {
                open_path(&expanded);
            }
        }
        self.path_editing = false;
        cx.notify();
    }

    pub fn create_new_folder(&mut self, cx: &mut Context<Self>) {
        use crate::filesystem::operations::get_unique_path;
        let target = get_unique_path(&self.current_path, "New Folder");
        let name = target.file_name().unwrap().to_string_lossy().to_string();

        if crate::filesystem::operations::create_folder(&self.current_path, &name).is_ok() {
            self.dir_cache.invalidate(&self.current_path);
            self.refresh(cx);
        }
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.dir_cache.invalidate(&self.current_path);
        self.navigate_to(self.current_path.clone(), cx);
    }

    pub fn open_selected(&mut self, cx: &mut Context<Self>) {
        if let Some(idx) = self.selected_index {
            if let Some(entry) = self.entry_at(idx) {
                if entry.is_directory {
                    let target = entry.path.clone();
                    self.navigate_to(target, cx);
                } else {
                    open_path(&entry.path);
                }
            }
        }
    }

    pub fn go_back(&mut self, cx: &mut Context<Self>) {
        if self.history.can_go_back() {
            if let Some(prev) = self.history.go_back() {
                self.navigate_to(prev, cx);
            }
        }
    }

    pub fn go_forward(&mut self, cx: &mut Context<Self>) {
        if self.history.can_go_forward() {
            if let Some(next) = self.history.go_forward() {
                self.navigate_to(next, cx);
            }
        }
    }

    pub fn go_up(&mut self, cx: &mut Context<Self>) {
        if let Some(parent) = self.current_path.parent().map(Path::to_path_buf) {
            self.navigate_to(parent, cx);
        }
    }

    pub fn set_view_mode(&mut self, mode: ViewMode, cx: &mut Context<Self>) {
        self.view_mode = mode;
        cx.notify();
    }

    pub fn toggle_inspector(&mut self, cx: &mut Context<Self>) {
        self.show_inspector = !self.show_inspector;
        cx.notify();
    }

    pub fn toggle_sort(&mut self, column: SortBy, cx: &mut Context<Self>) {
        if self.sort_by == column {
            self.sort_reversed = !self.sort_reversed;
        } else {
            self.sort_by = column;
            self.sort_reversed = false;
        }
        self.sort_and_filter();
        cx.notify();
    }

    pub fn zoom_in(&mut self, cx: &mut Context<Self>) {
        self.zoom = (self.zoom + 20.).min(300.);
        cx.notify();
    }

    pub fn zoom_out(&mut self, cx: &mut Context<Self>) {
        self.zoom = (self.zoom - 20.).max(60.);
        cx.notify();
    }

    pub fn save_config(&self) {
        let mut config = crate::config::Config::load();
        config.show_hidden = self.show_hidden;
        config.view_mode = match self.view_mode {
            ViewMode::Grid => "grid",
            ViewMode::List => "list",
        }.to_string();
        config.sort_by = match self.sort_by {
            SortBy::Name => "name",
            SortBy::Size => "size",
            SortBy::Modified => "modified",
            SortBy::Type => "type",
        }.to_string();
        config.sort_reversed = self.sort_reversed;
        config.font_size = self.font_size;
        config.background_mode = self.background_mode.clone();
        config.save();
    }

    // --- Sub-renderers ---

    fn render_header(&self, theme: &theme::ThemeColors, cx: &mut Context<Self>) -> Div {
        let can_back = self.history.can_go_back();
        let can_forward = self.history.can_go_forward();
        let window_width = self.window_size.0 as f32;
        let location_center = window_width / 2. - theme::SIDEBAR_PANEL_WIDTH - 20.;
        // Reserve room for the right toolbar cluster before sizing the centered pill.
        let location_width = (window_width - 920.).clamp(0., 460.);
        let divider = || {
            div()
                .w(px(1.))
                .h(px(14.))
                .bg(theme.button_border)
        };

        div()
            .relative()
            .h(px(TITLE_BAR_HEIGHT))
            .flex_shrink_0()
            .px_3()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            // Left navigation cluster: Back / Forward in joint capsule + Folder title
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        theme::macos_capsule_container("nav-segment", theme)
                            .child(
                                theme::macos_joint_button("nav-back", "chevron-left", false, can_back, theme)
                                    .on_click(cx.listener(|this, _, _, cx| this.go_back(cx))),
                            )
                            .child(divider())
                            .child(
                                theme::macos_joint_button("nav-forward", "chevron-right", false, can_forward, theme)
                                    .on_click(cx.listener(|this, _, _, cx| this.go_forward(cx))),
                            ),
                    )
            )
            .child(
                div().flex_1().min_w_0().max_w(px(460.))
                    .when(location_width >= 180., |d| {
                        d.absolute()
                            .left(px(location_center - location_width / 2.))
                            .top(px((TITLE_BAR_HEIGHT - theme::BUTTON_HEIGHT) / 2.))
                            .w(px(location_width))
                    })
                    .child(self.render_location_bar(theme, cx)),
            )
            // Right controls cluster: Joint Capsules + Search Bar (matching macOS Finder in Image 2)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2p5()
                    // 1. Segmented View Mode Switcher [ ⊞ | ≡ ]
                    .child(
                        theme::macos_joint_container("view-switcher-seg", theme)
                            .child(
                                theme::macos_joint_button("view-grid", "grid", self.view_mode == ViewMode::Grid, true, theme)
                                    .on_click(cx.listener(|this, _, _, cx| this.set_view_mode(ViewMode::Grid, cx))),
                            )
                            .child(divider())
                            .child(
                                theme::macos_joint_button("view-list", "list", self.view_mode == ViewMode::List, true, theme)
                                    .on_click(cx.listener(|this, _, _, cx| this.set_view_mode(ViewMode::List, cx))),
                            ),
                    )
                    // 2. Joint action capsule: [ ⇧ | 🏷 | ⋯ ] (Share, Tag/Filter, More grouped in ONE capsule)
                    .child(
                        theme::macos_joint_container("joint-actions-capsule", theme)
                            .child(
                                theme::macos_joint_button("joint-btn-share", "share", false, true, theme)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        let path_str = if let Some(e) = this.selected_index.and_then(|i| this.entry_at(i)) {
                                            e.path.to_string_lossy().to_string()
                                        } else {
                                            this.current_path.to_string_lossy().to_string()
                                        };
                                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(path_str));
                                    })),
                            )
                            .child(divider())
                            .child(
                                theme::macos_joint_button(
                                    "joint-btn-filter",
                                    "tag",
                                    self.show_filter_menu || self.has_active_filters(),
                                    true,
                                    theme,
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.show_filter_menu = !this.show_filter_menu;
                                    this.show_more_menu = false;
                                    cx.notify();
                                })),
                            )
                            .child(divider())
                            .child(
                                theme::macos_joint_button(
                                    "joint-btn-more",
                                    "more-horizontal",
                                    self.show_more_menu,
                                    true,
                                    theme,
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.show_more_menu = !this.show_more_menu;
                                    this.show_filter_menu = false;
                                    cx.notify();
                                })),
                            ),
                    )
                    // 3. macOS Finder-style search capsule
                    .child(
                        div()
                            .id("macos-search-bar")
                            .w(px(230.))
                            .min_w(px(150.))
                            .flex_shrink_1()
                            .h(px(theme::BUTTON_HEIGHT))
                            .px_3()
                            .bg(theme.search_fill)
                            .rounded_full()
                            .border_1()
                            .border_color(theme.search_border)
                            .shadow(theme::control_shadow(theme))
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .gap_2()
                            .hover(|s| s.bg(theme.button_hover))
                            .active(|s| s.bg(theme.sidebar_selected))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_command_palette(None, cx);
                            }))
                            .child(theme::icon("search", 13.).text_color(theme.text_muted))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(theme::TEXT_SMALL))
                                    .text_color(theme.text_muted)
                                    .child(if self.search_query.is_empty() { "Search".to_string() } else { self.search_query.clone() }),
                            ),
                    ),
            )
    }

    fn render_location_bar(&self, theme: &theme::ThemeColors, cx: &mut Context<Self>) -> AnyElement {
        if self.path_editing {
            return div()
                .h(px(theme::BUTTON_HEIGHT))
                .px_2p5()
                .bg(theme.search_fill)
                .rounded_full()
                .border_1()
                .border_color(theme.accent)
                .shadow(theme::control_shadow(theme))
                .flex()
                .items_center()
                .gap_2()
                .child(theme::icon("hard-drive", 14.).text_color(theme.accent))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(self.font_size.max(14.)))
                        .text_color(theme.text_primary)
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .min_w_0()
                        .truncate()
                        .child(if self.path_input_text.is_empty() { "Enter a folder path…".to_string() } else { format!("{}|", self.path_input_text) }),
                )
                .child(
                    div()
                        .id("path-commit-btn")
                        .role(Role::Button)
                        .cursor_pointer()
                        .p_1()
                        .rounded_full()
                        .hover(|s| s.bg(theme.button_hover))
                        .on_click(cx.listener(|this, _, _, cx| this.commit_path_input(cx)))
                        .child(theme::icon("check-hi", 13.).text_color(theme.accent)),
                )
                .child(
                    div()
                        .id("path-cancel-btn")
                        .role(Role::Button)
                        .cursor_pointer()
                        .p_1()
                        .rounded_full()
                        .hover(|s| s.bg(theme.button_hover))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.path_editing = false;
                            cx.notify();
                        }))
                        .child(theme::icon("cancel-01", 13.).text_color(theme.text_muted)),
                ).into_any_element();
        }

        div()
            .id("location-pill")
            .role(Role::Button)
            .aria_label("Edit location")
            .h(px(theme::BUTTON_HEIGHT))
            .px_3()
            .rounded_full()
            .bg(theme.search_fill)
            .border_1()
            .border_color(theme.search_border)
            .flex()
            .items_center()
            .gap_2()
            .cursor_pointer()
            .hover(|style| style.bg(theme.button_hover))
            .on_click(cx.listener(|this, _, _, cx| {
                this.path_editing = true;
                this.search_active = false;
                this.path_input_text = this.current_path.to_string_lossy().into_owned();
                cx.notify();
            }))
            .child(icon("hard-drive", 14.).flex_shrink_0().text_color(theme.text_muted))
            .child(div().flex_1().min_w_0().truncate()
                .text_size(px(self.font_size.max(14.)))
                .text_color(theme.text_primary)
                .child(self.current_path.to_string_lossy().into_owned()))
            .child(icon("pencil", 13.).flex_shrink_0().text_color(theme.text_muted))
            .into_any_element()
    }

    fn render_settings_modal(
        &self,
        theme: &theme::ThemeColors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .id("settings-modal-backdrop")
            .absolute()
            .inset_0()
            .bg(gpui::rgba(0x00000055))
            .flex()
            .items_center()
            .justify_center()
            .p_4()
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(|this, _, _, cx| {
                this.show_settings_modal = false;
                cx.notify();
            }))
            .child(
                div()
                    .id("settings-modal-content")
                    .w(px(560.))
                    .max_w_full()
                    .max_h_full()
                    .bg(if theme.is_dark {
                        gpui::rgb(0x202126)
                    } else {
                        gpui::rgb(0xf5f5f7)
                    })
                    .border_1()
                    .border_color(theme.surface_border)
                    .rounded(px(14.))
                    .shadow(theme::floating_panel_shadows(theme))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .h(px(60.))
                            .flex_shrink_0()
                            .px_5()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(18.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.text_primary)
                                    .child("Settings"),
                            )
                            .child(
                                icon_button("close-settings", "cancel-01", false, true).on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.show_settings_modal = false;
                                        cx.notify();
                                    }),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .id("settings-scroll")
                            .overflow_y_scroll()
                            .px_5()
                            .pb_5()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.text_muted)
                                    .mt_4()
                                    .mb_2()
                                    .child("Appearance"),
                            )
                            .child(
                                div()
                                    .rounded(px(10.))
                                    .bg(theme.card)
                                    .border_1()
                                    .border_color(theme.surface_border)
                                    .overflow_hidden()
                                    .children(vec![
                                        div()
                                            .min_h(px(64.))
                                            .px_4()
                                            .py_3()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .gap_4()
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .flex()
                                                    .flex_col()
                                                    .gap_1()
                                                    .child(
                                                        div()
                                                            .text_size(px(13.))
                                                            .font_weight(gpui::FontWeight::MEDIUM)
                                                            .text_color(theme.text_primary)
                                                            .child("Window material"),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(theme.text_muted)
                                                            .child("Choose the window background"),
                                                    ),
                                            )
                                            .child(
                                                macos_segmented_track(
                                                    "settings-bg-frosted-track",
                                                    theme,
                                                )
                                                .w(px(180.))
                                                .h(px(30.))
                                                .rounded(px(7.))
                                                .flex_shrink_0()
                                                .child(
                                                    macos_segmented_item(
                                                        "bg-frosted",
                                                        self.background_mode != "solid",
                                                        theme,
                                                    )
                                                    .rounded(px(5.))
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.background_mode =
                                                            "frosted".to_string();
                                                        if let Some(coord) =
                                                            &mut this.blur_coordinator
                                                        {
                                                            coord.set_enabled(true);
                                                        }
                                                        this.save_config();
                                                        cx.notify();
                                                    }))
                                                    .child("Frosted"),
                                                )
                                                .child(
                                                    macos_segmented_item(
                                                        "bg-solid",
                                                        self.background_mode == "solid",
                                                        theme,
                                                    )
                                                    .rounded(px(5.))
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.background_mode = "solid".to_string();
                                                        if let Some(coord) =
                                                            &mut this.blur_coordinator
                                                        {
                                                            coord.set_enabled(false);
                                                        }
                                                        this.save_config();
                                                        cx.notify();
                                                    }))
                                                    .child("Solid"),
                                                ),
                                            )
                                            .into_any_element(),
                                        div()
                                            .min_h(px(64.))
                                            .px_4()
                                            .py_3()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .gap_4()
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .flex()
                                                    .flex_col()
                                                    .gap_1()
                                                    .child(
                                                        div()
                                                            .text_size(px(13.))
                                                            .font_weight(gpui::FontWeight::MEDIUM)
                                                            .text_color(theme.text_primary)
                                                            .child("Icon size"),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(theme.text_muted)
                                                            .child("Size of files in grid view"),
                                                    ),
                                            )
                                            .child(
                                                macos_segmented_track(
                                                    "settings-zoom-sm-track",
                                                    theme,
                                                )
                                                .w(px(180.))
                                                .h(px(30.))
                                                .rounded(px(7.))
                                                .flex_shrink_0()
                                                .child(
                                                    macos_segmented_item(
                                                        "zoom-sm",
                                                        self.zoom <= 95.,
                                                        theme,
                                                    )
                                                    .rounded(px(5.))
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.zoom = 80.;
                                                        cx.notify();
                                                    }))
                                                    .child("S"),
                                                )
                                                .child(
                                                    macos_segmented_item(
                                                        "zoom-med",
                                                        self.zoom > 95. && self.zoom <= 140.,
                                                        theme,
                                                    )
                                                    .rounded(px(5.))
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.zoom = 120.;
                                                        cx.notify();
                                                    }))
                                                    .child("M"),
                                                )
                                                .child(
                                                    macos_segmented_item(
                                                        "zoom-lg",
                                                        self.zoom > 140. && self.zoom <= 200.,
                                                        theme,
                                                    )
                                                    .rounded(px(5.))
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.zoom = 180.;
                                                        cx.notify();
                                                    }))
                                                    .child("L"),
                                                )
                                                .child(
                                                    macos_segmented_item(
                                                        "zoom-xl",
                                                        self.zoom > 200.,
                                                        theme,
                                                    )
                                                    .rounded(px(5.))
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.zoom = 250.;
                                                        cx.notify();
                                                    }))
                                                    .child("XL"),
                                                ),
                                            )
                                            .into_any_element(),
                                        div()
                                            .min_h(px(64.))
                                            .px_4()
                                            .py_3()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .gap_4()
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .flex()
                                                    .flex_col()
                                                    .gap_1()
                                                    .child(
                                                        div()
                                                            .text_size(px(13.))
                                                            .font_weight(gpui::FontWeight::MEDIUM)
                                                            .text_color(theme.text_primary)
                                                            .child("Text size"),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(theme.text_muted)
                                                            .child("Interface font size"),
                                                    ),
                                            )
                                            .child(
                                                macos_segmented_track(
                                                    "settings-font-sm-track",
                                                    theme,
                                                )
                                                .w(px(180.))
                                                .h(px(30.))
                                                .rounded(px(7.))
                                                .flex_shrink_0()
                                                .child(
                                                    macos_segmented_item(
                                                        "font-sm",
                                                        (self.font_size - 12.).abs() < 0.5,
                                                        theme,
                                                    )
                                                    .rounded(px(5.))
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.font_size = 12.;
                                                        this.save_config();
                                                        cx.notify();
                                                    }))
                                                    .child("12"),
                                                )
                                                .child(
                                                    macos_segmented_item(
                                                        "font-def",
                                                        (self.font_size - 14.).abs() < 0.5,
                                                        theme,
                                                    )
                                                    .rounded(px(5.))
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.font_size = 14.;
                                                        this.save_config();
                                                        cx.notify();
                                                    }))
                                                    .child("14"),
                                                )
                                                .child(
                                                    macos_segmented_item(
                                                        "font-med",
                                                        (self.font_size - 15.).abs() < 0.5,
                                                        theme,
                                                    )
                                                    .rounded(px(5.))
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.font_size = 15.;
                                                        this.save_config();
                                                        cx.notify();
                                                    }))
                                                    .child("15"),
                                                )
                                                .child(
                                                    macos_segmented_item(
                                                        "font-lg",
                                                        (self.font_size - 16.).abs() < 0.5,
                                                        theme,
                                                    )
                                                    .rounded(px(5.))
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.font_size = 16.;
                                                        this.save_config();
                                                        cx.notify();
                                                    }))
                                                    .child("16"),
                                                ),
                                            )
                                            .into_any_element(),
                                    ]),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.text_muted)
                                    .mt_4()
                                    .mb_2()
                                    .child("Browsing"),
                            )
                            .child(
                                div()
                                    .rounded(px(10.))
                                    .bg(theme.card)
                                    .border_1()
                                    .border_color(theme.surface_border)
                                    .overflow_hidden()
                                    .children(vec![
                                        div()
                                            .min_h(px(64.))
                                            .px_4()
                                            .py_3()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .gap_4()
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .flex()
                                                    .flex_col()
                                                    .gap_1()
                                                    .child(
                                                        div()
                                                            .text_size(px(13.))
                                                            .font_weight(gpui::FontWeight::MEDIUM)
                                                            .text_color(theme.text_primary)
                                                            .child("Hidden files"),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(theme.text_muted)
                                                            .child(
                                                                "Show files beginning with a dot",
                                                            ),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .id("toggle-hidden-btn")
                                                    .aria_label("Show hidden files")
                                                    .role(Role::Button)
                                                    .cursor_pointer()
                                                    .w(px(38.))
                                                    .h(px(24.))
                                                    .flex_shrink_0()
                                                    .rounded_full()
                                                    .p(px(3.))
                                                    .bg(if self.show_hidden {
                                                        theme.accent
                                                    } else {
                                                        theme.text_faint.opacity(0.35)
                                                    })
                                                    .flex()
                                                    .items_center()
                                                    .when(self.show_hidden, |d| d.justify_end())
                                                    .hover(|d| d.opacity(0.85))
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.show_hidden = !this.show_hidden;
                                                        this.sort_and_filter();
                                                        this.save_config();
                                                        cx.notify();
                                                    }))
                                                    .child(
                                                        div()
                                                            .size(px(18.))
                                                            .rounded_full()
                                                            .bg(gpui::white()),
                                                    ),
                                            )
                                            .into_any_element(),
                                        div()
                                            .min_h(px(64.))
                                            .px_4()
                                            .py_3()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .gap_4()
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .flex()
                                                    .flex_col()
                                                    .gap_1()
                                                    .child(
                                                        div()
                                                            .text_size(px(13.))
                                                            .font_weight(gpui::FontWeight::MEDIUM)
                                                            .text_color(theme.text_primary)
                                                            .child("Folders first"),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(theme.text_muted)
                                                            .child("Group folders above files"),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .id("toggle-folders-first-btn")
                                                    .aria_label("Sort folders first")
                                                    .role(Role::Button)
                                                    .cursor_pointer()
                                                    .w(px(38.))
                                                    .h(px(24.))
                                                    .flex_shrink_0()
                                                    .rounded_full()
                                                    .p(px(3.))
                                                    .bg(if self.sort_folders_first {
                                                        theme.accent
                                                    } else {
                                                        theme.text_faint.opacity(0.35)
                                                    })
                                                    .flex()
                                                    .items_center()
                                                    .when(self.sort_folders_first, |d| {
                                                        d.justify_end()
                                                    })
                                                    .hover(|d| d.opacity(0.85))
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.sort_folders_first =
                                                            !this.sort_folders_first;
                                                        this.sort_and_filter();
                                                        this.save_config();
                                                        cx.notify();
                                                    }))
                                                    .child(
                                                        div()
                                                            .size(px(18.))
                                                            .rounded_full()
                                                            .bg(gpui::white()),
                                                    ),
                                            )
                                            .into_any_element(),
                                        div()
                                            .min_h(px(64.))
                                            .px_4()
                                            .py_3()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .gap_4()
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .flex()
                                                    .flex_col()
                                                    .gap_1()
                                                    .child(
                                                        div()
                                                            .text_size(px(13.))
                                                            .font_weight(gpui::FontWeight::MEDIUM)
                                                            .text_color(theme.text_primary)
                                                            .child("Thumbnail previews"),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(theme.text_muted)
                                                            .child("Preview images and videos"),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .id("toggle-thumbs-btn")
                                                    .role(Role::Button)
                                                    .cursor_pointer()
                                                    .w(px(38.))
                                                    .h(px(24.))
                                                    .flex_shrink_0()
                                                    .rounded_full()
                                                    .p(px(3.))
                                                    .bg(if self.enable_thumbnails {
                                                        theme.accent
                                                    } else {
                                                        theme.text_faint.opacity(0.35)
                                                    })
                                                    .flex()
                                                    .items_center()
                                                    .when(self.enable_thumbnails, |d| {
                                                        d.justify_end()
                                                    })
                                                    .hover(|d| d.opacity(0.85))
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.enable_thumbnails =
                                                            !this.enable_thumbnails;
                                                        cx.notify();
                                                    }))
                                                    .child(
                                                        div()
                                                            .size(px(18.))
                                                            .rounded_full()
                                                            .bg(gpui::white()),
                                                    ),
                                            )
                                            .into_any_element(),
                                    ]),
                            )
                            .child(
                                div().mt_4().child(
                                    div()
                                        .min_h(px(64.))
                                        .px_4()
                                        .py_3()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .gap_4()
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .flex()
                                                .flex_col()
                                                .gap_1()
                                                .child(
                                                    div()
                                                        .text_size(px(13.))
                                                        .font_weight(gpui::FontWeight::MEDIUM)
                                                        .text_color(theme.text_primary)
                                                        .child("Terminal"),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.))
                                                        .text_color(theme.text_muted)
                                                        .child(
                                                            "Open a shell in the current folder",
                                                        ),
                                                ),
                                        )
                                        .child(
                                            theme_button(
                                                "settings-open-term",
                                                ButtonLook::Secondary,
                                                theme,
                                            )
                                            .h(px(28.))
                                            .rounded(px(6.))
                                            .on_click(cx.listener(|this, _, _, _| {
                                                open_in_terminal(&this.current_path);
                                            }))
                                            .child("Open Terminal"),
                                        ),
                                ),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn render_filter_popover(&self, theme: &theme::ThemeColors, cx: &mut Context<Self>) -> AnyElement {
        let is_any_filter_active = self.has_active_filters();
        let total_count = self.entries.len();
        let filtered_count = self.filtered_indices.len();

        let popover_card_bg = theme.card;
        let popover_border = theme.surface_border;

        div()
            .id("filter-popover-backdrop")
            .absolute()
            .inset_0()
            .bg(gpui::rgba(0x00000000)) // Pure transparent: no subtle black layer over the app
            .flex()
            .items_start()
            .justify_end()
            .pt(px(48.))
            .pr(px(210.)) // Slid left to align under the action capsule & view switcher
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(|this, _, _, cx| {
                this.show_filter_menu = false;
                cx.notify();
            }))
            .child(
                div()
                    .id("filter-popover-card")
                    .relative()
                    .w(px(330.))
                    .bg(popover_card_bg)
                    .border_1()
                    .border_color(popover_border)
                    .shadow(theme::popover_shadows(theme))
                    .rounded(px(14.))
                    .overflow_hidden()
                    .p_3p5()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(theme::glass_specular_sheen(theme))
                    // Header
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(icon("filter", 15.).text_color(theme.accent))
                                    .child(
                                        div()
                                            .text_size(px(13.5))
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(theme.text_primary)
                                            .child("Filter & Sort"),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .when(is_any_filter_active, |d| {
                                        d.child(
                                            theme_button("popover-reset-filters", ButtonLook::Ghost, theme)
                                                .h(px(24.))
                                                .px(px(8.))
                                                .text_size(px(11.))
                                                .on_click(cx.listener(|this, _, _, cx| this.reset_filters(cx)))
                                                .child("Reset"),
                                        )
                                    })
                                    .child(
                                        icon_button("close-filter-popover", "cancel-01", false, true)
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.show_filter_menu = false;
                                                cx.notify();
                                            })),
                                    ),
                            ),
                    )
                    // Section 1: Sort Column
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1p5()
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.text_faint)
                                    .child("SORT BY"),
                            )
                            .child(
                                macos_segmented_track("sort-by-track", theme)
                                    .child(
                                        macos_segmented_item("sort-name", self.sort_by == SortBy::Name, theme)
                                            .on_click(cx.listener(|this, _, _, cx| this.toggle_sort(SortBy::Name, cx)))
                                            .child("Name"),
                                    )
                                    .child(
                                        macos_segmented_item("sort-mod", self.sort_by == SortBy::Modified, theme)
                                            .on_click(cx.listener(|this, _, _, cx| this.toggle_sort(SortBy::Modified, cx)))
                                            .child("Date"),
                                    )
                                    .child(
                                        macos_segmented_item("sort-size", self.sort_by == SortBy::Size, theme)
                                            .on_click(cx.listener(|this, _, _, cx| this.toggle_sort(SortBy::Size, cx)))
                                            .child("Size"),
                                    )
                                    .child(
                                        macos_segmented_item("sort-type", self.sort_by == SortBy::Type, theme)
                                            .on_click(cx.listener(|this, _, _, cx| this.toggle_sort(SortBy::Type, cx)))
                                            .child("Type"),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .pt_1()
                                    .child(
                                        div()
                                            .text_size(px(11.5))
                                            .text_color(theme.text_muted)
                                            .child("Direction:"),
                                    )
                                    .child(
                                        theme_chip("sort-direction", self.sort_reversed, theme)
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.sort_reversed = !this.sort_reversed;
                                                this.sort_and_filter();
                                                cx.notify();
                                            }))
                                            .child(if self.sort_reversed { "Descending ↓" } else { "Ascending ↑" }),
                                    ),
                            ),
                    )
                    // Divider
                    .child(div().h(px(1.)).w_full().bg(theme.surface_border))
                    // Section 2: Filter by Type
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1p5()
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.text_faint)
                                    .child("FILTER BY TYPE"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .gap_1p5()
                                    .child(
                                        theme_chip("f-type-all", self.type_filter == TypeFilter::All, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.type_filter = TypeFilter::All; this.sort_and_filter(); cx.notify(); }))
                                            .child("All"),
                                    )
                                    .child(
                                        theme_chip("f-type-folders", self.type_filter == TypeFilter::Folders, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.type_filter = TypeFilter::Folders; this.sort_and_filter(); cx.notify(); }))
                                            .child("Folders"),
                                    )
                                    .child(
                                        theme_chip("f-type-docs", self.type_filter == TypeFilter::Documents, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.type_filter = TypeFilter::Documents; this.sort_and_filter(); cx.notify(); }))
                                            .child("Docs"),
                                    )
                                    .child(
                                        theme_chip("f-type-images", self.type_filter == TypeFilter::Images, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.type_filter = TypeFilter::Images; this.sort_and_filter(); cx.notify(); }))
                                            .child("Images"),
                                    )
                                    .child(
                                        theme_chip("f-type-media", self.type_filter == TypeFilter::Media, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.type_filter = TypeFilter::Media; this.sort_and_filter(); cx.notify(); }))
                                            .child("Media"),
                                    )
                                    .child(
                                        theme_chip("f-type-code", self.type_filter == TypeFilter::Code, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.type_filter = TypeFilter::Code; this.sort_and_filter(); cx.notify(); }))
                                            .child("Code"),
                                    )
                                    .child(
                                        theme_chip("f-type-archives", self.type_filter == TypeFilter::Archives, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.type_filter = TypeFilter::Archives; this.sort_and_filter(); cx.notify(); }))
                                            .child("Archives"),
                                    ),
                            ),
                    )
                    // Divider
                    .child(div().h(px(1.)).w_full().bg(theme.surface_border))
                    // Section 3: Filter by Date Modified
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1p5()
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.text_faint)
                                    .child("DATE MODIFIED"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .gap_1p5()
                                    .child(
                                        theme_chip("f-date-any", self.date_filter == DateFilter::Any, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.date_filter = DateFilter::Any; this.sort_and_filter(); cx.notify(); }))
                                            .child("Any time"),
                                    )
                                    .child(
                                        theme_chip("f-date-today", self.date_filter == DateFilter::Today, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.date_filter = DateFilter::Today; this.sort_and_filter(); cx.notify(); }))
                                            .child("Today"),
                                    )
                                    .child(
                                        theme_chip("f-date-7d", self.date_filter == DateFilter::Last7Days, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.date_filter = DateFilter::Last7Days; this.sort_and_filter(); cx.notify(); }))
                                            .child("Past 7 Days"),
                                    )
                                    .child(
                                        theme_chip("f-date-30d", self.date_filter == DateFilter::Last30Days, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.date_filter = DateFilter::Last30Days; this.sort_and_filter(); cx.notify(); }))
                                            .child("Past 30 Days"),
                                    ),
                            ),
                    )
                    // Divider
                    .child(div().h(px(1.)).w_full().bg(theme.surface_border))
                    // Section 4: Filter by Size
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1p5()
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.text_faint)
                                    .child("FILE SIZE"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_1p5()
                                    .child(
                                        theme_chip("f-size-any", self.size_filter == SizeFilter::Any, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.size_filter = SizeFilter::Any; this.sort_and_filter(); cx.notify(); }))
                                            .child("Any"),
                                    )
                                    .child(
                                        theme_chip("f-size-small", self.size_filter == SizeFilter::Small, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.size_filter = SizeFilter::Small; this.sort_and_filter(); cx.notify(); }))
                                            .child("< 1 MB"),
                                    )
                                    .child(
                                        theme_chip("f-size-med", self.size_filter == SizeFilter::Medium, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.size_filter = SizeFilter::Medium; this.sort_and_filter(); cx.notify(); }))
                                            .child("1-100 MB"),
                                    )
                                    .child(
                                        theme_chip("f-size-large", self.size_filter == SizeFilter::Large, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.size_filter = SizeFilter::Large; this.sort_and_filter(); cx.notify(); }))
                                            .child("> 100 MB"),
                                    ),
                            ),
                    )
                    // Footer status
                    .child(
                        div()
                            .pt_2()
                            .border_t_1()
                            .border_color(theme.surface_border)
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(11.))
                            .text_color(theme.text_faint)
                            .child(format!("Showing {} of {} items", filtered_count, total_count))
                            .child(
                                theme_button("done-filter-popover", ButtonLook::Primary, theme)
                                    .h(px(28.))
                                    .px(px(14.))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.show_filter_menu = false;
                                        cx.notify();
                                    }))
                                    .child("Done"),
                            ),
                    ),
            )
            .into_any_element()
    }

    pub fn set_theme_mode(&mut self, mode: theme::ThemeMode, cx: &mut Context<Self>) {
        self.theme_mode = mode;
        let mut config = crate::config::Config::load();
        config.theme = match mode {
            theme::ThemeMode::Light => "light",
            theme::ThemeMode::Dark => "dark",
            theme::ThemeMode::System => "system",
        }.to_string();
        config.save();
        cx.notify();
    }

    fn render_more_menu(&self, theme: &theme::ThemeColors, cx: &mut Context<Self>) -> AnyElement {
        let popover_card_bg = theme.card;
        let popover_border = theme.surface_border;

        let is_grid = self.view_mode == ViewMode::Grid;
        let view_mode_text = if is_grid { "Switch to List View" } else { "Switch to Grid View" };
        let view_mode_icon = if is_grid { "list" } else { "grid" };

        let hidden_text = if self.show_hidden { "Hide Hidden Files" } else { "Show Hidden Files" };
        let inspector_text = if self.show_inspector { "Hide Inspector" } else { "Show Inspector" };

        div()
            .id("more-menu-backdrop")
            .absolute()
            .inset_0()
            .bg(gpui::rgba(0x00000000))
            .flex()
            .items_start()
            .justify_end()
            .pt(px(48.))
            .pr(px(180.))
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(|this, _, _, cx| {
                this.show_more_menu = false;
                cx.notify();
            }))
            .child(
                div()
                    .id("more-menu-card")
                    .relative()
                    .w(px(250.))
                    .bg(popover_card_bg)
                    .border_1()
                    .border_color(popover_border)
                    .shadow(theme::popover_shadows(theme))
                    .rounded(px(14.))
                    .overflow_hidden()
                    .p_2p5()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                    .on_click(|_, _, cx| cx.stop_propagation())
                    // 1. Appearance / Theme Section
                    .child(
                        div()
                            .px_2()
                            .pt_1()
                            .pb_1()
                            .text_size(px(10.5))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.text_faint)
                            .child("APPEARANCE"),
                    )
                    .child(
                        div()
                            .h(px(28.))
                            .p_0p5()
                            .rounded(px(7.))
                            .bg(if theme.is_dark { gpui::rgba(0x00000030) } else { gpui::rgba(0x0000000d) })
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .id("theme-btn-system")
                                    .flex_1()
                                    .h_full()
                                    .rounded(px(5.))
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .gap_1()
                                    .text_size(px(11.5))
                                    .when(self.theme_mode == theme::ThemeMode::System, |d| {
                                        d.bg(theme.card)
                                            .shadow(theme::control_shadow(theme))
                                            .text_color(theme.text_primary)
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                    })
                                    .when(self.theme_mode != theme::ThemeMode::System, |d| {
                                        d.text_color(theme.text_muted)
                                            .hover(|s| s.bg(theme.button_hover).text_color(theme.text_primary))
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.set_theme_mode(theme::ThemeMode::System, cx);
                                    }))
                                    .child(icon("sliders", 12.))
                                    .child("Auto"),
                            )
                            .child(
                                div()
                                    .id("theme-btn-light")
                                    .flex_1()
                                    .h_full()
                                    .rounded(px(5.))
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .gap_1()
                                    .text_size(px(11.5))
                                    .when(self.theme_mode == theme::ThemeMode::Light, |d| {
                                        d.bg(theme.card)
                                            .shadow(theme::control_shadow(theme))
                                            .text_color(theme.text_primary)
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                    })
                                    .when(self.theme_mode != theme::ThemeMode::Light, |d| {
                                        d.text_color(theme.text_muted)
                                            .hover(|s| s.bg(theme.button_hover).text_color(theme.text_primary))
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.set_theme_mode(theme::ThemeMode::Light, cx);
                                    }))
                                    .child(icon("palette", 12.))
                                    .child("Light"),
                            )
                            .child(
                                div()
                                    .id("theme-btn-dark")
                                    .flex_1()
                                    .h_full()
                                    .rounded(px(5.))
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .gap_1()
                                    .text_size(px(11.5))
                                    .when(self.theme_mode == theme::ThemeMode::Dark, |d| {
                                        d.bg(theme.card)
                                            .shadow(theme::control_shadow(theme))
                                            .text_color(theme.text_primary)
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                    })
                                    .when(self.theme_mode != theme::ThemeMode::Dark, |d| {
                                        d.text_color(theme.text_muted)
                                            .hover(|s| s.bg(theme.button_hover).text_color(theme.text_primary))
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.set_theme_mode(theme::ThemeMode::Dark, cx);
                                    }))
                                    .child(icon("palette", 12.))
                                    .child("Dark"),
                            ),
                    )
                    // Divider
                    .child(div().h(px(1.)).mx_1().my_1().bg(theme.surface_border))
                    // 2. View Options
                    .child(
                        div()
                            .id("more-opt-view-mode")
                            .h(px(30.))
                            .px_2()
                            .rounded(px(6.))
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .justify_between()
                            .hover(|s| s.bg(theme.sidebar_hover))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.show_more_menu = false;
                                let next = if this.view_mode == ViewMode::Grid { ViewMode::List } else { ViewMode::Grid };
                                this.set_view_mode(next, cx);
                            }))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2p5()
                                    .child(icon(view_mode_icon, 14.).text_color(theme.text_muted))
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .text_color(theme.text_primary)
                                            .child(view_mode_text),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme.text_faint)
                                    .child("Ctrl+V"),
                            ),
                    )
                    .child(
                        div()
                            .id("more-opt-hidden")
                            .h(px(30.))
                            .px_2()
                            .rounded(px(6.))
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .justify_between()
                            .hover(|s| s.bg(theme.sidebar_hover))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.show_more_menu = false;
                                this.show_hidden = !this.show_hidden;
                                this.sort_and_filter();
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2p5()
                                    .child(icon("eye", 14.).text_color(theme.text_muted))
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .text_color(theme.text_primary)
                                            .child(hidden_text),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme.text_faint)
                                    .child("Ctrl+H"),
                            ),
                    )
                    .child(
                        div()
                            .id("more-opt-inspector")
                            .h(px(30.))
                            .px_2()
                            .rounded(px(6.))
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .justify_between()
                            .hover(|s| s.bg(theme.sidebar_hover))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.show_more_menu = false;
                                this.toggle_inspector(cx);
                            }))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2p5()
                                    .child(icon("sliders", 14.).text_color(theme.text_muted))
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .text_color(theme.text_primary)
                                            .child(inspector_text),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme.text_faint)
                                    .child("Ctrl+I"),
                            ),
                    )
                    // Divider
                    .child(div().h(px(1.)).mx_1().my_1().bg(theme.surface_border))
                    // 3. Actions
                    .child(
                        div()
                            .id("more-opt-new-folder")
                            .h(px(30.))
                            .px_2()
                            .rounded(px(6.))
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .justify_between()
                            .hover(|s| s.bg(theme.sidebar_hover))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.show_more_menu = false;
                                this.create_new_folder(cx);
                            }))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2p5()
                                    .child(icon("add-01", 14.).text_color(theme.text_muted))
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .text_color(theme.text_primary)
                                            .child("New Folder"),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme.text_faint)
                                    .child("Ctrl+N"),
                            ),
                    )
                    .child(
                        div()
                            .id("more-opt-terminal")
                            .h(px(30.))
                            .px_2()
                            .rounded(px(6.))
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .justify_between()
                            .hover(|s| s.bg(theme.sidebar_hover))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.show_more_menu = false;
                                open_in_terminal(&this.current_path);
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2p5()
                                    .child(icon("terminal", 14.).text_color(theme.text_muted))
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .text_color(theme.text_primary)
                                            .child("Open in Terminal"),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme.text_faint)
                                    .child("Ctrl+T"),
                            ),
                    )
                    .child(
                        div()
                            .id("more-opt-refresh")
                            .h(px(30.))
                            .px_2()
                            .rounded(px(6.))
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .justify_between()
                            .hover(|s| s.bg(theme.sidebar_hover))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.show_more_menu = false;
                                this.refresh(cx);
                            }))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2p5()
                                    .child(icon("rotate-cw", 14.).text_color(theme.text_muted))
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .text_color(theme.text_primary)
                                            .child("Refresh Directory"),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme.text_faint)
                                    .child("Ctrl+R"),
                            ),
                    )
                    // Divider
                    .child(div().h(px(1.)).mx_1().my_1().bg(theme.surface_border))
                    // 4. Settings
                    .child(
                        div()
                            .id("more-opt-settings")
                            .h(px(30.))
                            .px_2()
                            .rounded(px(6.))
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .justify_between()
                            .hover(|s| s.bg(theme.sidebar_hover))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.show_more_menu = false;
                                this.show_settings_modal = true;
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2p5()
                                    .child(icon("settings", 14.).text_color(theme.text_muted))
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .text_color(theme.text_primary)
                                            .child("Settings & Preferences..."),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme.text_faint)
                                    .child("Ctrl+,"),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn render_active_filters_bar(&self, theme: &theme::ThemeColors, cx: &mut Context<Self>) -> Div {
        div()
            .h(px(28.))
            .px_3()
            .border_b_1()
            .border_color(theme.surface_border)
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.text_faint)
                            .child("Active Filters:"),
                    )
                    .when(self.type_filter != TypeFilter::All, |d| {
                        d.child(
                            div()
                                .id("active-filter-type-chip")
                                .px_2()
                                .py_0p5()
                                .rounded(px(4.))
                                .bg(theme.accent.opacity(0.15))
                                .border_1()
                                .border_color(theme.accent.opacity(0.3))
                                .flex()
                                .items_center()
                                .gap_1()
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.type_filter = TypeFilter::All;
                                    this.sort_and_filter();
                                    cx.notify();
                                }))
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(theme.accent)
                                        .child(format!("{:?} ✕", self.type_filter)),
                                ),
                        )
                    })
                    .when(self.date_filter != DateFilter::Any, |d| {
                        d.child(
                            div()
                                .id("active-filter-date-chip")
                                .px_2()
                                .py_0p5()
                                .rounded(px(4.))
                                .bg(theme.accent.opacity(0.15))
                                .border_1()
                                .border_color(theme.accent.opacity(0.3))
                                .flex()
                                .items_center()
                                .gap_1()
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.date_filter = DateFilter::Any;
                                    this.sort_and_filter();
                                    cx.notify();
                                }))
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(theme.accent)
                                        .child(format!("{:?} ✕", self.date_filter)),
                                ),
                        )
                    })
                    .when(self.size_filter != SizeFilter::Any, |d| {
                        d.child(
                            div()
                                .id("active-filter-size-chip")
                                .px_2()
                                .py_0p5()
                                .rounded(px(4.))
                                .bg(theme.accent.opacity(0.15))
                                .border_1()
                                .border_color(theme.accent.opacity(0.3))
                                .flex()
                                .items_center()
                                .gap_1()
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.size_filter = SizeFilter::Any;
                                    this.sort_and_filter();
                                    cx.notify();
                                }))
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(theme.accent)
                                        .child(format!("{:?} ✕", self.size_filter)),
                                ),
                        )
                    }),
            )
            .child(
                button("clear-all-filters-chip", ButtonLook::Ghost)
                    .on_click(cx.listener(|this, _, _, cx| this.reset_filters(cx)))
                    .child("Clear All"),
            )
    }

    fn get_palette_items(&self) -> Vec<PaletteItem> {
        let q = self.command_palette_query.trim().to_lowercase();
        let mut items = Vec::new();

        let mut nav = Vec::new();
        if let Some(p) = dirs::home_dir() {
            let path_display = p.to_string_lossy().to_string();
            nav.push(PaletteItem {
                section: "Navigation",
                title: "Home".to_string(),
                subtitle: Some(path_display),
                icon_name: "home",
                shortcut: Some("H"),
                action: PaletteAction::Navigate(p),
            });
        }
        if let Some(p) = dirs::document_dir() {
            let path_display = p.to_string_lossy().to_string();
            nav.push(PaletteItem {
                section: "Navigation",
                title: "Work".to_string(),
                subtitle: Some(path_display),
                icon_name: "user",
                shortcut: Some("W"),
                action: PaletteAction::Navigate(p),
            });
        }
        let doc_dir = dirs::document_dir().or_else(|| dirs::home_dir().map(|h| h.join("Documents")));
        if let Some(p) = &doc_dir {
            let path_display = p.to_string_lossy().to_string();
            nav.push(PaletteItem {
                section: "Navigation",
                title: "Projects".to_string(),
                subtitle: Some(path_display),
                icon_name: "layers",
                shortcut: Some("P"),
                action: PaletteAction::Navigate(p.clone()),
            });
        }
        if let Some(p) = dirs::download_dir() {
            let path_display = p.to_string_lossy().to_string();
            nav.push(PaletteItem {
                section: "Navigation",
                title: "Downloads".to_string(),
                subtitle: Some(path_display),
                icon_name: "download",
                shortcut: Some("D"),
                action: PaletteAction::Navigate(p),
            });
        }
        if let Some(p) = dirs::home_dir() {
            let path_display = p.to_string_lossy().to_string();
            nav.push(PaletteItem {
                section: "Navigation",
                title: "Bookmarks".to_string(),
                subtitle: Some(path_display),
                icon_name: "bookmark",
                shortcut: Some("M"),
                action: PaletteAction::Navigate(p),
            });
        }
        if let Some(p) = dirs::desktop_dir() {
            let path_display = p.to_string_lossy().to_string();
            nav.push(PaletteItem {
                section: "Navigation",
                title: "Desktop".to_string(),
                subtitle: Some(path_display),
                icon_name: "app-window",
                shortcut: Some("K"),
                action: PaletteAction::Navigate(p),
            });
        }
        if let Some(p) = dirs::picture_dir() {
            let path_display = p.to_string_lossy().to_string();
            nav.push(PaletteItem {
                section: "Navigation",
                title: "Pictures".to_string(),
                subtitle: Some(path_display),
                icon_name: "image",
                shortcut: Some("I"),
                action: PaletteAction::Navigate(p),
            });
        }
        if let Some(p) = dirs::video_dir() {
            let path_display = p.to_string_lossy().to_string();
            nav.push(PaletteItem {
                section: "Navigation",
                title: "Videos".to_string(),
                subtitle: Some(path_display),
                icon_name: "video",
                shortcut: Some("V"),
                action: PaletteAction::Navigate(p),
            });
        }
        if let Some(p) = dirs::audio_dir() {
            let path_display = p.to_string_lossy().to_string();
            nav.push(PaletteItem {
                section: "Navigation",
                title: "Music".to_string(),
                subtitle: Some(path_display),
                icon_name: "audio",
                shortcut: Some("S"),
                action: PaletteAction::Navigate(p),
            });
        }

        // Projects Section (discover local projects in Documents)
        let mut projects = Vec::new();
        if let Some(doc_path) = doc_dir {
            let candidate_names = ["Movie Finder", "Nova Gaming", "shark", "monodrive", "portfolio", "Recordly", "tolaria", "socialmess", "Bookmrk"];
            for name in candidate_names {
                let candidate_path = doc_path.join(name);
                if candidate_path.is_dir() {
                    let sub = candidate_path.to_string_lossy().to_string();
                    projects.push(PaletteItem {
                        section: "Projects",
                        title: name.to_string(),
                        subtitle: Some(sub),
                        icon_name: "rocket",
                        shortcut: None,
                        action: PaletteAction::Navigate(candidate_path),
                    });
                }
            }
            // If empty, fall back to any subdirectories in Documents
            if projects.is_empty() {
                if let Ok(read_dir) = std::fs::read_dir(&doc_path) {
                    for entry in read_dir.flatten().take(6) {
                        let path = entry.path();
                        if path.is_dir() {
                            let title = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                            if !title.starts_with('.') {
                                let sub = path.to_string_lossy().to_string();
                                projects.push(PaletteItem {
                                    section: "Projects",
                                    title,
                                    subtitle: Some(sub),
                                    icon_name: "rocket",
                                    shortcut: None,
                                    action: PaletteAction::Navigate(path),
                                });
                            }
                        }
                    }
                }
            }
        }

        for item in nav {
            if q.is_empty() || item.title.to_lowercase().contains(&q) || item.subtitle.as_deref().unwrap_or("").to_lowercase().contains(&q) {
                items.push(item);
            }
        }
        for item in projects {
            if q.is_empty() || item.title.to_lowercase().contains(&q) || item.subtitle.as_deref().unwrap_or("").to_lowercase().contains(&q) {
                items.push(item);
            }
        }

        if !q.is_empty() {
            let mut folder_matches = 0;
            let mut file_matches = 0;

            for entry in &self.entries {
                if entry.name_lower.contains(&q) {
                    if entry.is_directory {
                        if folder_matches < 15 {
                            folder_matches += 1;
                            items.push(PaletteItem {
                                section: "Folders",
                                title: entry.name.clone(),
                                subtitle: Some(entry.path.to_string_lossy().to_string()),
                                icon_name: "folder",
                                shortcut: None,
                                action: PaletteAction::Navigate(entry.path.clone()),
                            });
                        }
                    } else {
                        if file_matches < 25 {
                            file_matches += 1;
                            items.push(PaletteItem {
                                section: "Files",
                                title: entry.name.clone(),
                                subtitle: Some(format!("{} • {}", entry.type_description(), entry.formatted_size())),
                                icon_name: entry.icon_name,
                                shortcut: None,
                                action: PaletteAction::OpenFile(entry.path.clone()),
                            });
                        }
                    }
                }
            }
        }

        items
    }

    pub fn open_command_palette(&mut self, initial_query: Option<&str>, cx: &mut Context<Self>) {
        self.show_command_palette = true;
        let q = initial_query.unwrap_or("").to_string();
        self.command_palette_query = q;
        self.command_palette_selected = 0;
        self.command_palette_has_query = !self.command_palette_query.is_empty();
        self.command_palette_search_epoch = 0;
        cx.notify();
    }

    pub fn close_command_palette(&mut self, cx: &mut Context<Self>) {
        self.show_command_palette = false;
        self.command_palette_query.clear();
        self.command_palette_has_query = false;
        self.command_palette_search_epoch = 0;
        cx.notify();
    }

    pub fn update_command_palette_query(&mut self, new_query: String, cx: &mut Context<Self>) {
        let had_query = self.command_palette_has_query;
        self.command_palette_query = new_query;
        self.command_palette_selected = 0;
        let has_query = !self.command_palette_query.is_empty();
        if has_query != had_query {
            self.command_palette_has_query = has_query;
            self.command_palette_search_epoch = self.command_palette_search_epoch.wrapping_add(1);
        }
        cx.notify();
    }

    pub fn execute_palette_action(&mut self, action: PaletteAction, cx: &mut Context<Self>) {
        self.close_command_palette(cx);
        match action {
            PaletteAction::Navigate(path) => {
                self.navigate_to(path, cx);
            }
            PaletteAction::NewFolder => {
                self.create_new_folder(cx);
            }
            PaletteAction::ToggleViewMode => {
                let next = if self.view_mode == ViewMode::Grid { ViewMode::List } else { ViewMode::Grid };
                self.set_view_mode(next, cx);
            }
            PaletteAction::ToggleHidden => {
                self.show_hidden = !self.show_hidden;
                self.sort_and_filter();
                cx.notify();
            }
            PaletteAction::ToggleInspector => {
                self.toggle_inspector(cx);
            }
            PaletteAction::OpenTerminal => {
                open_in_terminal(&self.current_path);
            }
            PaletteAction::OpenSettings => {
                self.show_settings_modal = true;
                cx.notify();
            }
            PaletteAction::Refresh => {
                self.refresh(cx);
            }
            PaletteAction::OpenFile(path) => {
                open_path(&path);
            }
            PaletteAction::SetTypeFilter(f) => {
                self.type_filter = f;
                self.sort_and_filter();
                cx.notify();
            }
            PaletteAction::SetDateFilter(f) => {
                self.date_filter = f;
                self.sort_and_filter();
                cx.notify();
            }
            PaletteAction::SetSizeFilter(f) => {
                self.size_filter = f;
                self.sort_and_filter();
                cx.notify();
            }
            PaletteAction::ResetFilters => {
                self.reset_filters(cx);
            }
            PaletteAction::SetTheme(mode) => {
                self.theme_mode = mode;
                let mut config = crate::config::Config::load();
                config.theme = match mode {
                    theme::ThemeMode::Light => "light",
                    theme::ThemeMode::Dark => "dark",
                    theme::ThemeMode::System => "system",
                }.to_string();
                config.save();
                cx.notify();
            }
        }
    }

    fn render_command_palette(&self, theme: &theme::ThemeColors, cx: &mut Context<Self>) -> AnyElement {
        let items = self.get_palette_items();
        let selected_idx = self.command_palette_selected;

        let mut rendered_items = Vec::new();
        let mut last_section = "";
        let mut row_top = 0.0;
        let mut highlight = None;

        for (idx, item) in items.iter().enumerate() {
            if item.section != last_section {
                row_top += 29.0;
                last_section = item.section;
                rendered_items.push(
                    div()
                        .px_2p5()
                        .h(px(29.))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .text_size(px(11.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.text_muted)
                        .child(item.section)
                        .into_any_element(),
                );
            }

            let is_selected = idx == selected_idx;
            let action = item.action.clone();
            let title = item.title.clone();
            let subtitle = match item.section {
                "Navigation" | "Projects" => None,
                _ => item.subtitle.clone(),
            };
            let row_height = if subtitle.is_some() { 38.0 } else { 33.0 };
            if is_selected { highlight = Some((row_top, row_height)); }
            row_top += row_height;
            let icon_name = item.icon_name;
            let shortcut = item.shortcut;

            let row = div()
                .id(ElementId::from(format!("palette-item-{}", idx)))
                .h(px(row_height))
                .flex_shrink_0()
                .px_2p5()
                .rounded(px(8.))
                .flex()
                .items_center()
                .justify_between()
                .cursor_pointer()
                .on_mouse_move(cx.listener(move |this, _ev: &MouseMoveEvent, _, cx| {
                    if this.command_palette_selected != idx {
                        this.command_palette_selected = idx;
                        cx.notify();
                    }
                }))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.execute_palette_action(action.clone(), cx);
                }))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            icon(icon_name, 15.)
                                .flex_shrink_0()
                                .text_color(if is_selected { theme.text_primary } else { theme.text_muted }),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .justify_center()
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .line_height(px(17.))
                                        .font_weight(if is_selected { gpui::FontWeight::SEMIBOLD } else { gpui::FontWeight::NORMAL })
                                        .text_color(theme.text_primary)
                                        .truncate()
                                        .child(title),
                                )
                                .when_some(subtitle, |d, sub| {
                                    d.child(
                                        div()
                                            .text_size(px(11.))
                                            .line_height(px(14.))
                                            .text_color(theme.text_muted)
                                            .truncate()
                                            .child(sub),
                                    )
                                }),
                        ),
                )
                .when_some(shortcut, |d, sc| {
                    d.child(
                        div()
                            .flex_shrink_0()
                            .ml_2()
                            .text_size(px(11.))
                            .text_color(theme.text_muted)
                            .child(sc),
                    )
                })
                .into_any_element();

            rendered_items.push(row);
        }

        let action_verb = if let Some(item) = items.get(selected_idx) {
            match item.section {
                "Navigation" | "Projects" | "Folders" => "Go to folder",
                "Files" => "Open file",
                _ => "Open",
            }
        } else {
            "Open"
        };

        // Live preview of highlighted palette item
        let selected_item = items.get(selected_idx);
        let preview_pane = if let Some(item) = selected_item {
            let item_action = item.action.clone();
            let mut preview_file: Option<FileEntry> = None;

            if let PaletteAction::OpenFile(path) = &item_action {
                if let Some(entry) = self.entries.iter().find(|e| &e.path == path) {
                    preview_file = Some(entry.clone());
                } else if let Ok(entry) = FileEntry::from_path(path) {
                    preview_file = Some(entry);
                }
            } else if let PaletteAction::Navigate(path) = &item_action {
                if let Some(entry) = self.entries.iter().find(|e| &e.path == path) {
                    preview_file = Some(entry.clone());
                } else if let Ok(entry) = FileEntry::from_path(path) {
                    preview_file = Some(entry);
                }
            }

            if let Some(entry) = preview_file {
                let thumb_path = if self.enable_thumbnails {
                    self.thumbnails.get_thumbnail_path(&entry.path, entry.content_type, entry.size)
                } else {
                    None
                };

                let parent_folder = entry.path.parent()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "/".to_string());

                div()
                    .w(px(310.))
                    .h_full()
                    .bg(if theme.is_dark { gpui::rgba(0x00000018) } else { gpui::rgba(0x00000006) })
                    .border_l_1()
                    .border_color(theme.surface_border)
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_3()
                            // Thumbnail / Icon card - bigger preview!
                            .child(
                                div()
                                    .h(px(142.))
                                    .w_full()

                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .justify_center()
                                    .gap_2()
                                    .overflow_hidden()
                                    .child(match thumb_path {
                                        Some(p) => img(p)
                                            .max_h(px(132.))
                                            .max_w(px(270.))
                                            .rounded(px(8.))
                                            .into_any_element(),
                                        None => div()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(
                                                if entry.is_directory {
                                                    img("folder.svg").size(px(72.)).into_any_element()
                                                } else {
                                                    img(SharedString::from(format!("{}.svg", entry.icon_name))).size(px(72.)).into_any_element()
                                                }
                                            )
                                            .into_any_element(),
                                    }),
                            )
                            // Title
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.text_primary)
                                    .text_center()
                                    .max_w_full()
                                    .overflow_hidden()
                                    .child(entry.name.clone()),
                            )
                            // Metadata Table (clean, no permissions row!)
                            .child(
                                div()
                                    .w_full()
                                    .pt_3()
                                    .border_t_1()
                                    .border_color(theme.surface_border)
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .text_size(px(11.))
                                    .child(
                                        div()
                                            .flex()
                                            .justify_between()
                                            .items_center()
                                            .child(div().text_color(theme.text_faint).child("Type"))
                                            .child(div().font_weight(gpui::FontWeight::MEDIUM).text_color(theme.text_primary).child(entry.type_description())),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .justify_between()
                                            .items_center()
                                            .child(div().text_color(theme.text_faint).child("Size"))
                                            .child(div().font_weight(gpui::FontWeight::MEDIUM).text_color(theme.text_primary).child(entry.formatted_size())),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .justify_between()
                                            .items_center()
                                            .child(div().text_color(theme.text_faint).child("Modified"))
                                            .child(div().font_weight(gpui::FontWeight::MEDIUM).text_color(theme.text_primary).child(entry.formatted_modified())),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .justify_between()
                                            .items_center()
                                            .child(div().text_color(theme.text_faint).child("Where"))
                                            .child(div().font_weight(gpui::FontWeight::MEDIUM).text_color(theme.text_muted).child(parent_folder)),
                                    ),
                            ),
                    )
            } else {
                div()
                    .w(px(310.))
                    .h_full()
                    .bg(if theme.is_dark { gpui::rgba(0x00000018) } else { gpui::rgba(0x00000006) })
                    .border_l_1()
                    .border_color(theme.surface_border)
                    .p_4()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_3()
                    .child(icon(item.icon_name, 56.).text_color(theme.accent))
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(theme.text_primary)
                            .child(item.title.clone()),
                    )
                    .when_some(item.subtitle.clone(), |d, sub| {
                        d.child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.text_muted)
                                .text_center()
                                .child(sub),
                        )
                    })
                    .when_some(item.shortcut, |d, sc| {
                        d.child(
                            div()
                                .px_2p5()
                                .py_1()
                                .rounded(px(4.))
                                .bg(theme.button_fill)
                                .border_1()
                                .border_color(theme.button_border)
                                .text_size(px(11.))
                                .text_color(theme.text_primary)
                                .child(format!("Shortcut: {}", sc)),
                        )
                    })
            }
        } else {
            div()
                .w(px(310.))
                .h_full()
                .bg(if theme.is_dark { gpui::rgba(0x00000018) } else { gpui::rgba(0x00000006) })
                .border_l_1()
                .border_color(theme.surface_border)
                .p_4()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(12.))
                .text_color(theme.text_faint)
                .child("Select an item to preview")
        };

        let has_query = self.command_palette_has_query;
        let search_epoch = self.command_palette_search_epoch;

        let card = div()
            .id("command-palette-card")
            .relative()
            .h(px(420.))
            .max_h_full()
            .bg(if theme.is_dark { gpui::rgba(0x1b1d24f5) } else { gpui::rgba(0xfcfcfdf8) })
            .border_1()
            .border_color(if theme.is_dark { gpui::rgba(0xffffff1c) } else { gpui::rgba(0x00000014) })
            .shadow(theme::floating_panel_shadows(theme))
            .rounded(px(14.))
            .flex()
            .flex_col()
            .overflow_hidden()
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .on_click(|_, _, cx| cx.stop_propagation())
            .with_animation(
                ("command-palette-width-spring", search_epoch),
                crate::ui::spring::SPRING_SILKY.into_animation(340),
                move |card_div, progress| {
                    let w = if search_epoch == 0 {
                        if has_query { 780.0 } else { 470.0 }
                    } else if has_query {
                        470.0 + (780.0 - 470.0) * progress
                    } else {
                        780.0 - (780.0 - 470.0) * progress
                    };
                    card_div.w(px(w.max(460.0).min(800.0)))
                },
            );

        let card = card
            // Header Input Row
            .child(
                div()
                    .min_h(px(34.))
                    .flex_shrink_0()
                    .mx_2()
                    .mt_2()
                    .px_2()
                    .rounded(px(8.))
                    .bg(theme.search_fill)
                    .border_1()
                    .border_color(theme.surface_border)
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(icon("search", 15.).flex_shrink_0().text_color(theme.text_muted))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .min_w_0()
                                    .truncate()
                                    .text_color(if self.command_palette_query.is_empty() {
                                        theme.text_muted
                                    } else {
                                        theme.text_primary
                                    })
                                    .child(if self.command_palette_query.is_empty() {
                                        "Search files and folders...".to_string()
                                    } else {
                                        format!("{}|", self.command_palette_query)
                                    }),
                            ),
                    )
                    .child(
                        icon_button("close-palette", "cancel-01", false, true)
                            .size(px(24.))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.close_command_palette(cx);
                            })),
                    ),
            )
            // Split Body: Left List + Right Preview
            .child(
                div()
                    .flex_1()
                    .flex()
                    .overflow_hidden()
                    // Left list & footer (pinned at 468px)
                    .child(
                        div()
                            .w(px(468.))
                            .flex_shrink_0()
                            .h_full()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .id("palette-items-list")
                                    .flex_1()
                                    .overflow_y_scroll()
                                    .px_2()
                                    .pb_2()
                                    .flex()
                                    .flex_col()
                                    .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                                    .child(
                                        div().relative().flex().flex_col()
                                            .when_some(highlight, |list, (top, height)| {
                                                list.child(
                                                    div().id("palette-hover-pill").absolute()
                                                        .left_0().right_0().rounded(px(8.))
                                                        .bg(theme.sidebar_selected)
                                                        .with_spring(
                                                            "palette-hover-position",
                                                            gpui::SpringAnimation::new(gpui::SpringConfig::new(420.0, 32.0, 0.55)).to(top),
                                                            move |pill, y| pill.top(px(y)).h(px(height)),
                                                        ),
                                                )
                                            })
                                            .children(rendered_items),
                                    ),
                            )

                    )
                    // Right Preview Pane with Spring Fade & Reveal
                    .child(
                        div()
                            .w(px(310.))
                            .flex_shrink_0()
                            .h_full()
                            .overflow_hidden()
                            .with_animation(
                                ("palette-preview-fade", search_epoch),
                                crate::ui::spring::SPRING_SILKY.into_animation(300),
                                move |fade_div, progress| {
                                    let opacity = if search_epoch == 0 {
                                        if has_query { 1.0 } else { 0.0 }
                                    } else {
                                        let p = progress.clamp(0.0, 1.0);
                                        if has_query { p } else { 1.0 - p }
                                    };
                                    fade_div.opacity(opacity)
                                },
                            )
                            .child(div().id("palette-preview-scroll").h_full().overflow_y_scroll().child(preview_pane)),
                    ),
            )
            // Shared footer across results and preview.
            .child(
                                div()
                                    .h(px(32.))
                                    .flex_shrink_0()
                                    .px_3()
                                    .border_t_1()
                                    .border_color(theme.surface_border)
                                    .flex()
                                    .items_center()
                                    .justify_end()
                                    .text_size(px(11.))
                                    .text_color(theme.text_muted)
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_4()
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_1p5()
                                                    .child(action_verb)
                                                    .child(
                                                        div()
                                                            .text_size(px(12.))
                                                            .text_color(theme.text_muted)
                                                            .child("↵"),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_1p5()
                                                    .child("Close")
                                                    .child(
                                                        div()
                                                            .px_1()
                                                            .py_0p5()
                                                            .rounded(px(3.))
                                                            .bg(theme.button_fill)
                                                            .border_1()
                                                            .border_color(theme.button_border)
                                                            .text_size(px(9.))
                                                            .text_color(theme.text_muted)
                                                            .child("Esc"),
                                                    ),
                                            ),
                                    ),
                             );

        div()
            .id("command-palette-backdrop")
            .absolute()
            .inset_0()
            .bg(if theme.is_dark { gpui::rgba(0x00000045) } else { gpui::rgba(0x0000001c) })
            .flex()
            .items_center()
            .justify_center()
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(|this, _, _, cx| {
                this.close_command_palette(cx);
            }))
            .child(card)
            .into_any_element()
    }

    fn render_traffic_lights(&self, cx: &mut Context<Self>) -> Div {
        div()
            .h(px(TITLE_BAR_HEIGHT))
            .pl(px(16.))
            .pr_3()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(
                div()
                    .id("traffic-light-close")
                    .size(px(12.))
                    .rounded_full()
                    .bg(theme::mac_close())
                    .border_1()
                    .border_color(gpui::rgba(0x00000040))
                    .cursor_pointer()
                    .hover(|s| s.opacity(0.85))
                    .active(|s| s.opacity(0.7))
                    .on_click(cx.listener(|_, _, window, _| {
                        window.remove_window();
                    })),
            )
            .child(
                div()
                    .id("traffic-light-minimize")
                    .size(px(12.))
                    .rounded_full()
                    .bg(theme::mac_minimize())
                    .border_1()
                    .border_color(gpui::rgba(0x00000040))
                    .cursor_pointer()
                    .hover(|s| s.opacity(0.85))
                    .active(|s| s.opacity(0.7))
                    .on_click(cx.listener(|_, _, window, _| {
                        window.minimize_window();
                    })),
            )
            .child(
                div()
                    .id("traffic-light-zoom")
                    .size(px(12.))
                    .rounded_full()
                    .bg(theme::mac_zoom())
                    .border_1()
                    .border_color(gpui::rgba(0x00000040))
                    .cursor_pointer()
                    .hover(|s| s.opacity(0.85))
                    .active(|s| s.opacity(0.7))
                    .on_click(cx.listener(|_, _, window, _| {
                        window.zoom_window();
                    })),
            )
    }

    fn render_sidebar(&self, theme: &theme::ThemeColors, cx: &mut Context<Self>) -> Div {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let favorites: Vec<_> = [
            ("place-desktop", "Desktop", "app-window", dirs::desktop_dir()),
            ("place-documents", "Documents", "document", dirs::document_dir()),
            ("place-downloads", "Downloads", "download", dirs::download_dir()),
            ("place-music", "Music", "audio", dirs::audio_dir()),
            ("place-pictures", "Pictures", "image", dirs::picture_dir()),
            ("place-videos", "Videos", "video", dirs::video_dir()),
        ].into_iter().filter_map(|(id, label, icon, path)| {
            path.filter(|path| path != &home && path.is_dir()).map(|path| (id, label, icon, path))
        }).collect();

        let mut locations = vec![
            ("loc-user", "Home", "user", home),
            ("loc-root", "File System", "hard-drive", PathBuf::from("/")),
        ];
        if let Some(data_dir) = dirs::data_local_dir() {
            locations.push(("place-trash", "Trash", "trash", data_dir.join("Trash/files")));
        }

        let tags: Vec<(&'static str, &'static str, Hsla)> = vec![
            ("tag-red", "Important", gpui::rgb(0xff453a).into()),
            ("tag-orange", "Orange", gpui::rgb(0xff9f0a).into()),
            ("tag-yellow", "Review", gpui::rgb(0xffd60a).into()),
            ("tag-green", "Work", gpui::rgb(0x30d158).into()),
            ("tag-blue", "Personal", gpui::rgb(0x0a84ff).into()),
            ("tag-purple", "Ideas", gpui::rgb(0xbf5af2).into()),
        ];

        div()
            .w(px(theme::SIDEBAR_PANEL_WIDTH))
            .h_full()
            .flex_shrink_0()
            .relative()
            .bg(theme.sidebar)
            .rounded(px(RADIUS_SIDEBAR))
            .border_1()
            .border_color(theme.sidebar_border)
            .shadow(theme::floating_panel_shadows(theme))
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(theme::glass_specular_sheen(theme))
            .child(self.render_traffic_lights(cx))
            .child(
                div()
                    .id("sidebar-sections-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px_3()
                    .pb_3()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .when(!favorites.is_empty(), |sidebar| {
                        sidebar.child(self.render_sidebar_section("Favorites", favorites, theme, cx))
                    })
                    .child(self.render_sidebar_section("Locations", locations, theme, cx))
                    .child(self.render_sidebar_tags_section("Tags", tags, theme, cx)),
            )
    }

    fn render_sidebar_section(
        &self,
        title: &'static str,
        items: Vec<(&'static str, &'static str, &'static str, PathBuf)>,
        theme: &theme::ThemeColors,
        cx: &mut Context<Self>,
    ) -> Div {
        div()
            .flex()
            .flex_col()
            .gap_0p5()
            .child(
                div()
                    .px_2()
                    .pb_1()
                    .flex()
                    .items_center()
                    .text_size(px(theme::TEXT_CAPTION))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(theme.text_muted)
                    .child(title),
            )
            .children(items.into_iter().map(|(id, label, icon_name, path)| {
                let is_active = self.current_path == path;
                let target = path.clone();

                div()
                    .id(ElementId::from(id))
                    .h(px(theme::SIDEBAR_ROW_HEIGHT))
                    .px_2()
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .gap_2()
                    .cursor_pointer()
                    .when(is_active, |d| {
                        d.bg(theme.sidebar_selected)
                    })
                    .when(!is_active, |d| {
                        d.hover(|s| s.bg(theme.sidebar_hover))
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.navigate_to(target.clone(), cx);
                    }))
                    .child(
                        theme::icon(icon_name, 14.)
                            .text_color(if is_active { theme.accent } else { theme.text_muted }),
                    )
                    .child(
                        div()
                            .text_size(px(theme::TEXT_BODY))
                            .text_color(if is_active { theme.accent } else { theme.text_primary })
                            .font_weight(if is_active { gpui::FontWeight::MEDIUM } else { gpui::FontWeight::NORMAL })
                            .child(label),
                    )
            }))
    }

    fn render_sidebar_tags_section(
        &self,
        title: &'static str,
        tags: Vec<(&'static str, &'static str, Hsla)>,
        theme: &theme::ThemeColors,
        cx: &mut Context<Self>,
    ) -> Div {
        div()
            .flex()
            .flex_col()
            .gap_0p5()
            .child(
                div()
                    .px_2()
                    .pb_1()
                    .flex()
                    .items_center()
                    .text_size(px(theme::TEXT_CAPTION))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(theme.text_muted)
                    .child(title),
            )
            .children(tags.into_iter().map(|(id, label, color)| {
                let has_tag = self.selected_index.and_then(|i| self.entry_at(i))
                    .and_then(|entry| self.tags.get(&entry.path))
                    .is_some_and(|tags| tags.iter().any(|tag| tag == label));
                div()
                    .h(px(theme::SIDEBAR_ROW_HEIGHT))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .when(self.active_tag == Some(label), |row| row.bg(theme.sidebar_selected))
                    .child(
                        div().id(id).role(Role::Button).flex_1().h_full().px_2()
                            .rounded(px(6.)).flex().items_center().gap_2().cursor_pointer()
                            .hover(|style| style.bg(theme.sidebar_hover))
                            .aria_label(format!("Filter this folder by {label}"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.active_tag = (this.active_tag != Some(label)).then_some(label);
                                this.selected_index = None;
                                this.scroll_handle = UniformListScrollHandle::new();
                                this.sort_and_filter();
                                cx.notify();
                            }))
                            .child(div().size(px(10.)).rounded_full().bg(color))
                            .child(div().text_size(px(theme::TEXT_BODY)).text_color(theme.text_primary).child(label)),
                    )
                    .when(self.selected_index.and_then(|i| self.entry_at(i)).is_some_and(|entry| entry.path.to_str().is_some()), |row| {
                        row.child(
                            icon_button(SharedString::from(format!("assign-{id}")), if has_tag { "check-hi" } else { "add-01" }, has_tag, true)
                                .size(px(24.))
                                .aria_label(format!("{} {label} tag", if has_tag { "Remove" } else { "Add" }))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    let Some(path) = this.selected_index.and_then(|i| this.entry_at(i)).map(|entry| entry.path.clone()) else { return; };
                                    // shortcut: tags are stored by path, migrate to file identity when tracking moves.
                                    let mut config = crate::config::Config::load();
                                    config.toggle_tag(&path, label);
                                    config.save();
                                    this.tags = config.tags;
                                    this.sort_and_filter();
                                    this.selected_index = this.filtered_indices.iter()
                                        .position(|&index| this.entries[index].path == path);
                                    cx.notify();
                                })),
                        )
                    })
            }))
    }

    fn render_pane(&mut self, theme: &theme::ThemeColors, cx: &mut Context<Self>) -> AnyElement {
        if self.filtered_indices.is_empty() {
            return div()
                .flex_1()
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.text_faint)
                .text_size(px(theme::TEXT_BODY))
                .child(if let Some(error) = &self.scan_error { format!("Could not open folder: {error}") } else if self.active_tag.is_some() { "No files with this tag in this folder".to_owned() } else { "This folder is empty".to_owned() })
                .into_any_element();
        }

        match self.view_mode {
            ViewMode::List => {
                let header_row = div()
                    .h(px(28.))
                    .px_3()
                    .border_b_1()
                    .border_color(theme.surface_border)
                    .flex()
                    .items_center()
                    .text_size(px(theme::TEXT_CAPTION))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(theme.text_faint)
                    .child(
                        div()
                            .id("col-header-name")
                            .flex_1()
                            .flex()
                            .items_center()
                            .gap_1()
                            .cursor_pointer()
                            .hover(|s| s.text_color(theme.text_primary))
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_sort(SortBy::Name, cx)))
                            .child("NAME")
                            .when(self.sort_by == SortBy::Name, |d| {
                                d.child(icon(if self.sort_reversed { "chevron-down-hi" } else { "chevron-left" }, 10.))
                            }),
                    )
                    .child(
                        div()
                            .id("col-header-size")
                            .w(px(100.))
                            .text_right()
                            .cursor_pointer()
                            .hover(|s| s.text_color(theme.text_primary))
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_sort(SortBy::Size, cx)))
                            .child("SIZE"),
                    )
                    .child(
                        div()
                            .id("col-header-type")
                            .w(px(130.))
                            .px_2()
                            .cursor_pointer()
                            .hover(|s| s.text_color(theme.text_primary))
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_sort(SortBy::Type, cx)))
                            .child("TYPE"),
                    )
                    .child(
                        div()
                            .id("col-header-modified")
                            .w(px(140.))
                            .cursor_pointer()
                            .hover(|s| s.text_color(theme.text_primary))
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_sort(SortBy::Modified, cx)))
                            .child("MODIFIED"),
                    );

                let list = uniform_list(
                    "list-view",
                    self.filtered_indices.len(),
                    cx.processor(Self::render_list_rows),
                )
                .track_scroll(&self.scroll_handle)
                .size_full()
                .min_h_0();

                div()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .h_full()
                    .relative()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(header_row)
                    .child(list)
                    .into_any_element()
            }
            ViewMode::Grid => {
                let cols = self.grid_cols.max(1);
                let num_rows = (self.filtered_indices.len() + cols - 1) / cols;

                let grid = uniform_list(
                    "grid-view",
                    num_rows,
                    cx.processor(Self::render_grid_rows),
                )
                .track_scroll(&self.scroll_handle)
                .size_full()
                .min_h_0()
                .p_3();

                div()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .h_full()
                    .relative()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(grid)
                    .into_any_element()
            }
        }
    }

    fn render_list_rows(
        &mut self,
        range: std::ops::Range<usize>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let theme = self.active_theme;
        range
            .map(|i| {
                let entry = &self.entries[self.filtered_indices[i]];
                let is_selected = self.selected_index == Some(i);
                let thumb_path = self.thumbnails.get_thumbnail_path(&entry.path, entry.content_type, entry.size);

                div()
                    .id(ElementId::from(format!("list-row-{}", i)))
                    .w_full()
                    .h(px(theme::ROW_HEIGHT))
                    .min_h(px(theme::ROW_HEIGHT))
                    .max_h(px(theme::ROW_HEIGHT))
                    .flex_shrink_0()
                    .px_3()
                    .rounded(px(RADIUS_THUMB))
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .when(is_selected, |d| {
                        d.bg(theme.selection)
                            .border_1()
                            .border_color(theme.accent)
                    })
                    .when(!is_selected, |d| {
                        d.hover(|s| s.bg(theme.card_hover))
                    })
                    .on_click(cx.listener(move |this, ev: &ClickEvent, _, cx| {
                        if ev.click_count() >= 2 {
                            this.open_selected(cx);
                        } else {
                            this.selected_index = Some(i);
                            cx.notify();
                        }
                    }))
                    // Name & Icon
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .gap_2()
                            .overflow_hidden()
                            .child(match thumb_path {
                                Some(p) => img(p)
                                    .size(px(18.))
                                    .rounded(px(2.))
                                    .into_any_element(),
                                None => {
                                    if entry.is_directory {
                                        img("folder.svg")
                                            .size(px(18.))
                                            .into_any_element()
                                    } else {
                                        img(SharedString::from(format!("{}.svg", entry.icon_name)))
                                            .size(px(18.))
                                            .into_any_element()
                                    }
                                }
                            })
                            .child(
                                div()
                                    .text_size(px(theme::TEXT_BODY))
                                    .text_color(if is_selected {
                                        theme.text_primary
                                    } else {
                                        theme.text_primary
                                    })
                                    .font_weight(if entry.is_directory {
                                        gpui::FontWeight::SEMIBOLD
                                    } else {
                                        gpui::FontWeight::NORMAL
                                    })
                                    .overflow_hidden()
                                    .child(entry.name.clone()),
                            ),
                    )
                    // Size
                    .child(
                        div()
                            .w(px(100.))
                            .text_right()
                            .text_size(px(theme::TEXT_SMALL))
                            .text_color(theme.text_muted)
                            .child(entry.formatted_size()),
                    )
                    // Type
                    .child(
                        div()
                            .w(px(130.))
                            .px_2()
                            .text_size(px(theme::TEXT_SMALL))
                            .text_color(theme.text_faint)
                            .overflow_hidden()
                            .child(entry.type_description()),
                    )
                    // Modified
                    .child(
                        div()
                            .w(px(140.))
                            .text_size(px(theme::TEXT_SMALL))
                            .text_color(theme.text_muted)
                            .child(entry.formatted_modified()),
                    )
                    .into_any_element()
            })
            .collect()
    }

    fn render_grid_rows(
        &mut self,
        range: std::ops::Range<usize>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let theme = self.active_theme;
        let cols = self.grid_cols.max(1);
        let card_width = self.zoom;
        let thumb_area_height = (card_width * 0.72).clamp(40., 220.);
        let is_compact = card_width < 95.;
        let text_height = if is_compact { (self.font_size * 2.0).max(28.) } else { (self.font_size * 2.8).max(40.) };
        let card_height = thumb_area_height + text_height + 14.;
        let entries_count = self.filtered_indices.len();

        range
            .map(|row_idx| {
                let start = row_idx * cols;
                let end = (start + cols).min(entries_count);

                div()
                    .id(ElementId::from(format!("grid-row-{}", row_idx)))
                    .w_full()
                    .h(px(card_height + 8.))
                    .px_3()
                    .py_1()
                    .flex()
                    .gap_3()
                    .children((start..end).map(|i| {
                        let entry = &self.entries[self.filtered_indices[i]];
                        let is_selected = self.selected_index == Some(i);
                        let thumb_path = self.thumbnails.get_thumbnail_path(&entry.path, entry.content_type, entry.size);

                        div()
                            .id(ElementId::from(format!("grid-card-{}", i)))
                            .w(px(card_width))
                            .h(px(card_height))
                            .p_1p5()
                            .rounded(px(RADIUS_ROW))
                            .when(is_selected, |d| {
                                d.bg(theme.selection)
                                    .border_1()
                                    .border_color(theme.accent)
                            })
                            .when(!is_selected, |d| {
                                d.hover(|s| s.bg(theme.card_hover))
                            })
                            .cursor_pointer()
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_between()
                            .on_click(cx.listener(move |this, ev: &ClickEvent, _, cx| {
                                if ev.click_count() >= 2 {
                                    this.open_selected(cx);
                                } else {
                                    this.selected_index = Some(i);
                                    cx.notify();
                                }
                            }))
                            // Thumbnail / Icon Area - Fixed height so it NEVER shrinks to 0
                            .child(
                                div()
                                    .h(px(thumb_area_height))
                                    .w_full()
                                    .flex_shrink_0()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .overflow_hidden()
                                    .child(match thumb_path {
                                        Some(p) => img(p)
                                            .max_w(px(card_width - 8.))
                                            .max_h(px(thumb_area_height))
                                            .rounded(px(RADIUS_THUMB))
                                            .into_any_element(),
                                        None => {
                                            if entry.is_directory {
                                                let icon_sz = (thumb_area_height * 0.90).clamp(28., 180.);
                                                img("folder.svg")
                                                    .size(px(icon_sz))
                                                    .into_any_element()
                                            } else {
                                                let icon_sz = (thumb_area_height * 0.85).clamp(26., 160.);
                                                img(SharedString::from(format!("{}.svg", entry.icon_name)))
                                                    .size(px(icon_sz))
                                                    .into_any_element()
                                            }
                                        }
                                    }),
                            )
                            // Title & Info Area - Clamped to prevent crushing thumbnail
                            .child(
                                div()
                                    .w_full()
                                    .h(px(text_height))
                                    .flex_shrink_0()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .justify_center()
                                    .overflow_hidden()
                                    .child(
                                        div()
                                            .text_size(px(if is_compact { (self.font_size * 0.85).max(11.) } else { (self.font_size * 0.95).max(13.) }))
                                            .text_color(theme.text_primary)
                                            .text_center()
                                            .line_clamp(2)
                                            .font_weight(if entry.is_directory {
                                                gpui::FontWeight::SEMIBOLD
                                            } else {
                                                gpui::FontWeight::NORMAL
                                            })
                                            .max_w_full()
                                            .overflow_hidden()
                                            .child(entry.name.clone()),
                                    )
                                    .when(!is_compact, |d| {
                                        d.child(
                                            div()
                                                .text_size(px((self.font_size * 0.82).max(11.)))
                                                .text_color(theme.text_faint)
                                                .child(entry.formatted_size()),
                                        )
                                    }),
                            )
                            .into_any_element()
                    }))
                    .into_any_element()
            })
            .collect()
    }

    fn render_inspector(&self, theme: &theme::ThemeColors, cx: &mut Context<Self>) -> Div {
        let selected_entry = self.selected_index.and_then(|i| self.entry_at(i));

        div()
            .w(px(270.))
            .h_full()
            .flex_shrink_0()
            .bg(theme.sidebar)
            .rounded(px(theme::RADIUS_PANEL))
            .border_1()
            .border_color(theme.surface_border)
            .shadow(theme::floating_panel_shadows(theme))
            .p_4()
            .overflow_hidden()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(theme::TEXT_HEADER))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.text_primary)
                            .child("Properties"),
                    )
                    .child(icon("sliders", 14.).text_color(theme.text_faint)),
            )
            .child(match selected_entry {
                Some(entry) => self.render_inspector_file(entry, theme, cx).into_any_element(),
                None => self.render_inspector_folder(theme, cx).into_any_element(),
            })
    }

    fn render_inspector_file(&self, entry: &FileEntry, theme: &theme::ThemeColors, cx: &mut Context<Self>) -> Div {
        let path = entry.path.clone();
        let thumb_path = self.thumbnails.get_thumbnail_path(&entry.path, entry.content_type, entry.size);

        let preview_box = div()
            .w_full()
            .h(px(140.))
            .bg(theme.card)
            .rounded(px(RADIUS_CARD))
            .border_1()
            .border_color(theme.card_border)
            .flex()
            .items_center()
            .justify_center()
            .overflow_hidden()
            .child(match thumb_path {
                Some(p) => img(p)
                    .max_w(px(130.))
                    .max_h(px(130.))
                    .rounded(px(theme::RADIUS_THUMB))
                    .into_any_element(),
                None => {
                    if entry.is_directory {
                        img("folder.svg")
                            .size(px(72.))
                            .into_any_element()
                    } else {
                        img(SharedString::from(format!("{}.svg", entry.icon_name)))
                            .size(px(72.))
                            .into_any_element()
                    }
                },
            });

        let file_name = div()
            .text_size(px(theme::TEXT_TITLE))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(theme.text_primary)
            .child(entry.name.clone());

        let meta_list = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(Self::meta_row("Size", &entry.formatted_size(), theme))
            .child(Self::meta_row("Type", &entry.type_description(), theme))
            .child(Self::meta_row("Modified", &entry.formatted_modified(), theme))
            .child(Self::meta_row("Permissions", &entry.formatted_permissions(), theme))
            .child(Self::meta_row("Location", &entry.path.parent().unwrap_or(&entry.path).to_string_lossy(), theme));

        let target_file = path.clone();
        let target_term = path.clone();

        let actions = div()
            .flex()
            .flex_col()
            .gap_2()
            .mt_auto()
            .child(
                button("inspector-btn-open", ButtonLook::Primary)
                    .w_full()
                    .on_click(cx.listener(move |_, _, _, _| {
                        open_path(&target_file);
                    }))
                    .child("Open"),
            )
            .child(
                button("inspector-btn-term", ButtonLook::Secondary)
                    .w_full()
                    .on_click(cx.listener(move |_, _, _, _| {
                        open_in_terminal(&target_term);
                    }))
                    .child("Open in Terminal"),
            );

        div()
            .flex_1()
            .flex()
            .flex_col()
            .gap_3()
            .child(preview_box)
            .child(file_name)
            .child(meta_list)
            .child(actions)
    }

    fn render_inspector_folder(&self, theme: &theme::ThemeColors, cx: &mut Context<Self>) -> Div {
        let name = self.current_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string());

        let free_space = crate::utils::get_disk_free_space(&self.current_path).unwrap_or_else(|| "Unknown".to_string());
        let target_term = self.current_path.clone();

        div()
            .flex_1()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .w_full()
                    .h(px(140.))
                    .bg(theme.card)
                    .rounded(px(RADIUS_CARD))
                    .border_1()
                    .border_color(theme.card_border)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(img("folder.svg").size(px(72.))),
            )
            .child(
                div()
                    .text_size(px(theme::TEXT_TITLE))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.text_primary)
                    .child(name),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Self::meta_row("Type", "Directory", theme))
                    .child(Self::meta_row("Disk Free", &format!("{} free", free_space), theme))
                    .child(Self::meta_row("Path", &self.current_path.to_string_lossy(), theme)),
            )
            .child(
                div()
                    .mt_auto()
                    .child(
                        button("inspector-btn-term-folder", ButtonLook::Secondary)
                            .w_full()
                            .on_click(cx.listener(move |_, _, _, _| {
                                open_in_terminal(&target_term);
                            }))
                            .child("Open in Terminal"),
                    ),
            )
    }

    fn meta_row(label: &'static str, value: &str, theme: &theme::ThemeColors) -> Div {
        div()
            .flex()
            .justify_between()
            .items_start()
            .text_size(px(theme::TEXT_SMALL))
            .child(
                div()
                    .text_color(theme.text_faint)
                    .child(label),
            )
            .child(
                div()
                    .text_color(theme.text_primary)
                    .max_w(px(160.))
                    .overflow_hidden()
                    .child(value.to_string()),
            )
    }
}

impl Render for MainWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let win_size = window.viewport_size();
        let cur_w = (win_size.width / px(1.0)).round() as i32;
        let cur_h = (win_size.height / px(1.0)).round() as i32;
        if cur_w > 0 && cur_h > 0 && (cur_w, cur_h) != self.window_size {
            self.window_size = (cur_w, cur_h);
            if let Some(coord) = &mut self.blur_coordinator {
                coord.update_bounds(cur_w, cur_h);
            }
        }

        self.active_theme = theme::ThemeColors::resolve(self.theme_mode, window, &self.background_mode);
        let theme = self.active_theme;

        if self.appearance_subscription.is_none() {
            self.appearance_subscription = Some(cx.observe_window_appearance(window, |this, window, cx| {
                this.active_theme = theme::ThemeColors::resolve(this.theme_mode, window, &this.background_mode);
                cx.notify();
            }));
        }

        if self.bounds_subscription.is_none() {
            self.bounds_subscription = Some(cx.observe_window_bounds(window, |_this, _window, cx| {
                cx.notify();
            }));
        }

        let total_items = self.filtered_indices.len();
        let selected_size = self.selected_index.and_then(|idx| self.entry_at(idx)).map(|e| e.formatted_size());
        let free_space = crate::utils::get_disk_free_space(&self.current_path).unwrap_or_else(|| "0 GB".to_string());
        let focus_handle = self.focus_handle.clone();

        // Dynamic responsive column calculation across pane width
        let window_width = f32::from(window.viewport_size().width);
        let sidebar_slot = theme::PANEL_MARGIN + theme::SIDEBAR_PANEL_WIDTH + theme::PANEL_GAP + theme::PANEL_MARGIN;
        let inspector_width = if self.show_inspector { 270.0 } else { 0.0 };
        let inspector_gap = if self.show_inspector { 8.0 } else { 0.0 };
        let pane_width = (window_width - sidebar_slot - inspector_width - inspector_gap - 16.0).max(200.0);

        let card_width = self.zoom;
        let gap = 12.0;
        let cols = ((pane_width + gap) / (card_width + gap)).floor().max(1.0) as usize;
        self.grid_cols = cols;

        let toolbar = self.render_header(&theme, cx);
        let sidebar = self.render_sidebar(&theme, cx);
        let inspector = if self.show_inspector {
            Some(self.render_inspector(&theme, cx))
        } else {
            None
        };
        let pane = self.render_pane(&theme, cx);

        // Bottom Status Bar
        let status_bar = div()
            .h(px(24.))
            .flex_shrink_0()
            .px_3()
            .border_t_1()
            .border_color(theme.surface_border)
            .flex()
            .items_center()
            .justify_between()
            .text_size(px(theme::TEXT_CAPTION))
            .text_color(theme.text_faint)
            .child(format!(
                "{} items{}",
                total_items,
                if let Some(size) = selected_size {
                    format!(" | 1 selected ({})", size)
                } else {
                    String::new()
                }
            ))
            .child(format!("{} free", free_space));

        let content_panel = div()
            .flex_1()
            .min_w_0()
            .h_full()
            .min_h_0()
            .overflow_hidden()
            .flex()
            .flex_col()
            .when(self.has_active_filters(), |d| {
                d.child(self.render_active_filters_bar(&theme, cx))
            })
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_row()
                    .overflow_hidden()
                    .child(pane)
                    .children(inspector),
            )
            .child(status_bar);

        // Keyboard & Scroll wheel handlers (Ctrl+Scroll for zooming)
        div()
            .id("shark-window-root")
            .relative()
            .font_family("SF Pro")
            .track_focus(&focus_handle)
            .size_full()
            .bg(theme.canvas)
            .rounded(px(16.))
            .border_1()
            .border_color(theme.surface_border)
            .flex()
            .flex_col()
            .overflow_hidden()
            .on_scroll_wheel(cx.listener(|this, ev: &ScrollWheelEvent, _, cx| {
                if ev.modifiers.control {
                    cx.stop_propagation();
                    let delta = match ev.delta {
                        gpui::ScrollDelta::Lines(l) => l.y,
                        gpui::ScrollDelta::Pixels(p) => f32::from(p.y) / 20.,
                    };
                    if delta > 0. {
                        this.zoom_in(cx);
                    } else if delta < 0. {
                        this.zoom_out(cx);
                    }
                }
            }))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                let key = ev.keystroke.key.as_str();
                let modifiers = ev.keystroke.modifiers;

                // 0. Command Palette Mode
                if this.show_command_palette {
                    let items = this.get_palette_items();
                    match key {
                        "escape" => {
                            this.close_command_palette(cx);
                        }
                        "enter" => {
                            if !items.is_empty() {
                                let idx = this.command_palette_selected.min(items.len().saturating_sub(1));
                                let action = items[idx].action.clone();
                                this.execute_palette_action(action, cx);
                            }
                        }
                        "up" => {
                            if !items.is_empty() {
                                let prev = if this.command_palette_selected == 0 {
                                    items.len().saturating_sub(1)
                                } else {
                                    this.command_palette_selected - 1
                                };
                                this.command_palette_selected = prev;
                                cx.notify();
                            }
                        }
                        "down" => {
                            if !items.is_empty() {
                                let next = (this.command_palette_selected + 1) % items.len();
                                this.command_palette_selected = next;
                                cx.notify();
                            }
                        }
                        "backspace" => {
                            let mut q = this.command_palette_query.clone();
                            q.pop();
                            this.update_command_palette_query(q, cx);
                        }
                        _ => {
                            if modifiers.control {
                                match key {
                                    "n" => { this.execute_palette_action(PaletteAction::NewFolder, cx); return; }
                                    "v" => { this.execute_palette_action(PaletteAction::ToggleViewMode, cx); return; }
                                    "h" => { this.execute_palette_action(PaletteAction::ToggleHidden, cx); return; }
                                    "i" => { this.execute_palette_action(PaletteAction::ToggleInspector, cx); return; }
                                    "t" => { this.execute_palette_action(PaletteAction::OpenTerminal, cx); return; }
                                    "," => { this.execute_palette_action(PaletteAction::OpenSettings, cx); return; }
                                    "r" => { this.execute_palette_action(PaletteAction::Refresh, cx); return; }
                                    _ => {}
                                }
                            } else if !modifiers.platform && !modifiers.alt {
                                let mut q = this.command_palette_query.clone();
                                if let Some(ch) = &ev.keystroke.key_char {
                                    q.push_str(ch);
                                    this.update_command_palette_query(q, cx);
                                } else if key.chars().count() == 1 {
                                    q.push_str(key);
                                    this.update_command_palette_query(q, cx);
                                }
                            }
                        }
                    }
                    return;
                }

                // Global Shortcuts
                if modifiers.control && (key == "k" || key == "p" || key == "f") || (key == "/" && !this.path_editing) {
                    this.open_command_palette(None, cx);
                    return;
                }
                if modifiers.control && key == "l" {
                    this.path_editing = true;
                    this.search_active = false;
                    this.path_input_text = this.current_path.to_string_lossy().to_string();
                    cx.notify();
                    return;
                }
                if modifiers.control && key == "h" {
                    this.show_hidden = !this.show_hidden;
                    this.sort_and_filter();
                    cx.notify();
                    return;
                }
                if (modifiers.alt && key == "up") || (key == "backspace" && !this.search_active && !this.path_editing) {
                    this.go_up(cx);
                    return;
                }
                if modifiers.alt && key == "left" {
                    this.go_back(cx);
                    return;
                }
                if modifiers.alt && key == "right" {
                    this.go_forward(cx);
                    return;
                }

                // 1. Path URL Editing Mode
                if this.path_editing {
                    match key {
                        "escape" => {
                            this.path_editing = false;
                            cx.notify();
                        }
                        "enter" => {
                            this.commit_path_input(cx);
                        }
                        "backspace" => {
                            this.path_input_text.pop();
                            cx.notify();
                        }
                        _ => {
                            if !modifiers.control && !modifiers.platform && !modifiers.alt {
                                if let Some(ch) = &ev.keystroke.key_char {
                                    this.path_input_text.push_str(ch);
                                    cx.notify();
                                } else if key.chars().count() == 1 {
                                    this.path_input_text.push_str(key);
                                    cx.notify();
                                }
                            }
                        }
                    }
                    return;
                }

                // 2. Normal File View Mode
                match key {
                    "up" => {
                        let prev = this.selected_index.map(|i| i.saturating_sub(1)).unwrap_or(0);
                        this.selected_index = Some(prev);
                        cx.notify();
                    }
                    "down" => {
                        let max = this.filtered_indices.len().saturating_sub(1);
                        let next = this.selected_index.map(|i| (i + 1).min(max)).unwrap_or(0);
                        this.selected_index = Some(next);
                        cx.notify();
                    }
                    "enter" => {
                        this.open_selected(cx);
                    }
                    "escape" => {
                        this.selected_index = None;
                        this.show_settings_modal = false;
                        this.show_filter_menu = false;
                        this.show_more_menu = false;
                        this.close_command_palette(cx);
                        this.search_query.clear();
                        this.sort_and_filter();
                        cx.notify();
                    }
                    _ => {
                        // Type-to-search anywhere in the window
                        if !modifiers.control && !modifiers.platform && !modifiers.alt {
                            if let Some(ch) = &ev.keystroke.key_char {
                                if !ch.trim().is_empty() {
                                    this.open_command_palette(Some(ch), cx);
                                }
                            } else if key.chars().count() == 1 && !key.trim().is_empty() {
                                this.open_command_palette(Some(key), cx);
                            }
                        }
                    }
                }
            }))
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_row()
                    .p_2()
                    .gap_3()
                    .overflow_hidden()
                    .child(sidebar)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .child(toolbar)
                            .child(content_panel),
                    ),
            )
            .when(self.show_filter_menu, |d| {
                d.child(self.render_filter_popover(&theme, cx))
            })
            .when(self.show_more_menu, |d| {
                d.child(self.render_more_menu(&theme, cx))
            })
            .when(self.show_settings_modal, |d| {
                d.child(self.render_settings_modal(&theme, cx))
            })
            .when(self.show_command_palette, |d| {
                d.child(self.render_command_palette(&theme, cx))
            })
            // 8 edge / corner interactive resize handles for Wayland/X11
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .child(
                        div()
                            .id("edge-top")
                            .absolute()
                            .top_0()
                            .left(px(14.))
                            .right(px(14.))
                            .h(px(6.))
                            .cursor(CursorStyle::ResizeUpDown)
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.start_window_resize(ResizeEdge::Top);
                            })
                    )
                    .child(
                        div()
                            .id("edge-bottom")
                            .absolute()
                            .bottom_0()
                            .left(px(14.))
                            .right(px(14.))
                            .h(px(6.))
                            .cursor(CursorStyle::ResizeUpDown)
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.start_window_resize(ResizeEdge::Bottom);
                            })
                    )
                    .child(
                        div()
                            .id("edge-left")
                            .absolute()
                            .left_0()
                            .top(px(14.))
                            .bottom(px(14.))
                            .w(px(6.))
                            .cursor(CursorStyle::ResizeLeftRight)
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.start_window_resize(ResizeEdge::Left);
                            })
                    )
                    .child(
                        div()
                            .id("edge-right")
                            .absolute()
                            .right_0()
                            .top(px(14.))
                            .bottom(px(14.))
                            .w(px(6.))
                            .cursor(CursorStyle::ResizeLeftRight)
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.start_window_resize(ResizeEdge::Right);
                            })
                    )
                    .child(
                        div()
                            .id("corner-top-left")
                            .absolute()
                            .top_0()
                            .left_0()
                            .size(px(14.))
                            .cursor(CursorStyle::ResizeUpRightDownLeft)
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.start_window_resize(ResizeEdge::TopLeft);
                            })
                    )
                    .child(
                        div()
                            .id("corner-top-right")
                            .absolute()
                            .top_0()
                            .right_0()
                            .size(px(14.))
                            .cursor(CursorStyle::ResizeUpLeftDownRight)
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.start_window_resize(ResizeEdge::TopRight);
                            })
                    )
                    .child(
                        div()
                            .id("corner-bottom-left")
                            .absolute()
                            .bottom_0()
                            .left_0()
                            .size(px(14.))
                            .cursor(CursorStyle::ResizeUpLeftDownRight)
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.start_window_resize(ResizeEdge::BottomLeft);
                            })
                    )
                    .child(
                        div()
                            .id("corner-bottom-right")
                            .absolute()
                            .bottom_0()
                            .right_0()
                            .size(px(14.))
                            .cursor(CursorStyle::ResizeUpRightDownLeft)
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.start_window_resize(ResizeEdge::BottomRight);
                            })
                    )
            )
    }
}
