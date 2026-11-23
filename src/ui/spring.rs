use gpui::Animation;
use std::time::Duration;

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

/// Silky fluid spring for popovers and layout morphing (matching macOS & Framer Motion):
/// stiffness: 260, damping: 26, mass: 0.95
pub const SPRING_SILKY: SpringPhysics = SpringPhysics::new(260.0, 26.0, 0.95);
