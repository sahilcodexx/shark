use std::ffi::c_void;
use log::{info, warn};
use wayland_backend::client::ObjectId;
use wayland_backend::sys::client::Backend;
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
    protocol::{wl_compositor, wl_region, wl_registry, wl_surface},
};
use wayland_protocols::ext::background_effect::v1::client::{
    ext_background_effect_manager_v1::{self, ExtBackgroundEffectManagerV1},
    ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1,
};
use wayland_protocols_plasma::blur::client::{
    org_kde_kwin_blur::OrgKdeKwinBlur,
    org_kde_kwin_blur_manager::OrgKdeKwinBlurManager,
};

use super::blur::{BlurEngineStatus, BlurProtocol};

/// Internal Wayland dispatch state
pub struct WaylandClientData {
    pub compositor: Option<wl_compositor::WlCompositor>,
    pub ext_manager: Option<ExtBackgroundEffectManagerV1>,
    pub ext_blur_supported: bool,
    pub kde_manager: Option<OrgKdeKwinBlurManager>,
    pub advertised_globals: Vec<String>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for WaylandClientData {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            state.advertised_globals.push(format!("{interface} v{version}"));
            if interface == "wl_compositor" {
                state.compositor = Some(registry.bind(name, version.min(6), qh, ()));
            } else if interface == "ext_background_effect_manager_v1" {
                info!("Found ext_background_effect_manager_v1 (v{version})");
                state.ext_manager = Some(registry.bind(name, 1, qh, ()));
            } else if interface == "org_kde_kwin_blur_manager" {
                info!("Found org_kde_kwin_blur_manager (v{version})");
                state.kde_manager = Some(registry.bind(name, 1, qh, ()));
            }
        }
    }
}

