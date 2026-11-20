pub mod blur;
pub mod wayland;

pub use blur::BlurEngineStatus;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
pub use wayland::WaylandBlurSession;

/// Top-level platform blur coordinator.
/// Encapsulates native platform integrations behind a safe, clean API.
pub struct PlatformBlurCoordinator {
    session: Option<WaylandBlurSession>,
    fallback_status: BlurEngineStatus,
}

impl PlatformBlurCoordinator {
    /// Attempts to initialize the native blur backend from GPUI window & display handles.
    pub fn try_init<W: HasWindowHandle + HasDisplayHandle>(
        window: &W,
        width: i32,
        height: i32,
    ) -> Self {
        let Ok(display_handle) = window.display_handle() else {
            return Self {
                session: None,
                fallback_status: BlurEngineStatus::SimulatedFallback {
                    reason: "Failed to query raw display handle from GPUI window".into(),
                    compositor: "Unknown".into(),
                },
            };
        };

        let Ok(window_handle) = window.window_handle() else {
            return Self {
                session: None,
                fallback_status: BlurEngineStatus::SimulatedFallback {
                    reason: "Failed to query raw window handle from GPUI window".into(),
                    compositor: "Unknown".into(),
                },
            };
        };

        match (display_handle.as_raw(), window_handle.as_raw()) {
            (RawDisplayHandle::Wayland(d), RawWindowHandle::Wayland(w)) => {
                log::info!("Detected Wayland platform handles from GPUI window");
                unsafe {
                    match WaylandBlurSession::new(
                        d.display.as_ptr(),
                        w.surface.as_ptr(),
                        width,
                        height,
                    ) {
                        Ok(session) => Self {
                            fallback_status: session.status(),
                            session: Some(session),
                        },
                        Err(err) => {
                            log::warn!("Wayland blur session initialization failed: {err}");
                            Self {
                                session: None,
                                fallback_status: BlurEngineStatus::SimulatedFallback {
                                    reason: err,
                                    compositor: "Wayland".into(),
                                },
                            }
                        }
                    }
                }
            }
            (RawDisplayHandle::Xlib(_), _) | (RawDisplayHandle::Xcb(_), _) => Self {
                session: None,
                fallback_status: BlurEngineStatus::NonWayland {
                    display_server: "X11".into(),
                },
            },
            _ => Self {
                session: None,
                fallback_status: BlurEngineStatus::SimulatedFallback {
                    reason: "Compositor interface unsupported on this platform".into(),
                    compositor: "Non-Wayland Platform".into(),
                },
            },
        }
    }

    pub fn status(&self) -> BlurEngineStatus {
        if let Some(session) = &self.session {
            session.status()
        } else {
            self.fallback_status.clone()
        }
    }

    #[allow(dead_code)]
    pub fn is_native_active(&self) -> bool {
        self.session
            .as_ref()
            .map(|s| s.is_enabled())
            .unwrap_or(false)
    }

    #[allow(dead_code)]
    pub fn set_enabled(&mut self, enabled: bool) {
        if let Some(session) = &mut self.session {
            session.set_enabled(enabled);
        }
    }

    #[allow(dead_code)]
    pub fn update_bounds(&mut self, width: i32, height: i32) {
        if let Some(session) = &mut self.session {
            session.update_bounds(width, height);
        }
    }
}
