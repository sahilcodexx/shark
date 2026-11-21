//! Retained motion & spring animation engine inspired by zgx (https://zgx.zeron.sh)
//! and Framer Motion, powering the morphing Search Bar component in GPUI.

use gpui::Animation;
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