impl Dispatch<wl_compositor::WlCompositor, ()> for WaylandClientData {
    fn event(
        _: &mut Self,
        _: &wl_compositor::WlCompositor,
        _: wl_compositor::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtBackgroundEffectManagerV1, ()> for WaylandClientData {
    fn event(
        state: &mut Self,
        _: &ExtBackgroundEffectManagerV1,
        event: ext_background_effect_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let ext_background_effect_manager_v1::Event::Capabilities { flags } = event {
            let cap_val = match flags {
                wayland_client::WEnum::Value(v) => v.bits(),
                wayland_client::WEnum::Unknown(u) => u,
            };
            info!("ext_background_effect_manager_v1 capabilities: {cap_val:#x}");
            if cap_val & ext_background_effect_manager_v1::Capability::Blur.bits() != 0 {
                state.ext_blur_supported = true;
            }
        }
    }
}

impl Dispatch<ExtBackgroundEffectSurfaceV1, ()> for WaylandClientData {
    fn event(
        _: &mut Self,
        _: &ExtBackgroundEffectSurfaceV1,
        _: <ExtBackgroundEffectSurfaceV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<OrgKdeKwinBlurManager, ()> for WaylandClientData {
    fn event(
        _: &mut Self,
        _: &OrgKdeKwinBlurManager,
        _: <OrgKdeKwinBlurManager as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<OrgKdeKwinBlur, ()> for WaylandClientData {
    fn event(
        _: &mut Self,
        _: &OrgKdeKwinBlur,
        _: <OrgKdeKwinBlur as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_region::WlRegion, ()> for WaylandClientData {
    fn event(
        _: &mut Self,
        _: &wl_region::WlRegion,
        _: wl_region::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

enum SurfaceBlurHandler {
    ExtEffect {
        effect_surface: ExtBackgroundEffectSurfaceV1,
    },
    KdeBlur {
        blur: OrgKdeKwinBlur,
    },
    None,
}

/// Controller for native Wayland background blur.
/// Operates on the existing GPUI wl_display and wl_surface.
#[allow(dead_code)]
pub struct WaylandBlurSession {
    connection: Connection,
    queue: EventQueue<WaylandClientData>,
    state: WaylandClientData,
    surface: wl_surface::WlSurface,
    handler: SurfaceBlurHandler,
    protocol: BlurProtocol,
    compositor_name: String,
    current_bounds: (i32, i32),
    enabled: bool,
}

#[allow(dead_code)]
impl WaylandBlurSession {
    /// Attempts to attach native blur to GPUI's active surface using the same Wayland display.
    pub unsafe fn new(
        display_ptr: *mut c_void,
        surface_ptr: *mut c_void,
        width: i32,
        height: i32,
    ) -> Result<Self, String> {
        if display_ptr.is_null() || surface_ptr.is_null() {
            return Err("Null display or surface pointer provided".to_string());
        }

        let compositor_name = detect_compositor_name();

        // 1. Wrap the existing C wl_display without opening a second independent socket.
        let backend = unsafe { Backend::from_foreign_display(display_ptr as *mut _) };
        let connection = Connection::from_backend(backend);

        // 2. Create an isolated event queue for our effect objects.
        let mut queue = connection.new_event_queue();
        let qh = queue.handle();

        // 3. Obtain registry and discover globals on the connection.
        let display = connection.display();
        let _registry = display.get_registry(&qh, ());

        let mut state = WaylandClientData {
            compositor: None,
            ext_manager: None,
            ext_blur_supported: false,
            kde_manager: None,
            advertised_globals: Vec::new(),
        };

        // Roundtrip to receive globals
        queue
            .roundtrip(&mut state)
            .map_err(|e| format!("Wayland roundtrip failed: {e}"))?;
        // Second roundtrip to receive ext_manager capabilities event
        queue
            .roundtrip(&mut state)
            .map_err(|e| format!("Wayland capabilities roundtrip failed: {e}"))?;

        // 4. Wrap GPUI's wl_surface proxy on this connection
        let surface_id = unsafe {
            ObjectId::from_ptr(wl_surface::WlSurface::interface(), surface_ptr as *mut _)
        }
        .map_err(|e| format!("Failed to create ObjectId for wl_surface: {e}"))?;

        let surface: wl_surface::WlSurface = wl_surface::WlSurface::from_id(&connection, surface_id)
            .map_err(|e| format!("Failed to bind wl_surface proxy: {e}"))?;

        // 5. Select blur protocol
        let (handler, protocol) = if let Some(ext_manager) = &state.ext_manager {
            if state.ext_blur_supported {
                info!("Using ext-background-effect-v1 on {compositor_name}");
                let effect_surface = ext_manager.get_background_effect(&surface, &qh, ());
                (
                    SurfaceBlurHandler::ExtEffect { effect_surface },
                    BlurProtocol::ExtBackgroundEffectV1,
                )
            } else {
                warn!("ext_background_effect_manager_v1 present but blur capability not flagged");
                (SurfaceBlurHandler::None, BlurProtocol::None)
            }
        } else if let Some(kde_manager) = &state.kde_manager {
            info!("Using org_kde_kwin_blur on {compositor_name}");
            let blur = kde_manager.create(&surface, &qh, ());
            (
                SurfaceBlurHandler::KdeBlur { blur },
                BlurProtocol::OrgKdeKwinBlur,
            )
        } else {
            warn!("No Wayland blur protocol supported by compositor: {compositor_name}");
            (SurfaceBlurHandler::None, BlurProtocol::None)
        };

        let mut session = Self {
            connection,
            queue,
            state,
            surface,
            handler,
            protocol,
            compositor_name,
            current_bounds: (width, height),
            enabled: true,
        };

        if session.protocol != BlurProtocol::None {
            session.apply_region(width, height, true);
        }

        Ok(session)
    }

    pub fn protocol(&self) -> BlurProtocol {
        self.protocol
    }

    pub fn compositor_name(&self) -> &str {
        &self.compositor_name
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled && self.protocol != BlurProtocol::None
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        let (w, h) = self.current_bounds;
        self.apply_region(w, h, enabled);
    }

    pub fn update_bounds(&mut self, width: i32, height: i32) {
        self.current_bounds = (width, height);
        if self.enabled {
            self.apply_region(width, height, true);
        }
    }

    fn apply_region(&mut self, width: i32, height: i32, enable: bool) {
        let qh = self.queue.handle();
        match &self.handler {
            SurfaceBlurHandler::ExtEffect { effect_surface } => {
                if enable {
                    if let Some(comp) = &self.state.compositor {
                        let region = comp.create_region(&qh, ());
                        region.add(0, 0, width, height);
                        effect_surface.set_blur_region(Some(&region));
                        region.destroy();
                        info!("Applied ext-background-effect-v1 blur region: {width}x{height}");
                    }
                } else {
                    effect_surface.set_blur_region(None);
                    info!("Cleared ext-background-effect-v1 blur region");
                }
                let _ = self.connection.flush();
            }
            SurfaceBlurHandler::KdeBlur { blur } => {
                if enable {
                    if let Some(comp) = &self.state.compositor {
                        let region = comp.create_region(&qh, ());
                        region.add(0, 0, width, height);
                        blur.set_region(Some(&region));
                        blur.commit();
                        region.destroy();
                        info!("Applied org_kde_kwin_blur region: {width}x{height}");
                    }
                } else {
                    blur.commit();
                    info!("Cleared org_kde_kwin_blur region");
                }
                let _ = self.connection.flush();
            }
            SurfaceBlurHandler::None => {}
        }
    }

    pub fn status(&self) -> BlurEngineStatus {
        match self.protocol {
            BlurProtocol::ExtBackgroundEffectV1 | BlurProtocol::OrgKdeKwinBlur => {
                if self.enabled {
                    BlurEngineStatus::NativeActive {
                        protocol: self.protocol,
                        compositor: self.compositor_name.clone(),
                        details: format!(
                            "Blur region configured to {}x{} px. Native surface double-buffered commit.",
                            self.current_bounds.0, self.current_bounds.1
                        ),
                    }
                } else {
                    BlurEngineStatus::NativeDisabled {
                        protocol: self.protocol,
                        compositor: self.compositor_name.clone(),
                    }
                }
            }
            BlurProtocol::None => BlurEngineStatus::SimulatedFallback {
                reason: "Neither ext-background-effect-v1 nor org_kde_kwin_blur advertised"
                    .to_string(),
                compositor: self.compositor_name.clone(),
            },
        }
    }
}

impl Drop for WaylandBlurSession {
    fn drop(&mut self) {
        match &self.handler {
            SurfaceBlurHandler::ExtEffect { effect_surface } => {
                effect_surface.destroy();
            }
            SurfaceBlurHandler::KdeBlur { blur } => {
                blur.release();
            }
            SurfaceBlurHandler::None => {}
        }
        let _ = self.connection.flush();
    }
}

fn detect_compositor_name() -> String {
    // 1. Check niri socket or running process
    if std::env::var("NIRI_SOCKET").is_ok() {
        return "niri 26.04 (Wayland)".to_string();
    }
    // 2. Check XDG_CURRENT_DESKTOP
    if let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") {
        return format!("{desktop} (Wayland)");
    }
    // 3. Check WAYLAND_DISPLAY
    if let Ok(disp) = std::env::var("WAYLAND_DISPLAY") {
        return format!("Wayland Compositor ({disp})");
    }
    "Unknown Wayland Compositor".to_string()
}
