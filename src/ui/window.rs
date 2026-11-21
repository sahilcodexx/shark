//! Main application window layout and controller for Shark GPUI.

use crate::filesystem::DirectoryCache;
use crate::filesystem::operations::{open_in_terminal, open_path};
use crate::filesystem::scanner::{scan_directory_async, ScanMessage};
use crate::models::file_entry::FileEntry;
use crate::navigation::history::NavigationHistory;
use crate::theme::{self, button, theme_button, icon, icon_button, ButtonLook, RADIUS_CARD, RADIUS_ROW, RADIUS_THUMB, TITLE_BAR_HEIGHT};
use crate::thumbnails::ThumbnailCache;
use gpui::{
    actions, div, img, prelude::*, px, uniform_list, AnyElement, ClickEvent, Context, CursorStyle, Div, ElementId,
    FocusHandle, Hsla, IntoElement, KeyDownEvent, MouseButton, Render, ResizeEdge, Role, ScrollWheelEvent, SharedString,
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
    pub view_mode: ViewMode,
    pub sort_by: SortBy,
    pub sort_reversed: bool,
    pub sort_folders_first: bool,
    pub type_filter: TypeFilter,
    pub date_filter: DateFilter,
    pub size_filter: SizeFilter,
    pub show_filter_menu: bool,
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
            view_mode: ViewMode::Grid,
            sort_by: SortBy::Name,
            sort_reversed: false,
            sort_folders_first: true,
            type_filter: TypeFilter::All,
            date_filter: DateFilter::Any,
            size_filter: SizeFilter::Any,
            show_filter_menu: false,
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
        self.search_query.clear();
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

                let should_continue = this.update(cx, |win, _| {
                    if let Some(ref rx) = win.scan_receiver {
                        while let Ok(msg) = rx.try_recv() {
                            match msg {
                                ScanMessage::Batch(batch) => batches.push(batch),
                                ScanMessage::Completed { total_items, total_bytes } => {
                                    completed = Some((total_items, total_bytes));
                                }
                                ScanMessage::Error(_) => {}
                            }
                        }
                    }
                    !cancellable.load(Ordering::Relaxed)
                });

                if should_continue.is_err() || !should_continue.unwrap() {
                    break;
                }

                if !batches.is_empty() || completed.is_some() {
                    let _ = this.update(cx, |win, cx| {
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

                if completed.is_some() {
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
        let folder_name = self.current_path.file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string());

        let divider = || {
            div()
                .w(px(1.))
                .h(px(14.))
                .bg(theme.button_border)
        };

        div()
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
                    .child(
                        if self.path_editing {
                            div()
                                .id("path-editing-box")
                                .flex_1()
                                .min_w(px(280.))
                                .child(self.render_breadcrumb(theme, cx))
                        } else {
                            div()
                                .id("current-folder-title-pill")
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap_2()
                                .cursor_pointer()
                                .hover(|s| s.opacity(0.85))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.path_editing = true;
                                    this.path_input_text = this.current_path.to_string_lossy().to_string();
                                    cx.notify();
                                }))
                                .child(
                                    div()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(px(15.))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_color(theme.text_primary)
                                        .child(folder_name),
                                )
                                .child(
                                    theme::icon("chevron-down-hi", 11.)
                                        .flex_shrink_0()
                                        .text_color(theme.text_faint),
                                )
                        }
                    ),
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
                                    cx.notify();
                                })),
                            )
                            .child(divider())
                            .child(
                                theme::macos_joint_button(
                                    "joint-btn-more",
                                    "more-horizontal",
                                    self.show_command_palette,
                                    true,
                                    theme,
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.show_command_palette = true;
                                    this.command_palette_query.clear();
                                    this.command_palette_selected = 0;
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
                                this.show_command_palette = true;
                                this.command_palette_query.clear();
                                this.command_palette_selected = 0;
                                cx.notify();
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

    fn render_breadcrumb(&self, theme: &theme::ThemeColors, cx: &mut Context<Self>) -> Div {
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
                        .text_size(px(theme::TEXT_SMALL))
                        .text_color(theme.text_primary)
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(format!("{}|", self.path_input_text)),
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
                );
        }

        let mut segments = Vec::new();
        let mut curr = Some(self.current_path.as_path());

        while let Some(p) = curr {
            if let Some(file_name) = p.file_name() {
                segments.push((file_name.to_string_lossy().to_string(), p.to_path_buf()));
            } else if p == Path::new("/") {
                segments.push(("/".to_string(), p.to_path_buf()));
            }
            curr = p.parent();
        }
        segments.reverse();

        if segments.is_empty() {
            segments.push(("/".to_string(), PathBuf::from("/")));
        }

        let current_path = self.current_path.clone();

        div()
            .h(px(theme::BUTTON_HEIGHT))
            .px_2()
            .bg(theme::glass_button_fill())
            .rounded(px(theme::RADIUS_ROW))
            .border_1()
            .border_color(theme::glass_border())
            .flex()
            .items_center()
            .justify_between()
            .gap_1()
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .gap_1()
                    .overflow_hidden()
                    .children(segments.into_iter().enumerate().flat_map(|(i, (name, path))| {
                        let is_last = current_path == path;
                        let target = path.clone();

                        let pill = div()
                            .id(ElementId::from(format!("crumb-{}", i)))
                            .px_2()
                            .py_0p5()
                            .rounded(px(4.))
                            .cursor_pointer()
                            .when(!is_last, |d| {
                                d.hover(|s| s.bg(theme::hover_fill()))
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.navigate_to(target.clone(), cx);
                            }))
                            .child(
                                div()
                                    .text_size(px(theme::TEXT_SMALL))
                                    .text_color(if is_last { theme::accent() } else { theme::text_muted() })
                                    .font_weight(if is_last { gpui::FontWeight::SEMIBOLD } else { gpui::FontWeight::NORMAL })
                                    .child(name),
                            )
                            .into_any_element();

                        let chevron = if !is_last {
                            Some(
                                theme::icon("chevron-right", 10.)
                                    .text_color(theme::text_faint())
                                    .into_any_element(),
                            )
                        } else {
                            None
                        };

                        [Some(pill), chevron].into_iter().flatten()
                    })),
            )
            .child(
                theme::glass_icon_button("edit-path-btn", "pencil", false, true)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.path_editing = true;
                        this.search_active = false;
                        this.path_input_text = this.current_path.to_string_lossy().to_string();
                        cx.notify();
                    })),
            )
    }

    fn render_settings_modal(&self, theme: &theme::ThemeColors, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("settings-modal-backdrop")
            .absolute()
            .inset_0()
            .bg(if theme.is_dark { gpui::rgba(0x00000088) } else { gpui::rgba(0x00000040) })
            .flex()
            .items_center()
            .justify_center()
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(|this, _, _, cx| {
                this.show_settings_modal = false;
                cx.notify();
            }))
            .child(
                div()
                    .id("settings-modal-content")
                    .w(px(500.))
                    .bg(theme.card)
                    .border_1()
                    .border_color(theme.surface_border)
                    .shadow(theme::floating_panel_shadows(theme))
                    .rounded(px(RADIUS_CARD))
                    .p_5()
                    .flex()
                    .flex_col()
                    .gap_3p5()
                    .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                    .on_click(|_, _, cx| cx.stop_propagation())
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
                                    .child(icon("settings", 18.).text_color(theme.accent))
                                    .child(
                                        div()
                                            .text_size(px(theme::TEXT_HEADER))
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(theme.text_primary)
                                            .child("Settings & Preferences"),
                                    ),
                            )
                            .child(
                                icon_button("close-settings", "cancel-01", false, true)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.show_settings_modal = false;
                                        cx.notify();
                                    })),
                            ),
                    )
                    // Section 1: View & Layout
                    .child(
                        div()
                            .text_size(px(11.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.accent)
                            .child("VIEW & LAYOUT"),
                    )
                    // Window Material (Frosted Glass with Hardware Blur / Solid)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .py_1p5()
                            .border_b_1()
                            .border_color(theme.surface_border)
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(theme::TEXT_BODY))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(theme.text_primary)
                                            .child("Frosted Glass & Blur"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(theme::TEXT_CAPTION))
                                            .text_color(theme.text_faint)
                                            .child("Hardware Wayland blur (Niri ext-background-effect-v1)"),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_1p5()
                                    .child(
                                        theme_button(
                                            "bg-frosted",
                                            if self.background_mode != "solid" { ButtonLook::Primary } else { ButtonLook::Secondary },
                                            theme,
                                        )
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.background_mode = "frosted".to_string();
                                            if let Some(coord) = &mut this.blur_coordinator {
                                                coord.set_enabled(true);
                                            }
                                            this.save_config();
                                            cx.notify();
                                        }))
                                        .child("Frosted Blur"),
                                    )
                                    .child(
                                        theme_button(
                                            "bg-solid",
                                            if self.background_mode == "solid" { ButtonLook::Primary } else { ButtonLook::Secondary },
                                            theme,
                                        )
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.background_mode = "solid".to_string();
                                            if let Some(coord) = &mut this.blur_coordinator {
                                                coord.set_enabled(false);
                                            }
                                            this.save_config();
                                            cx.notify();
                                        }))
                                        .child("Solid"),
                                    ),
                            ),
                    )
                    // Option 1: Show Hidden Files
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .py_1p5()
                            .border_b_1()
                            .border_color(theme.surface_border)
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(theme::TEXT_BODY))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(theme.text_primary)
                                            .child("Show Hidden Files"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(theme::TEXT_CAPTION))
                                            .text_color(theme.text_faint)
                                            .child("Display dotfiles and hidden folders (Ctrl+H)"),
                                    ),
                            )
                            .child(
                                theme_button(
                                    "toggle-hidden-btn",
                                    if self.show_hidden { ButtonLook::Primary } else { ButtonLook::Secondary },
                                    theme,
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.show_hidden = !this.show_hidden;
                                    this.sort_and_filter();
                                    this.save_config();
                                    cx.notify();
                                }))
                                .child(if self.show_hidden { "ON" } else { "OFF" }),
                            ),
                    )
                    // Option 2: Sort Folders First
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .py_1p5()
                            .border_b_1()
                            .border_color(theme.surface_border)
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(theme::TEXT_BODY))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(theme.text_primary)
                                            .child("Sort Folders First"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(theme::TEXT_CAPTION))
                                            .text_color(theme.text_faint)
                                            .child("Keep directories grouped above files"),
                                    ),
                            )
                            .child(
                                theme_button(
                                    "toggle-folders-first-btn",
                                    if self.sort_folders_first { ButtonLook::Primary } else { ButtonLook::Secondary },
                                    theme,
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.sort_folders_first = !this.sort_folders_first;
                                    this.sort_and_filter();
                                    this.save_config();
                                    cx.notify();
                                }))
                                .child(if self.sort_folders_first { "ON" } else { "OFF" }),
                            ),
                    )
                    // Option 3: View Mode & Card Zoom
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .py_1p5()
                            .border_b_1()
                            .border_color(theme.surface_border)
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(theme::TEXT_BODY))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(theme.text_primary)
                                            .child("Grid Icon Size"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(theme::TEXT_CAPTION))
                                            .text_color(theme.text_faint)
                                            .child("Zoom level for grid thumbnails"),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_1p5()
                                    .child(
                                        theme_button("zoom-sm", if self.zoom <= 95. { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.zoom = 80.; cx.notify(); }))
                                            .child("Small"),
                                    )
                                    .child(
                                        theme_button("zoom-med", if self.zoom > 95. && self.zoom <= 140. { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.zoom = 120.; cx.notify(); }))
                                            .child("Medium"),
                                    )
                                    .child(
                                        theme_button("zoom-lg", if self.zoom > 140. && self.zoom <= 200. { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.zoom = 180.; cx.notify(); }))
                                            .child("Large"),
                                    )
                                    .child(
                                        theme_button("zoom-xl", if self.zoom > 200. { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.zoom = 250.; cx.notify(); }))
                                            .child("Huge"),
                                    ),
                            ),
                    )
                    // Option 4: Font Size (SF Pro text scaling)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .py_1p5()
                            .border_b_1()
                            .border_color(theme.surface_border)
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(theme::TEXT_BODY))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(theme.text_primary)
                                            .child("Interface Font Size"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(theme::TEXT_CAPTION))
                                            .text_color(theme.text_faint)
                                            .child("System typography scale (Apple SF Pro)"),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_1p5()
                                    .child(
                                        theme_button("font-sm", if (self.font_size - 12.).abs() < 0.5 { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.font_size = 12.; this.save_config(); cx.notify(); }))
                                            .child("12px"),
                                    )
                                    .child(
                                        theme_button("font-def", if (self.font_size - 14.).abs() < 0.5 { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.font_size = 14.; this.save_config(); cx.notify(); }))
                                            .child("14px"),
                                    )
                                    .child(
                                        theme_button("font-med", if (self.font_size - 15.).abs() < 0.5 { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.font_size = 15.; this.save_config(); cx.notify(); }))
                                            .child("15px"),
                                    )
                                    .child(
                                        theme_button("font-lg", if (self.font_size - 16.).abs() < 0.5 { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.font_size = 16.; this.save_config(); cx.notify(); }))
                                            .child("16px"),
                                    ),
                            ),
                    )
                    // Section 2: Thumbnails & Performance
                    .child(
                        div()
                            .text_size(px(11.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.accent)
                            .child("PERFORMANCE & MEDIA"),
                    )
                    // Option 5: Thumbnails
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .py_1p5()
                            .border_b_1()
                            .border_color(theme.surface_border)
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(theme::TEXT_BODY))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(theme.text_primary)
                                            .child("Thumbnail Previews"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(theme::TEXT_CAPTION))
                                            .text_color(theme.text_faint)
                                            .child("Render image and video thumbnails"),
                                    ),
                            )
                            .child(
                                theme_button(
                                    "toggle-thumbs-btn",
                                    if self.enable_thumbnails { ButtonLook::Primary } else { ButtonLook::Secondary },
                                    theme,
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.enable_thumbnails = !this.enable_thumbnails;
                                    cx.notify();
                                }))
                                .child(if self.enable_thumbnails { "ON" } else { "OFF" }),
                            ),
                    )
                    // Option 6: Terminal
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .py_1p5()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(theme::TEXT_BODY))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(theme.text_primary)
                                            .child("External Terminal"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(theme::TEXT_CAPTION))
                                            .text_color(theme.text_faint)
                                            .child("Launch shell at current directory"),
                                    ),
                            )
                            .child(
                                theme_button("settings-open-term", ButtonLook::Secondary, theme)
                                    .on_click(cx.listener(|this, _, _, _| {
                                        open_in_terminal(&this.current_path);
                                    }))
                                    .child("Open Terminal"),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn render_filter_popover(&self, theme: &theme::ThemeColors, cx: &mut Context<Self>) -> AnyElement {
        let is_any_filter_active = self.has_active_filters();
        let total_count = self.entries.len();
        let filtered_count = self.filtered_indices.len();

        div()
            .id("filter-popover-backdrop")
            .absolute()
            .inset_0()
            .bg(if theme.is_dark { gpui::rgba(0x00000077) } else { gpui::rgba(0x00000030) })
            .flex()
            .items_start()
            .justify_end()
            .pt(px(46.))
            .pr(px(16.))
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(|this, _, _, cx| {
                this.show_filter_menu = false;
                cx.notify();
            }))
            .child(
                div()
                    .id("filter-popover-card")
                    .w(px(350.))
                    .bg(theme.card)
                    .border_1()
                    .border_color(theme.surface_border)
                    .shadow(theme::floating_panel_shadows(theme))
                    .rounded(px(12.))
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                    .on_click(|_, _, cx| cx.stop_propagation())
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
                                    .child(icon("filter", 16.).text_color(theme.accent))
                                    .child(
                                        div()
                                            .text_size(px(14.))
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
                                    .text_size(px(11.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.text_faint)
                                    .child("SORT BY"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_1p5()
                                    .child(
                                        theme_button("sort-name", if self.sort_by == SortBy::Name { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
                                            .on_click(cx.listener(|this, _, _, cx| this.toggle_sort(SortBy::Name, cx)))
                                            .child("Name"),
                                    )
                                    .child(
                                        theme_button("sort-mod", if self.sort_by == SortBy::Modified { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
                                            .on_click(cx.listener(|this, _, _, cx| this.toggle_sort(SortBy::Modified, cx)))
                                            .child("Date"),
                                    )
                                    .child(
                                        theme_button("sort-size", if self.sort_by == SortBy::Size { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
                                            .on_click(cx.listener(|this, _, _, cx| this.toggle_sort(SortBy::Size, cx)))
                                            .child("Size"),
                                    )
                                    .child(
                                        theme_button("sort-type", if self.sort_by == SortBy::Type { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
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
                                            .text_size(px(12.))
                                            .text_color(theme.text_muted)
                                            .child("Direction:"),
                                    )
                                    .child(
                                        theme_button("sort-direction", ButtonLook::Secondary, theme)
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
                                    .text_size(px(11.))
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
                                        theme_button("f-type-all", if self.type_filter == TypeFilter::All { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.type_filter = TypeFilter::All; this.sort_and_filter(); cx.notify(); }))
                                            .child("All"),
                                    )
                                    .child(
                                        theme_button("f-type-folders", if self.type_filter == TypeFilter::Folders { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.type_filter = TypeFilter::Folders; this.sort_and_filter(); cx.notify(); }))
                                            .child("Folders"),
                                    )
                                    .child(
                                        theme_button("f-type-docs", if self.type_filter == TypeFilter::Documents { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.type_filter = TypeFilter::Documents; this.sort_and_filter(); cx.notify(); }))
                                            .child("Docs"),
                                    )
                                    .child(
                                        theme_button("f-type-images", if self.type_filter == TypeFilter::Images { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.type_filter = TypeFilter::Images; this.sort_and_filter(); cx.notify(); }))
                                            .child("Images"),
                                    )
                                    .child(
                                        theme_button("f-type-media", if self.type_filter == TypeFilter::Media { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.type_filter = TypeFilter::Media; this.sort_and_filter(); cx.notify(); }))
                                            .child("Media"),
                                    )
                                    .child(
                                        theme_button("f-type-code", if self.type_filter == TypeFilter::Code { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.type_filter = TypeFilter::Code; this.sort_and_filter(); cx.notify(); }))
                                            .child("Code"),
                                    )
                                    .child(
                                        theme_button("f-type-archives", if self.type_filter == TypeFilter::Archives { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
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
                                    .text_size(px(11.))
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
                                        theme_button("f-date-any", if self.date_filter == DateFilter::Any { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.date_filter = DateFilter::Any; this.sort_and_filter(); cx.notify(); }))
                                            .child("Any time"),
                                    )
                                    .child(
                                        theme_button("f-date-today", if self.date_filter == DateFilter::Today { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.date_filter = DateFilter::Today; this.sort_and_filter(); cx.notify(); }))
                                            .child("Today"),
                                    )
                                    .child(
                                        theme_button("f-date-7d", if self.date_filter == DateFilter::Last7Days { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.date_filter = DateFilter::Last7Days; this.sort_and_filter(); cx.notify(); }))
                                            .child("Past 7 Days"),
                                    )
                                    .child(
                                        theme_button("f-date-30d", if self.date_filter == DateFilter::Last30Days { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
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
                                    .text_size(px(11.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.text_faint)
                                    .child("FILE SIZE"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_1p5()
                                    .child(
                                        theme_button("f-size-any", if self.size_filter == SizeFilter::Any { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.size_filter = SizeFilter::Any; this.sort_and_filter(); cx.notify(); }))
                                            .child("Any"),
                                    )
                                    .child(
                                        theme_button("f-size-small", if self.size_filter == SizeFilter::Small { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.size_filter = SizeFilter::Small; this.sort_and_filter(); cx.notify(); }))
                                            .child("< 1 MB"),
                                    )
                                    .child(
                                        theme_button("f-size-med", if self.size_filter == SizeFilter::Medium { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
                                            .on_click(cx.listener(|this, _, _, cx| { this.size_filter = SizeFilter::Medium; this.sort_and_filter(); cx.notify(); }))
                                            .child("1-100 MB"),
                                    )
                                    .child(
                                        theme_button("f-size-large", if self.size_filter == SizeFilter::Large { ButtonLook::Primary } else { ButtonLook::Secondary }, theme)
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
            nav.push(PaletteItem {
                section: "Navigation",
                title: "Home".to_string(),
                subtitle: None,
                icon_name: "home",
                shortcut: Some("H"),
                action: PaletteAction::Navigate(p),
            });
        }
        if let Some(p) = dirs::document_dir() {
            nav.push(PaletteItem {
                section: "Navigation",
                title: "Work".to_string(),
                subtitle: None,
                icon_name: "user",
                shortcut: Some("W"),
                action: PaletteAction::Navigate(p),
            });
        }
        let doc_dir = dirs::document_dir().or_else(|| dirs::home_dir().map(|h| h.join("Documents")));
        if let Some(p) = &doc_dir {
            nav.push(PaletteItem {
                section: "Navigation",
                title: "Projects".to_string(),
                subtitle: None,
                icon_name: "layers",
                shortcut: Some("P"),
                action: PaletteAction::Navigate(p.clone()),
            });
        }
        if let Some(p) = dirs::download_dir() {
            nav.push(PaletteItem {
                section: "Navigation",
                title: "Downloads".to_string(),
                subtitle: None,
                icon_name: "download",
                shortcut: Some("D"),
                action: PaletteAction::Navigate(p),
            });
        }
        if let Some(p) = dirs::home_dir() {
            nav.push(PaletteItem {
                section: "Navigation",
                title: "Bookmarks".to_string(),
                subtitle: None,
                icon_name: "bookmark",
                shortcut: Some("M"),
                action: PaletteAction::Navigate(p),
            });
        }
        if let Some(p) = dirs::desktop_dir() {
            nav.push(PaletteItem {
                section: "Navigation",
                title: "Desktop".to_string(),
                subtitle: None,
                icon_name: "app-window",
                shortcut: Some("K"),
                action: PaletteAction::Navigate(p),
            });
        }
        if let Some(p) = dirs::picture_dir() {
            nav.push(PaletteItem {
                section: "Navigation",
                title: "Pictures".to_string(),
                subtitle: None,
                icon_name: "image",
                shortcut: Some("I"),
                action: PaletteAction::Navigate(p),
            });
        }
        if let Some(p) = dirs::video_dir() {
            nav.push(PaletteItem {
                section: "Navigation",
                title: "Videos".to_string(),
                subtitle: None,
                icon_name: "video",
                shortcut: Some("V"),
                action: PaletteAction::Navigate(p),
            });
        }
        if let Some(p) = dirs::audio_dir() {
            nav.push(PaletteItem {
                section: "Navigation",
                title: "Music".to_string(),
                subtitle: None,
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
                    projects.push(PaletteItem {
                        section: "Projects",
                        title: name.to_string(),
                        subtitle: None,
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
                                projects.push(PaletteItem {
                                    section: "Projects",
                                    title,
                                    subtitle: None,
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

        let actions = vec![
            PaletteItem {
                section: "Commands",
                title: "Create New Folder".to_string(),
                subtitle: Some("Create a new directory".to_string()),
                icon_name: "add-01",
                shortcut: Some("Ctrl+N"),
                action: PaletteAction::NewFolder,
            },
            PaletteItem {
                section: "Commands",
                title: if self.view_mode == ViewMode::Grid { "Switch to List View".to_string() } else { "Switch to Grid View".to_string() },
                subtitle: Some("Toggle display layout".to_string()),
                icon_name: if self.view_mode == ViewMode::Grid { "list" } else { "grid" },
                shortcut: Some("Ctrl+V"),
                action: PaletteAction::ToggleViewMode,
            },
            PaletteItem {
                section: "Commands",
                title: if self.show_hidden { "Hide Hidden Files".to_string() } else { "Show Hidden Files".to_string() },
                subtitle: Some("Toggle dotfiles visibility".to_string()),
                icon_name: "eye",
                shortcut: Some("Ctrl+H"),
                action: PaletteAction::ToggleHidden,
            },
            PaletteItem {
                section: "Commands",
                title: "Toggle Properties Inspector".to_string(),
                subtitle: Some("Show or hide sidebar properties".to_string()),
                icon_name: "sliders",
                shortcut: Some("Ctrl+I"),
                action: PaletteAction::ToggleInspector,
            },
            PaletteItem {
                section: "Commands",
                title: "Open Folder in Terminal".to_string(),
                subtitle: Some("Launch terminal at current path".to_string()),
                icon_name: "terminal",
                shortcut: Some("Ctrl+T"),
                action: PaletteAction::OpenTerminal,
            },
            PaletteItem {
                section: "Commands",
                title: "Settings & Preferences".to_string(),
                subtitle: Some("Configure Shark file manager".to_string()),
                icon_name: "settings",
                shortcut: Some("Ctrl+,"),
                action: PaletteAction::OpenSettings,
            },
            PaletteItem {
                section: "Commands",
                title: "Refresh Directory".to_string(),
                subtitle: Some("Reload folder from disk".to_string()),
                icon_name: "rotate-cw",
                shortcut: Some("Ctrl+R"),
                action: PaletteAction::Refresh,
            },
            PaletteItem {
                section: "Commands",
                title: "Theme: System (Auto)".to_string(),
                subtitle: Some("Follow OS light/dark appearance".to_string()),
                icon_name: "sliders",
                shortcut: None,
                action: PaletteAction::SetTheme(theme::ThemeMode::System),
            },
            PaletteItem {
                section: "Commands",
                title: "Theme: Light Mode".to_string(),
                subtitle: Some("Switch to clean macOS light mode".to_string()),
                icon_name: "palette",
                shortcut: None,
                action: PaletteAction::SetTheme(theme::ThemeMode::Light),
            },
            PaletteItem {
                section: "Commands",
                title: "Theme: Dark Mode".to_string(),
                subtitle: Some("Switch to solid macOS dark mode".to_string()),
                icon_name: "palette",
                shortcut: None,
                action: PaletteAction::SetTheme(theme::ThemeMode::Dark),
            },
        ];

        let filters = vec![
            PaletteItem {
                section: "Filters",
                title: "Filter: Only Images".to_string(),
                subtitle: Some("Show PNG, JPG, WebP, SVG files".to_string()),
                icon_name: "image",
                shortcut: None,
                action: PaletteAction::SetTypeFilter(TypeFilter::Images),
            },
            PaletteItem {
                section: "Filters",
                title: "Filter: Only Documents & Text".to_string(),
                subtitle: Some("Show TXT, MD, PDF, CSV documents".to_string()),
                icon_name: "document",
                shortcut: None,
                action: PaletteAction::SetTypeFilter(TypeFilter::Documents),
            },
            PaletteItem {
                section: "Filters",
                title: "Filter: Only Code & Scripts".to_string(),
                subtitle: Some("Show RS, PY, JS, HTML, JSON files".to_string()),
                icon_name: "code",
                shortcut: None,
                action: PaletteAction::SetTypeFilter(TypeFilter::Code),
            },
            PaletteItem {
                section: "Filters",
                title: "Filter: Modified Past 7 Days".to_string(),
                subtitle: Some("Show files changed this week".to_string()),
                icon_name: "clock",
                shortcut: None,
                action: PaletteAction::SetDateFilter(DateFilter::Last7Days),
            },
            PaletteItem {
                section: "Filters",
                title: "Filter: Large Files (> 100 MB)".to_string(),
                subtitle: Some("Show files larger than 100 MB".to_string()),
                icon_name: "hard-drive",
                shortcut: None,
                action: PaletteAction::SetSizeFilter(SizeFilter::Large),
            },
            PaletteItem {
                section: "Filters",
                title: "Clear All Active Filters".to_string(),
                subtitle: Some("Reset to show all directory contents".to_string()),
                icon_name: "rotate-ccw",
                shortcut: None,
                action: PaletteAction::ResetFilters,
            },
        ];

        for item in nav {
            if q.is_empty() || item.title.to_lowercase().contains(&q) {
                items.push(item);
            }
        }
        for item in projects {
            if q.is_empty() || item.title.to_lowercase().contains(&q) {
                items.push(item);
            }
        }
        for item in actions {
            if q.is_empty() || item.title.to_lowercase().contains(&q) {
                items.push(item);
            }
        }
        for item in filters {
            if q.is_empty() || item.title.to_lowercase().contains(&q) {
                items.push(item);
            }
        }

        if !q.is_empty() {
            let mut file_matches = 0;
            for entry in &self.entries {
                if file_matches >= 20 {
                    break;
                }
                if entry.name_lower.contains(&q) {
                    file_matches += 1;
                    items.push(PaletteItem {
                        section: "Files",
                        title: entry.name.clone(),
                        subtitle: Some(format!("{} • {}", entry.type_description(), entry.formatted_size())),
                        icon_name: if entry.is_directory { "folder" } else { entry.icon_name },
                        shortcut: None,
                        action: if entry.is_directory {
                            PaletteAction::Navigate(entry.path.clone())
                        } else {
                            PaletteAction::OpenFile(entry.path.clone())
                        },
                    });
                }
            }
        }

        items
    }

    pub fn execute_palette_action(&mut self, action: PaletteAction, cx: &mut Context<Self>) {
        self.show_command_palette = false;
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

        for (idx, item) in items.iter().enumerate() {
            if item.section != last_section {
                if !last_section.is_empty() {
                    rendered_items.push(
                        div()
                            .h(px(1.))
                            .mx_2()
                            .my_1p5()
                            .bg(theme.surface_border)
                            .into_any_element(),
                    );
                }
                last_section = item.section;
                rendered_items.push(
                    div()
                        .px_3()
                        .pt_1p5()
                        .pb_1()
                        .text_size(px(11.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.text_faint)
                        .child(item.section)
                        .into_any_element(),
                );
            }

            let is_selected = idx == selected_idx;
            let action = item.action.clone();
            let title = item.title.clone();
            let subtitle = item.subtitle.clone();
            let icon_name = item.icon_name;
            let shortcut = item.shortcut;

            let row = div()
                .id(ElementId::from(format!("palette-item-{}", idx)))
                .h(px(36.))
                .px_3()
                .rounded(px(8.))
                .flex()
                .items_center()
                .justify_between()
                .cursor_pointer()
                .when(is_selected, |d| {
                    d.bg(theme.sidebar_selected)
                })
                .when(!is_selected, |d| {
                    d.hover(|s| s.bg(theme.sidebar_hover))
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.execute_palette_action(action.clone(), cx);
                }))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(
                            icon(icon_name, 16.)
                                .text_color(if is_selected { theme.text_primary } else { theme.text_muted })
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .text_color(theme.text_primary)
                                        .font_weight(if is_selected { gpui::FontWeight::MEDIUM } else { gpui::FontWeight::NORMAL })
                                        .child(title),
                                )
                                .when_some(subtitle, |d, sub| {
                                    d.child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(theme.text_faint)
                                            .child(format!("• {}", sub)),
                                    )
                                }),
                        ),
                )
                .when_some(shortcut, |d, sc| {
                    if sc.len() == 1 {
                        d.child(
                            div()
                                .text_size(px(12.))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.text_faint)
                                .child(sc),
                        )
                    } else {
                        d.child(
                            div()
                                .px_1p5()
                                .py_0p5()
                                .rounded(px(4.))
                                .bg(theme.button_fill)
                                .border_1()
                                .border_color(theme.button_border)
                                .text_size(px(10.))
                                .text_color(theme.text_muted)
                                .child(sc),
                        )
                    }
                })
                .into_any_element();

            rendered_items.push(row);
        }

        let user_label = std::env::var("USER").unwrap_or_else(|_| "Sahilcodex".to_string());
        let capitalized_user = if user_label == "sahilcodex" {
            "Sahilcodex".to_string()
        } else {
            let mut c = user_label.chars();
            match c.next() {
                None => "Shark".to_string(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        };

        let action_verb = if let Some(item) = items.get(selected_idx) {
            match item.section {
                "Navigation" | "Projects" => "Go to page",
                "Commands" => "Run command",
                "Filters" => "Apply filter",
                _ => "Open file",
            }
        } else {
            "Go to page"
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
                    .bg(if theme.is_dark { gpui::rgba(0x0e0e11fa) } else { gpui::rgba(0xf4f4f7ee) })
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
                                    .h(px(180.))
                                    .w_full()
                                    .rounded(px(10.))
                                    .bg(if theme.is_dark { gpui::rgba(0x00000030) } else { gpui::rgba(0x00000006) })
                                    .border_1()
                                    .border_color(theme.surface_border)
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .justify_center()
                                    .gap_2()
                                    .overflow_hidden()
                                    .child(match thumb_path {
                                        Some(p) => img(p)
                                            .max_h(px(170.))
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
                                    .p_3()
                                    .rounded(px(8.))
                                    .bg(if theme.is_dark { gpui::rgba(0xffffff06) } else { gpui::rgba(0x00000004) })
                                    .border_1()
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
                    .bg(if theme.is_dark { gpui::rgba(0x0e0e11fa) } else { gpui::rgba(0xf4f4f7ee) })
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
                .bg(if theme.is_dark { gpui::rgba(0x0e0e11fa) } else { gpui::rgba(0xf4f4f7ee) })
                .p_4()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(12.))
                .text_color(theme.text_faint)
                .child("Select an item to preview")
        };

        div()
            .id("command-palette-backdrop")
            .absolute()
            .inset_0()
            .bg(if theme.is_dark { gpui::rgba(0x000000a6) } else { gpui::rgba(0x00000045) })
            .flex()
            .items_start()
            .justify_center()
            .pt(px(70.))
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(|this, _, _, cx| {
                this.show_command_palette = false;
                this.command_palette_query.clear();
                cx.notify();
            }))
            .child(
                div()
                    .id("command-palette-card")
                    .w(px(780.))
                    .max_h(px(480.))
                    .bg(theme.card)
                    .border_1()
                    .border_color(theme.surface_border)
                    .shadow(theme::floating_panel_shadows(theme))
                    .rounded(px(14.))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                    .on_click(|_, _, cx| cx.stop_propagation())
                    // Header Input Row
                    .child(
                        div()
                            .h(px(48.))
                            .px_4()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child(icon("search", 16.).text_color(theme.text_faint))
                                    .child(
                                        div()
                                            .text_size(px(14.))
                                            .text_color(if self.command_palette_query.is_empty() {
                                                theme.text_faint
                                            } else {
                                                theme.text_primary
                                            })
                                            .child(if self.command_palette_query.is_empty() {
                                                "Type a command or search...".to_string()
                                            } else {
                                                format!("{}|", self.command_palette_query)
                                            }),
                                    ),
                            )
                            .child(
                                icon_button("close-palette", "cancel-01", false, true)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.show_command_palette = false;
                                        this.command_palette_query.clear();
                                        cx.notify();
                                    })),
                            ),
                    )
                    // Divider
                    .child(div().h(px(1.)).w_full().bg(theme.surface_border))
                    // Split Body: Left List + Right Preview
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .overflow_hidden()
                            // Left list & footer
                            .child(
                                div()
                                    .w(px(469.))
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .id("palette-items-list")
                                            .flex_1()
                                            .overflow_y_scroll()
                                            .max_h(px(340.))
                                            .p_2()
                                            .flex()
                                            .flex_col()
                                            .gap_0p5()
                                            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                                            .children(rendered_items),
                                    )
                                    // Footer
                                    .child(
                                        div()
                                            .h(px(34.))
                                            .px_4()
                                            .bg(if theme.is_dark { gpui::rgba(0x101012fa) } else { gpui::rgba(0xf0f0f4ee) })
                                            .border_t_1()
                                            .border_color(theme.surface_border)
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .text_size(px(11.))
                                            .text_color(theme.text_faint)
                                            .child(
                                                div()
                                                    .child(capitalized_user),
                                            )
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
                                                            .child("Exit")
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
                                     ),
                            )
                            // Vertical Divider
                            .child(div().w(px(1.)).h_full().bg(theme.surface_border))
                            // Right Preview Pane
                            .child(preview_pane),
                    ),
            )
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
        let favorites = vec![
            ("place-recents", "Recents", "clock", home.clone()),
            ("place-applications", "Applications", "app-window", PathBuf::from("/usr/share/applications")),
            ("place-desktop", "Desktop", "app-window", home.join("Desktop")),
            ("place-documents", "Documents", "document", home.join("Documents")),
            ("place-downloads", "Downloads", "download", home.join("Downloads")),
            ("place-music", "Music", "audio", home.join("Music")),
            ("place-pictures", "Pictures", "image", home.join("Pictures")),
            ("place-videos", "Videos", "video", home.join("Videos")),
        ];

        let mut locations = vec![
            ("loc-cloud", "iCloud Drive", "cloud", home.join("Documents")),
            ("loc-user", "User Home", "user", home.clone()),
            ("loc-root", "Macintosh HD", "hard-drive", PathBuf::from("/")),
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
            .border_r_1()
            .border_color(theme.sidebar_border)
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
                    .child(self.render_sidebar_section("Favorites", favorites, theme, cx))
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
                    .text_color(theme.text_faint)
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
                    .text_color(theme.text_faint)
                    .child(title),
            )
            .children(tags.into_iter().map(|(id, label, color)| {
                div()
                    .id(ElementId::from(id))
                    .h(px(theme::SIDEBAR_ROW_HEIGHT))
                    .px_2()
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .gap_2()
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.sidebar_hover))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.search_query = label.to_lowercase();
                        this.sort_and_filter();
                        cx.notify();
                    }))
                    .child(
                        div()
                            .size(px(10.))
                            .rounded_full()
                            .bg(color)
                            .border_1()
                            .border_color(gpui::rgba(0x00000022)),
                    )
                    .child(
                        div()
                            .text_size(px(theme::TEXT_BODY))
                            .text_color(theme.text_muted)
                            .child(label),
                    )
            }))
            .child(
                div()
                    .id("tag-all")
                    .h(px(theme::SIDEBAR_ROW_HEIGHT))
                    .px_2()
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .gap_2()
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.sidebar_hover))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.search_query.clear();
                        this.sort_and_filter();
                        cx.notify();
                    }))
                    .child(
                        div()
                            .size(px(10.))
                            .rounded_full()
                            .border_1()
                            .border_color(theme.text_faint),
                    )
                    .child(
                        div()
                            .text_size(px(theme::TEXT_BODY))
                            .text_color(theme.text_faint)
                            .child("All Tags..."),
                    ),
            )
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
                .child("This folder is empty")
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
                            this.show_command_palette = false;
                            this.command_palette_query.clear();
                            cx.notify();
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
                            this.command_palette_query.pop();
                            this.command_palette_selected = 0;
                            cx.notify();
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
                                if let Some(ch) = &ev.keystroke.key_char {
                                    this.command_palette_query.push_str(ch);
                                    this.command_palette_selected = 0;
                                    cx.notify();
                                } else if key.chars().count() == 1 {
                                    this.command_palette_query.push_str(key);
                                    this.command_palette_selected = 0;
                                    cx.notify();
                                }
                            }
                        }
                    }
                    return;
                }

                // Global Shortcuts
                if modifiers.control && (key == "k" || key == "p" || key == "f") || (key == "/" && !this.path_editing) {
                    this.show_command_palette = true;
                    this.command_palette_query.clear();
                    this.command_palette_selected = 0;
                    cx.notify();
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
                        this.show_command_palette = false;
                        this.search_query.clear();
                        this.sort_and_filter();
                        cx.notify();
                    }
                    _ => {
                        // Type-to-search anywhere in the window
                        if !modifiers.control && !modifiers.platform && !modifiers.alt {
                            if let Some(ch) = &ev.keystroke.key_char {
                                if !ch.trim().is_empty() {
                                    this.show_command_palette = true;
                                    this.command_palette_query.push_str(ch);
                                    this.command_palette_selected = 0;
                                    cx.notify();
                                }
                            } else if key.chars().count() == 1 && !key.trim().is_empty() {
                                this.show_command_palette = true;
                                this.command_palette_query.push_str(key);
                                this.command_palette_selected = 0;
                                cx.notify();
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
