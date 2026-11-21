//! Retained motion & spring animation engine inspired by zgx (https://zgx.zeron.sh)
//! and Framer Motion, powering the morphing Search Bar component in GPUI.

use crate::theme::{self, ThemeColors};
use gpui::{
    div, prelude::*, px, Animation, AnimationExt, AnyElement, BoxShadow, Context, Div,
    FontWeight, MouseMoveEvent, Role, SharedString,
};
use std::time::Duration;

// -----------------------------------------------------------------------------
// Closed-Form Spring Physics Engine (matching zgx & Framer Motion)
// -----------------------------------------------------------------------------

/// Analytical damped harmonic oscillator parameters.
#[derive(Clone, Copy, Debug)]
pub struct SpringPhysics {
    pub stiffness: f32,
    pub damping: f32,
    pub mass: f32,
}

impl SpringPhysics {
    pub const fn new(stiffness: f32, damping: f32, mass: f32) -> Self {
        Self { stiffness, damping, mass }
    }

    /// Evaluates the closed-form damped spring equation at time `t` (in seconds).
    /// Produces natural physical harmonic oscillation and subtle overshoot.
    pub fn evaluate(&self, from: f32, to: f32, t: f32) -> f32 {
        if t <= 0.0 {
            return from;
        }
        let delta = from - to;
        let w0 = (self.stiffness / self.mass).sqrt();
        let zeta = self.damping / (2.0 * (self.stiffness * self.mass).sqrt());

        if zeta < 1.0 {
            // Underdamped: creates responsive, smooth harmonic overshoot
            let wd = w0 * (1.0 - zeta * zeta).sqrt();
            let envelope = (-zeta * w0 * t).exp();
            let cos_term = (wd * t).cos();
            let sin_term = (wd * t).sin();
            let displacement = envelope * (delta * cos_term + ((zeta * w0 * delta) / wd) * sin_term);
            to + displacement
        } else if (zeta - 1.0).abs() < 1e-4 {
            // Critically damped
            let envelope = (-w0 * t).exp();
            let displacement = envelope * (delta + (w0 * delta) * t);
            to + displacement
        } else {
            // Overdamped
            let w1 = w0 * (zeta * zeta - 1.0).sqrt();
            let envelope = (-zeta * w0 * t).exp();
            let c1 = delta;
            let c2 = (zeta * w0 * delta) / w1;
            let displacement = envelope * (c1 * (w1 * t).cosh() + c2 * (w1 * t).sinh());
            to + displacement
        }
    }

    /// Converts this spring configuration into a GPUI Animation with settling duration.
    pub fn into_animation(&self, settle_ms: u64) -> Animation {
        let spring = *self;
        let duration_secs = (settle_ms as f32) / 1000.0;
        Animation::new(Duration::from_millis(settle_ms)).with_easing(move |progress| {
            let t = progress * duration_secs;
            spring.evaluate(0.0, 1.0, t)
        })
    }
}

/// Exact spring presets from the React search-bar component:
/// const springSnappy = { stiffness: 520, damping: 34, mass: 0.72 };
pub const SPRING_SNAPPY: SpringPhysics = SpringPhysics::new(520.0, 34.0, 0.72);

/// const springPill = { stiffness: 420, damping: 32, mass: 0.55 };
pub const SPRING_PILL: SpringPhysics = SpringPhysics::new(420.0, 32.0, 0.55);

/// const springMicro = { stiffness: 620, damping: 28, mass: 0.6 };
pub const SPRING_MICRO: SpringPhysics = SpringPhysics::new(620.0, 28.0, 0.60);

// -----------------------------------------------------------------------------
// Data Models
// -----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub struct Country {
    pub name: &'static str,
    pub code: &'static str,
    pub flag: &'static str,
}

