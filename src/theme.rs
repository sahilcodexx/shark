//! Appearance tokens and styling utilities for Shark GPUI file manager.

use gpui::{
    div, px, rgb, svg, Div, ElementId, FontWeight, Hsla, Role, Stateful, Svg, Window, prelude::*,
};
use serde::{Deserialize, Serialize};

pub const RADIUS_THUMB: f32 = 6.;
pub const RADIUS_ROW: f32 = 8.;
pub const RADIUS_CARD: f32 = 12.;
pub const RADIUS_PILL: f32 = 16.;

pub const TEXT_CAPTION: f32 = 11.;
pub const TEXT_SMALL: f32 = 12.;
pub const TEXT_BODY: f32 = 13.;
pub const TEXT_TITLE: f32 = 14.;
pub const TEXT_HEADER: f32 = 16.;

pub const TITLE_BAR_HEIGHT: f32 = 50.;
pub const ROW_HEIGHT: f32 = 32.;
pub const BUTTON_HEIGHT: f32 = 28.;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug)]
pub struct ThemeColors {
    pub is_dark: bool,
    pub canvas: Hsla,
    pub pane: Hsla,
    pub card: Hsla,
    pub card_border: Hsla,
    pub card_hover: Hsla,
    pub sidebar: Hsla,
    pub sidebar_border: Hsla,
    pub topbar: Hsla,
    pub topbar_border: Hsla,
    pub text_primary: Hsla,
    pub text_muted: Hsla,
    pub text_faint: Hsla,
    pub accent: Hsla,
    pub selection: Hsla,
    pub button_fill: Hsla,
    pub button_border: Hsla,
    pub button_hover: Hsla,
    pub search_fill: Hsla,
    pub search_border: Hsla,
}

impl ThemeColors {
    pub fn light() -> Self {
        Self {
            is_dark: false,
            canvas: rgb(0xf2f2f5).into(),
            pane: rgb(0xffffff).into(), // Pure crisp white for main file browser
            card: rgb(0xffffff).into(),
            card_border: rgb(0xe5e5ea).into(),
            card_hover: rgb(0xf3f4f6).into(),
            sidebar: rgb(0xf0f0f3).into(), // macOS light gray floating sidebar
            sidebar_border: rgb(0xdcdce2).into(),
            topbar: rgb(0xf6f6f8).into(),
            topbar_border: rgb(0xe5e5ea).into(),
            text_primary: rgb(0x1d1d1f).into(),
            text_muted: rgb(0x5c5c62).into(),
            text_faint: rgb(0x8e8e93).into(),
            accent: rgb(0x007aff).into(), // Apple system blue
            selection: gpui::rgba(0x007aff20).into(),
            button_fill: rgb(0xffffff).into(),
            button_border: rgb(0xdcdce2).into(),
            button_hover: rgb(0xf0f0f4).into(),
            search_fill: rgb(0xe8e8ed).into(),
            search_border: rgb(0xd8d8de).into(),
        }
    }

    pub fn dark() -> Self {
        Self {
            is_dark: true,
            canvas: rgb(0x141418).into(),
            pane: rgb(0x1c1c22).into(), // Solid rich dark background
            card: rgb(0x24242c).into(),
            card_border: rgb(0x31313c).into(),
            card_hover: rgb(0x2a2a34).into(),
            sidebar: rgb(0x18181e).into(), // Solid dark sidebar
            sidebar_border: rgb(0x282832).into(),
            topbar: rgb(0x22222a).into(),
            topbar_border: rgb(0x2e2e3a).into(),
            text_primary: rgb(0xf5f5f7).into(),
            text_muted: rgb(0xa1a1a6).into(),
            text_faint: rgb(0x6e6e73).into(),
            accent: rgb(0x0a84ff).into(),
            selection: gpui::rgba(0x0a84ff35).into(),
            button_fill: rgb(0x282832).into(),
            button_border: rgb(0x383844).into(),
            button_hover: rgb(0x32323e).into(),
            search_fill: rgb(0x282832).into(),
            search_border: rgb(0x383844).into(),
        }
    }

    pub fn resolve(mode: ThemeMode, window: &Window) -> Self {
        let is_dark = match mode {
            ThemeMode::Light => false,
            ThemeMode::Dark => true,
            ThemeMode::System => matches!(
                window.appearance(),
                gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
            ),
        };
        if is_dark {
            Self::dark()
        } else {
            Self::light()
        }
    }
}

// Fallback tokens for static calls
pub fn shell_fill() -> Hsla {
    rgb(0x141418).into()
}

pub fn pane_fill() -> Hsla {
    rgb(0x1c1c22).into()
}

pub fn card_fill() -> Hsla {
    rgb(0x24242c).into()
}

pub fn sidebar_fill() -> Hsla {
    rgb(0x18181e).into()
}

