//! Appearance tokens and styling utilities for Shark GPUI file manager.

use gpui::{
    div, px, rgb, svg, ElementId, Hsla, Role, Stateful, Svg, Div,
    FontWeight, prelude::*,
};

pub const RADIUS_THUMB: f32 = 6.;
pub const RADIUS_ROW: f32 = 8.;
pub const RADIUS_CARD: f32 = 12.;
pub const RADIUS_PILL: f32 = 16.;

pub const TEXT_CAPTION: f32 = 11.;
pub const TEXT_SMALL: f32 = 12.;
pub const TEXT_BODY: f32 = 13.;
pub const TEXT_TITLE: f32 = 14.;
pub const TEXT_HEADER: f32 = 16.;

pub const TITLE_BAR_HEIGHT: f32 = 48.;
pub const ROW_HEIGHT: f32 = 32.;
pub const SIDEBAR_ROW_HEIGHT: f32 = 34.;
pub const PATH_BAR_HEIGHT: f32 = 32.;
pub const BUTTON_HEIGHT: f32 = 28.;

// Shark sleek dark palette
pub fn shell_fill() -> Hsla {
    rgb(0x0f1217).into()
}

pub fn pane_fill() -> Hsla {
    rgb(0x13171f).into()
}

pub fn card_fill() -> Hsla {
    rgb(0x181e28).into()
}

pub fn sidebar_fill() -> Hsla {
    rgb(0x0c0f14).into()
}

pub fn border_color() -> Hsla {
    rgb(0x232a38).into()
}

pub fn border_subtle() -> Hsla {
    rgb(0x1b202c).into()
}

pub fn accent() -> Hsla {
    rgb(0x38bdf8).into() // Sky blue / cyan 400
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
    content(1.0)
}

pub fn text_muted() -> Hsla {
    content(0.65)
}

pub fn text_faint() -> Hsla {
    content(0.40)
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