pub const COUNTRIES: [Country; 20] = [
    Country { name: "United States", code: "US", flag: "🇺🇸" },
    Country { name: "United Kingdom", code: "GB", flag: "🇬🇧" },
    Country { name: "Canada", code: "CA", flag: "🇨🇦" },
    Country { name: "Australia", code: "AU", flag: "🇦🇺" },
    Country { name: "Germany", code: "DE", flag: "🇩🇪" },
    Country { name: "France", code: "FR", flag: "🇫🇷" },
    Country { name: "Italy", code: "IT", flag: "🇮🇹" },
    Country { name: "Spain", code: "ES", flag: "🇪🇸" },
    Country { name: "Japan", code: "JP", flag: "🇯🇵" },
    Country { name: "South Korea", code: "KR", flag: "🇰🇷" },
    Country { name: "China", code: "CN", flag: "🇨🇳" },
    Country { name: "India", code: "IN", flag: "🇮🇳" },
    Country { name: "Brazil", code: "BR", flag: "🇧🇷" },
    Country { name: "Mexico", code: "MX", flag: "🇲🇽" },
    Country { name: "Netherlands", code: "NL", flag: "🇳🇱" },
    Country { name: "Sweden", code: "SE", flag: "🇸🇪" },
    Country { name: "Switzerland", code: "CH", flag: "🇨🇭" },
    Country { name: "Singapore", code: "SG", flag: "🇸🇬" },
    Country { name: "New Zealand", code: "NZ", flag: "🇳🇿" },
    Country { name: "Ireland", code: "IE", flag: "🇮🇪" },
];

pub fn compute_list_height(count: usize) -> f32 {
    if count == 0 {
        48.0
    } else {
        ((count as f32) * 36.5 + 8.0).min(288.0)
    }
}

#[derive(Clone, Debug)]
pub struct SearchBarState {
    pub query: String,
    pub selected_code: String,
    pub prev_active_index: usize,
    pub active_index: usize,
    pub prev_item_count: usize,
    pub is_open: bool,
    pub open_epoch: usize,
    pub hover_epoch: usize,
    pub filter_epoch: usize,
}

impl Default for SearchBarState {
    fn default() -> Self {
        Self {
            query: String::new(),
            selected_code: "US".to_string(),
            prev_active_index: 0,
            active_index: 0,
            prev_item_count: COUNTRIES.len(),
            is_open: false,
            open_epoch: 0,
            hover_epoch: 0,
            filter_epoch: 0,
        }
    }
}

impl SearchBarState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self) {
        if !self.is_open {
            self.is_open = true;
            self.open_epoch = self.open_epoch.wrapping_add(1);
            self.prev_active_index = 0;
            self.active_index = 0;
            self.prev_item_count = COUNTRIES.len();
        }
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn toggle(&mut self) {
        if self.is_open {
            self.close();
        } else {
            self.open();
        }
    }

    pub fn set_active_index(&mut self, new_idx: usize) {
        if self.active_index != new_idx {
            self.prev_active_index = self.active_index;
            self.active_index = new_idx;
            self.hover_epoch = self.hover_epoch.wrapping_add(1);
        }
    }

    pub fn filtered_countries(&self) -> Vec<Country> {
        let q = self.query.trim().to_lowercase();
        if q.is_empty() {
            COUNTRIES.to_vec()
        } else {
            COUNTRIES
                .iter()
                .filter(|c| c.name.to_lowercase().contains(&q) || c.code.to_lowercase().contains(&q))
                .copied()
                .collect()
        }
    }