pub fn glass_panel_fill() -> Hsla {
    rgb(0x22222a).into()
}

pub fn glass_sidebar_fill() -> Hsla {
    rgb(0x18181e).into()
}

pub fn glass_button_fill() -> Hsla {
    gpui::rgba(0xffffff15).into()
}

pub fn glass_button_hover() -> Hsla {
    gpui::rgba(0xffffff28).into()
}

pub fn glass_button_active() -> Hsla {
    gpui::rgba(0xffffff3d).into()
}

pub fn glass_border() -> Hsla {
    gpui::rgba(0xffffff22).into()
}

pub fn glass_border_subtle() -> Hsla {
    gpui::rgba(0xffffff10).into()
}

pub fn glass_border_highlight() -> Hsla {
    gpui::rgba(0xffffff42).into()
}

pub fn glass_selection() -> Hsla {
    gpui::rgba(0x0a84ff35).into()
}

pub fn mac_close() -> Hsla {
    rgb(0xff5f56).into()
}

pub fn mac_minimize() -> Hsla {
    rgb(0xffbd2e).into()
}

pub fn mac_zoom() -> Hsla {
    rgb(0x27c93f).into()
}

pub fn border_color() -> Hsla {
    rgb(0x2e2e3a).into()
}

pub fn border_subtle() -> Hsla {
    rgb(0x282832).into()
}

pub fn accent() -> Hsla {
    rgb(0x38bdf8).into()
}

pub fn accent_hover() -> Hsla {
    rgb(0x0284c7).into()
}

pub fn on_accent() -> Hsla {
    rgb(0xffffff).into()
}

pub fn selection_fill() -> Hsla {
    gpui::hsla(204. / 360., 0.9, 0.45, 0.35)
}

pub fn hover_fill() -> Hsla {
    gpui::hsla(215. / 360., 0.2, 0.25, 0.4)
}

pub fn danger() -> Hsla {
    rgb(0xf43f5e).into()
}

pub fn success() -> Hsla {
    rgb(0x10b981).into()
}

pub fn warning() -> Hsla {
    rgb(0xf59e0b).into()
}

pub fn content(alpha: f32) -> Hsla {
    gpui::hsla(215. / 360., 0.15, 0.96, alpha)
}

pub fn text_primary() -> Hsla {
    rgb(0xf5f5f7).into()
}

pub fn text_muted() -> Hsla {
    rgb(0xa1a1a6).into()
}

pub fn text_faint() -> Hsla {
    rgb(0x6e6e73).into()
}

