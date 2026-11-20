/// Core abstractions for window background blur.
/// Clearly differentiates real compositor-backed blur from simulated translucency.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlurProtocol {
    /// Upstream Wayland staging protocol: `ext-background-effect-v1`
    ExtBackgroundEffectV1,
    /// KDE Plasma legacy protocol: `org_kde_kwin_blur`
    OrgKdeKwinBlur,
    /// No blur protocol advertised by compositor
    None,
}

impl BlurProtocol {
    pub fn name(&self) -> &'static str {
        match self {
            BlurProtocol::ExtBackgroundEffectV1 => "ext-background-effect-v1",
            BlurProtocol::OrgKdeKwinBlur => "org_kde_kwin_blur",
            BlurProtocol::None => "None (Unsupported)",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlurEngineStatus {
    /// Native compositor blur is active and applied via protocol
    NativeActive {
        protocol: BlurProtocol,
        compositor: String,
        details: String,
    },
    /// Compositor supports the protocol, but native blur is temporarily disabled by user
    NativeDisabled {
        protocol: BlurProtocol,
        compositor: String,
    },
    /// The running compositor does not support Wayland background blur protocols.
    /// Translucent fallback is in use and explicitly identified as simulated glass.
    SimulatedFallback {
        reason: String,
        compositor: String,
    },
    /// Running on non-Wayland platform (e.g. X11)
    NonWayland {
        display_server: String,
    },
}

#[allow(dead_code)]
impl BlurEngineStatus {
    pub fn is_native(&self) -> bool {
        matches!(self, BlurEngineStatus::NativeActive { .. })
    }

    pub fn supports_native(&self) -> bool {
        matches!(
            self,
            BlurEngineStatus::NativeActive { .. } | BlurEngineStatus::NativeDisabled { .. }
        )
    }

    pub fn badge_text(&self) -> &'static str {
        match self {
            BlurEngineStatus::NativeActive { .. } => "Native Wayland Blur Active",
            BlurEngineStatus::NativeDisabled { .. } => "Native Blur Paused",
            BlurEngineStatus::SimulatedFallback { .. } => "Simulated Glass (No Compositor Protocol)",
            BlurEngineStatus::NonWayland { .. } => "Simulated Glass (Non-Wayland)",
        }
    }

    pub fn description(&self) -> String {
        match self {
            BlurEngineStatus::NativeActive {
                protocol,
                compositor,
                details,
            } => {
                format!(
                    "Hardware compositor-backed blur via {} on {}. {}",
                    protocol.name(),
                    compositor,
                    details
                )
            }
            BlurEngineStatus::NativeDisabled {
                protocol,
                compositor,
            } => {
                format!(
                    "Compositor ({}) supports {}, but blur region is currently cleared.",
                    compositor,
                    protocol.name()
                )
            }
            BlurEngineStatus::SimulatedFallback { reason, compositor } => {
                format!(
                    "Simulated glass only: {}. Compositor: {}. Note: Transparency and GPU alpha alone do NOT provide background blur.",
                    reason, compositor
                )
            }
            BlurEngineStatus::NonWayland { display_server } => {
                format!(
                    "Simulated glass on {}. Wayland background effect protocols require a Wayland session.",
                    display_server
                )
            }
        }
    }
}