    pub fn handle_key(&mut self, key: &str) -> bool {
        if !self.is_open {
            if key == "down" || key == "enter" {
                self.open();
                return true;
            }
            return false;
        }

        let filtered = self.filtered_countries();
        match key {
            "escape" => {
                self.query.clear();
                self.close();
                true
            }
            "down" => {
                if !filtered.is_empty() {
                    let next = (self.active_index + 1) % filtered.len();
                    self.set_active_index(next);
                }
                true
            }
            "up" => {
                if !filtered.is_empty() {
                    let prev = (self.active_index + filtered.len() - 1) % filtered.len();
                    self.set_active_index(prev);
                }
                true
            }
            "enter" => {
                if let Some(c) = filtered.get(self.active_index) {
                    self.selected_code = c.code.to_string();
                    self.query = c.name.to_string();
                    self.close();
                }
                true
            }
            "backspace" => {
                self.prev_item_count = filtered.len();
                self.query.pop();
                self.active_index = 0;
                self.prev_active_index = 0;
                self.filter_epoch = self.filter_epoch.wrapping_add(1);
                true
            }
            _ => {
                if key.len() == 1 {
                    let ch = key.chars().next().unwrap();
                    if ch.is_ascii_alphanumeric() || ch == ' ' || ch == '-' {
                        self.prev_item_count = filtered.len();
                        self.query.push(ch);
                        self.active_index = 0;
                        self.prev_active_index = 0;
                        self.filter_epoch = self.filter_epoch.wrapping_add(1);
                        return true;
                    }
                }
                false
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Component Helpers
// -----------------------------------------------------------------------------

fn highlight_match(text: &str, query: &str, theme: &ThemeColors) -> Div {
    let q = query.trim();
    if q.is_empty() {
        return div().child(text.to_string());
    }

    let text_lower = text.to_lowercase();
    let q_lower = q.to_lowercase();

    if let Some(idx) = text_lower.find(&q_lower) {
        let (before, rest) = text.split_at(idx);
        let (matched, after) = rest.split_at(q.len());

        div()
            .flex()
            .items_center()
            .child(div().child(before.to_string()))
            .child(
                div()
                    .px_1()
                    .py(px(0.5))
                    .rounded(px(3.))
                    .bg(if theme.is_dark {
                        gpui::rgba(0xe0af6835)
                    } else {
                        gpui::rgba(0xfef08a90)
                    })
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.accent)
                    .child(matched.to_string()),
            )
            .child(div().child(after.to_string()))
    } else {
        div().child(text.to_string())
    }
}

fn key_badge(label: &str, theme: &ThemeColors) -> Div {
    div()
        .h(px(18.))
        .min_w(px(18.))
        .px(px(4.))
        .rounded(px(4.))
        .bg(if theme.is_dark { gpui::rgba(0xffffff12) } else { gpui::rgba(0x0000000a) })
        .border_1()
        .border_color(theme.surface_border)
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(10.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(theme.text_faint)
        .child(label.to_string())
}

// -----------------------------------------------------------------------------
// Public Renderers
// -----------------------------------------------------------------------------

/// Renders the search bar capsule in the top toolbar with spring morph feedback.
pub fn render_search_capsule<T: 'static>(
    state: &SearchBarState,
    theme: &ThemeColors,
    on_toggle: impl Fn(&mut T, &mut Context<T>) + Copy + 'static,
    on_clear: impl Fn(&mut T, &mut Context<T>) + Copy + 'static,
    cx: &mut Context<T>,
) -> AnyElement {
    let filtered_count = state.filtered_countries().len();
    let is_searching = !state.query.trim().is_empty();

    div()
        .id("animated-search-bar-input")
        .role(Role::Button)
        .h(px(theme::BUTTON_HEIGHT))
        .px_3()
        .bg(theme.search_fill)
        .rounded_full()
        .border_1()
        .border_color(if state.is_open {
            theme.accent
        } else {
            theme.search_border
        })
        .shadow(if state.is_open {
            vec![
                BoxShadow::new(px(0.), px(0.), gpui::rgba(0x007aff28).into())
                    .blur_radius(px(6.))
                    .spread_radius(px(1.)),
            ]
        } else {
            theme::control_shadow(theme)
        })
        .flex()
        .items_center()
        .gap_2()
        .cursor_pointer()
        .hover(|s| s.bg(theme.button_hover))
        .on_click(cx.listener(move |this, _, _, cx| on_toggle(this, cx)))
        // Dynamic spring width morphing: 280px closed -> 310px open
        .with_animation(
            ("capsule-width-spring", state.open_epoch),
            SPRING_SNAPPY.into_animation(240),
            {
                let is_open = state.is_open;
                move |div, progress| {
                    let w = if is_open {
                        280.0 + (310.0 - 280.0) * progress.clamp(0.0, 1.0)
                    } else {
                        310.0 - (310.0 - 280.0) * progress.clamp(0.0, 1.0)
                    };
                    div.w(px(w))
                }
            },
        )
        .child(theme::icon("search", 13.).text_color(theme.text_muted))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(px(theme::TEXT_SMALL))
                .text_color(if state.query.is_empty() {
                    theme.text_faint
                } else {
                    theme.text_primary
                })
                .child(if state.query.is_empty() {
                    format!("Search {} countries", COUNTRIES.len())
                } else {
                    format!("{}|", state.query)
                }),
        )
        // Dynamic Right Badge with SPRING_MICRO transition
        .map(|d| {
            if is_searching {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .with_animation(
                            "search-counter-spring",
                            SPRING_MICRO.into_animation(200),
                            |div, progress| {
                                div.opacity(progress.clamp(0.0, 1.0))
                            },
                        )
                        .child(
                            div()
                                .px_1p5()
                                .py(px(1.5))
                                .rounded(px(5.))
                                .bg(if theme.is_dark {
                                    gpui::rgba(0xffffff18)
                                } else {
                                    gpui::rgba(0x0000000a)
                                })
                                .text_size(px(10.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(theme.text_faint)
                                .child(format!("{}", filtered_count)),
                        )
                        .child(
                            div()
                                .id("animated-search-bar-clear-btn")
                                .role(Role::Button)
                                .cursor_pointer()
                                .p_0p5()
                                .rounded_full()
                                .hover(|s| s.bg(theme.button_hover))
                                .on_click(cx.listener(move |this, _, _, cx| on_clear(this, cx)))
                                .child(theme::icon("cancel-01", 11.).text_color(theme.text_muted)),
                        ),
                )
            } else {
                d.child(
                    div()
                        .with_animation(
                            "esc-badge-spring",
                            SPRING_MICRO.into_animation(200),
                            |div, progress| {
                                div.opacity(progress.clamp(0.0, 1.0))
                            },
                        )
                        .child(
                            div()
                                .px_1p5()
                                .py(px(1.5))
                                .rounded(px(5.))
                                .bg(if theme.is_dark {
                                    gpui::rgba(0xffffff10)
                                } else {
                                    gpui::rgba(0x00000008)
                                })
                                .border_1()
                                .border_color(theme.surface_border)
                                .text_size(px(10.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(theme.text_faint)
                                .child("esc"),
                        ),
                )
            }
        })
        .into_any_element()
}

/// Renders the animated dropdown popover overlay with:
/// 1. Card spring entrance with slight overshoot
/// 2. layoutId="active-pill" gliding morph across items on hover/selection
/// 3. Spring height shrink/morph of the list when filtering
pub fn render_search_popover<T: 'static>(
    state: &SearchBarState,
    theme: &ThemeColors,
    on_close: impl Fn(&mut T, &mut Context<T>) + Copy + 'static,
    on_select: impl Fn(&mut T, &'static str, &'static str, &mut Context<T>) + Copy + 'static,
    on_hover_index: impl Fn(&mut T, usize, &mut Context<T>) + Copy + 'static,
    cx: &mut Context<T>,
) -> AnyElement {
    let filtered = state.filtered_countries();
    let filtered_count = filtered.len();

    // 1. Calculate height morphing between previous item count and current item count
    let from_h = compute_list_height(state.prev_item_count);
    let to_h = compute_list_height(filtered_count);

    // 2. Calculate active-pill glide positions from previous active index to current active index
    let from_pill_y = 6.0 + (state.prev_active_index as f32) * 36.5;
    let to_pill_y = 6.0 + (state.active_index as f32) * 36.5;

    div()
        .id("search-bar-popover-overlay")
        .absolute()
        .inset_0()
        .bg(gpui::rgba(0x00000000))
        .flex()
        .items_start()
        .justify_center()
        .pt(px(46.))
        .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
        .on_click(cx.listener(move |this, _, _, cx| on_close(this, cx)))
        .child(
            div()
                .id("animated-search-bar-card")
                .relative()
                .w(px(390.))
                .bg(theme.card)
                .border_1()
                .border_color(theme.surface_border)
                .shadow(theme::popover_shadows(theme))
                .rounded(px(16.))
                .overflow_hidden()
                .flex()
                .flex_col()
                .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                .on_click(|_, _, cx| cx.stop_propagation())
                .child(theme::glass_specular_sheen(theme))
                // Card Entrance with SPRING_SNAPPY (stiffness: 520, damping: 34, mass: 0.72)
                .with_animation(
                    ("search-bar-card-spring", state.open_epoch),
                    SPRING_SNAPPY.into_animation(280),
                    |card_div, phase| {
                        let y_offset = (1.0 - phase) * -8.0;
                        card_div
                            .opacity(phase.clamp(0.0, 1.0))
                            .top(px(y_offset))
                    },
                )
                // Scrollable List Container with Height Morphing Animation:
                // Smoothly shrinks and expands height as items are filtered!
                .child(
                    div()
                        .id("animated-search-bar-list-viewport")
                        .relative()
                        .overflow_y_scroll()
                        .p_1p5()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                        // Spring Height Morphing Animation
                        .with_animation(
                            ("search-bar-list-height-morph", state.filter_epoch),
                            SPRING_SNAPPY.into_animation(240),
                            move |viewport_div, progress| {
                                let h = from_h + (to_h - from_h) * progress.clamp(0.0, 1.0);
                                viewport_div.h(px(h))
                            },
                        )
                        .map(|mut list_container| {
                            if filtered.is_empty() {
                                list_container = list_container.child(
                                    div()
                                        .px_3()
                                        .py_4()
                                        .text_size(px(12.))
                                        .text_color(theme.text_faint)
                                        .child(format!("No matches for \"{}\"", state.query)),
                                );
                            } else {
                                // Floating Animated Active Pill (`layoutId="active-pill"`):
                                // Smoothly morphs and glides between items on hover or keyboard selection!
                                list_container = list_container.child(
                                    div()
                                        .id("morphing-active-pill-tracker")
                                        .absolute()
                                        .left(px(6.))
                                        .right(px(6.))
                                        .h(px(34.))
                                        .rounded(px(10.))
                                        .bg(if theme.is_dark {
                                            gpui::rgba(0xffffff18)
                                        } else {
                                            gpui::rgba(0x0000000d)
                                        })
                                        // Gliding Spring Morph
                                        .with_animation(
                                            ("active-pill-glide-morph", state.hover_epoch),
                                            SPRING_PILL.into_animation(220),
                                            move |pill_div, progress| {
                                                let current_y = from_pill_y + (to_pill_y - from_pill_y) * progress;
                                                pill_div.top(px(current_y))
                                            },
                                        ),
                                );

                                // List items
                                for (idx, country) in filtered.iter().enumerate() {
                                    let is_selected = country.code == state.selected_code;
                                    let country_code = country.code;
                                    let country_name = country.name;

                                    let item_row = div()
                                        .id(SharedString::from(format!("country-item-{}", country.code)))
                                        .role(Role::Button)
                                        .relative()
                                        .h(px(36.))
                                        .px_3()
                                        .rounded(px(10.))
                                        .cursor_pointer()
                                        .flex()
                                        .items_center()
                                        .gap_3()
                                        .active(|s| s.opacity(0.85))
                                        // Hover updates the active index, triggering the gliding pill morph!
                                        .on_mouse_move(cx.listener(move |this, _: &MouseMoveEvent, _, cx| {
                                            on_hover_index(this, idx, cx);
                                        }))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            on_select(this, country_code, country_name, cx);
                                        }))
                                        // Flag emoji
                                        .child(
                                            div()
                                                .text_size(px(16.))
                                                .child(country.flag),
                                        )
                                        // Country Name with Substring Highlight
                                        .child(
                                            div()
                                                .flex_1()
                                                .text_size(px(13.))
                                                .text_color(if is_selected {
                                                    theme.text_primary
                                                } else {
                                                    theme.text_muted
                                                })
                                                .font_weight(if is_selected {
                                                    FontWeight::SEMIBOLD
                                                } else {
                                                    FontWeight::NORMAL
                                                })
                                                .child(highlight_match(
                                                    country.name,
                                                    &state.query,
                                                    theme,
                                                )),
                                        )
                                        // Selection Checkmark with SPRING_MICRO pop-in
                                        .when(is_selected, |row| {
                                            row.child(
                                                div()
                                                    .with_animation(
                                                        SharedString::from(format!("check-spring-{}", country.code)),
                                                        SPRING_MICRO.into_animation(180),
                                                        |check_div, scale_phase| {
                                                            check_div.opacity(scale_phase.clamp(0.0, 1.0))
                                                        },
                                                    )
                                                    .child(
                                                        theme::icon("check-hi", 13.)
                                                            .text_color(theme.accent),
                                                    ),
                                            )
                                        })
                                        // Country Code Badge
                                        .child(
                                            div()
                                                .px_2()
                                                .py(px(2.))
                                                .rounded(px(6.))
                                                .bg(if is_selected {
                                                    theme.accent
                                                } else if theme.is_dark {
                                                    gpui::rgba(0xffffff0e).into()
                                                } else {
                                                    gpui::rgba(0x00000008).into()
                                                })
                                                .text_size(px(10.))
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(if is_selected {
                                                    gpui::white()
                                                } else {
                                                    theme.text_faint
                                                })
                                                .child(country.code),
                                        );

                                    list_container = list_container.child(item_row);
                                }
                            }
                            list_container
                        }),
                )
                // Footer
                .child(
                    div()
                        .h(px(32.))
                        .px_3()
                        .border_t_1()
                        .border_color(theme.surface_border)
                        .bg(if theme.is_dark {
                            gpui::rgba(0x00000018)
                        } else {
                            gpui::rgba(0x00000006)
                        })
                        .flex()
                        .items_center()
                        .justify_between()
                        .text_size(px(11.))
                        .text_color(theme.text_faint)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_3()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(key_badge("↑", theme))
                                        .child(key_badge("↓", theme))
                                        .child(div().ml_1().child("move")),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(key_badge("↵", theme))
                                        .child(div().ml_1().child("select")),
                                ),
                        )
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .child(format!(
                                    "{} {}",
                                    filtered_count,
                                    if filtered_count == 1 { "country" } else { "countries" }
                                )),
                        ),
                ),
        )
        .into_any_element()
}