/// Icon helper using embedded assets
pub fn icon(name: &'static str, side: f32) -> Svg {
    svg()
        .path(name)
        .size(px(side))
        .text_color(text_muted())
        .flex_shrink_0()
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonLook {
    Ghost,
    Secondary,
    Primary,
    Destructive,
    Disabled,
}

pub fn button(id: impl Into<ElementId>, look: ButtonLook) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::Button)
        .h(px(BUTTON_HEIGHT))
        .px(px(if look == ButtonLook::Ghost { 8. } else { 12. }))
        .flex_shrink_0()
        .rounded(px(RADIUS_ROW))
        .flex()
        .items_center()
        .justify_center()
        .gap_1()
        .text_size(px(TEXT_SMALL))
        .font_weight(if look == ButtonLook::Primary { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
        .when(look != ButtonLook::Disabled, |d| d.cursor_pointer())
        .map(|d| match look {
            ButtonLook::Ghost => d
                .text_color(text_muted())
                .hover(|s| s.bg(hover_fill()).text_color(text_primary()))
                .active(|s| s.bg(selection_fill())),
            ButtonLook::Secondary => d
                .bg(card_fill())
                .border_1()
                .border_color(border_color())
                .text_color(text_primary())
                .hover(|s| s.bg(hover_fill()))
                .active(|s| s.bg(selection_fill())),
            ButtonLook::Primary => d
                .bg(accent())
                .text_color(gpui::black())
                .hover(|s| s.opacity(0.9))
                .active(|s| s.opacity(0.8)),
            ButtonLook::Destructive => d
                .bg(danger())
                .text_color(on_accent())
                .hover(|s| s.opacity(0.9)),
            ButtonLook::Disabled => d
                .bg(content(0.05))
                .text_color(content(0.25)),
        })
}

pub fn icon_button(
    id: impl Into<ElementId>,
    name: &'static str,
    active: bool,
    enabled: bool,
) -> Stateful<Div> {
    let base = div()
        .id(id)
        .role(Role::Button)
        .size(px(BUTTON_HEIGHT))
        .rounded(px(RADIUS_ROW))
        .flex()
        .items_center()
        .justify_center()
        .flex_shrink_0();

    if !enabled {
        return base
            .text_color(text_faint())
            .child(icon(name, 14.).text_color(text_faint()));
    }

    let ink = if active { accent() } else { text_muted() };

    base
        .cursor_pointer()
        .when(active, |d| d.bg(selection_fill()))
        .hover(|s| s.bg(hover_fill()).text_color(text_primary()))
        .active(|s| s.bg(selection_fill()))
        .child(icon(name, 14.).text_color(ink))
}

/// macOS-style joint container for grouped buttons (Share, Tag, More in Image 2)
pub fn macos_joint_container(id: impl Into<ElementId>, theme: &ThemeColors) -> Stateful<Div> {
    div()
        .id(id)
        .h(px(BUTTON_HEIGHT))
        .bg(theme.button_fill)
        .border_1()
        .border_color(theme.button_border)
        .rounded(px(7.))
        .flex()
        .items_center()
        .p(px(1.5))
        .gap(px(1.))
        .flex_shrink_0()
}

/// Button inside the joint container with subtle hover and active state
pub fn macos_joint_button(
    id: impl Into<ElementId>,
    name: &'static str,
    active: bool,
    enabled: bool,
    theme: &ThemeColors,
) -> Stateful<Div> {
    let base = div()
        .id(id)
        .role(Role::Button)
        .h(px(BUTTON_HEIGHT - 3.))
        .px(px(8.))
        .rounded(px(5.))
        .flex()
        .items_center()
        .justify_center()
        .flex_shrink_0();

    if !enabled {
        return base
            .text_color(theme.text_faint)
            .child(icon(name, 13.).text_color(theme.text_faint));
    }

    base
        .cursor_pointer()
        .when(active, |d| {
            d.bg(if theme.is_dark { gpui::rgba(0xffffff22) } else { rgb(0xffffff) })
                .border_1()
                .border_color(if theme.is_dark { gpui::rgba(0xffffff30) } else { rgb(0xd0d0d8) })
        })
        .when(!active, |d| {
            d.hover(|s| s.bg(theme.button_hover).text_color(theme.text_primary))
        })
        .child(icon(name, 13.).text_color(if active { theme.text_primary } else { theme.text_muted }))
}

/// macOS-style segmented container for grouped buttons (e.g. < | > navigation, view switchers)
pub fn glass_segmented_container(id: impl Into<ElementId>) -> Stateful<Div> {
    div()
        .id(id)
        .h(px(BUTTON_HEIGHT))
        .bg(glass_button_fill())
        .border_1()
        .border_color(glass_border())
        .rounded(px(7.))
        .flex()
        .items_center()
        .p(px(2.))
        .gap(px(1.))
        .flex_shrink_0()
}

/// Segment button inside a segmented container
pub fn glass_segment_button(
    id: impl Into<ElementId>,
    name: &'static str,
    active: bool,
    enabled: bool,
) -> Stateful<Div> {
    let base = div()
        .id(id)
        .role(Role::Button)
        .h(px(BUTTON_HEIGHT - 4.))
        .px(px(8.))
        .rounded(px(5.))
        .flex()
        .items_center()
        .justify_center()
        .flex_shrink_0();

    if !enabled {
        return base
            .text_color(text_faint())
            .child(icon(name, 13.).text_color(text_faint()));
    }

    base
        .cursor_pointer()
        .when(active, |d| {
            d.bg(gpui::rgba(0xffffff2b))
                .border_1()
                .border_color(gpui::rgba(0xffffff3d))
        })
        .when(!active, |d| {
            d.hover(|s| s.bg(gpui::rgba(0xffffff18)))
        })
        .child(
            icon(name, 13.)
                .text_color(if active { text_primary() } else { text_muted() }),
        )
}

/// macOS-style frosted glass icon button
pub fn glass_icon_button(
    id: impl Into<ElementId>,
    name: &'static str,
    active: bool,
    enabled: bool,
) -> Stateful<Div> {
    let base = div()
        .id(id)
        .role(Role::Button)
        .h(px(BUTTON_HEIGHT))
        .w(px(BUTTON_HEIGHT + 4.))
        .rounded(px(7.))
        .border_1()
        .flex()
        .items_center()
        .justify_center()
        .flex_shrink_0();

    if !enabled {
        return base
            .bg(glass_button_fill().opacity(0.3))
            .border_color(glass_border_subtle())
            .text_color(text_faint())
            .child(icon(name, 14.).text_color(text_faint()));
    }

    let ink = if active { accent() } else { text_muted() };

    base
        .cursor_pointer()
        .map(|d| {
            if active {
                d.bg(glass_selection())
                    .border_color(accent().opacity(0.6))
            } else {
                d.bg(glass_button_fill())
                    .border_color(glass_border())
            }
        })
        .hover(|s| {
            s.bg(glass_button_hover())
                .border_color(glass_border_highlight())
                .text_color(text_primary())
        })
        .active(|s| s.bg(glass_button_active()))
        .child(icon(name, 14.).text_color(if active { text_primary() } else { ink }))
}
