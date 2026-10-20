//! Slint panel beside the arm view's OpenGL context.
//!
//! MuJoCo draws the scene directly into the window's default framebuffer,
//! with the viewport narrowed to leave room for the panel. Slint then draws
//! only the panel, through the *same* GL context with
//! [`FemtoVGRenderer`](slint::platform::femtovg_renderer::FemtoVGRenderer).
//!
//! The scene is rendered from Slint's `BeforeRendering` notifier: femtovg has
//! already cleared the window at that point, but no item has been drawn yet.
//! There is no offscreen target and no full-screen composition pass.

// `slint_build`'s generated component items are `pub` inside this private
// module; they are not part of the crate's API.
#![allow(unreachable_pub)]

use std::cell::{Cell, RefCell};
use std::ffi::{CStr, CString, c_void};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};

use glow::HasContext as _;
use glutin::display::GlDisplay as _;
use glutin::surface::GlSurface as _;
use slint::platform::femtovg_renderer::{FemtoVGRenderer, OpenGLInterface};
use slint::platform::{Platform, WindowAdapter};
use slint::{PhysicalSize, PlatformError, Window};

use crate::armview::{ArmView, GlStack};

slint::include_modules!();

/// Width of the side panel in logical pixels; the rest of the window is the 3D
/// viewport.
const PANEL_WIDTH: f32 = 300.0;

/// The 3D viewport is kept at 4:3 by fixing the window size: Wayland has no
/// aspect-ratio hint, but tiling compositors honour a min == max size.
const SCENE_WIDTH: u32 = 960;
const SCENE_HEIGHT: u32 = 720;

/// Fixed arm view window size: the 4:3 viewport plus the panel (1260×720).
pub fn arm_view_window_size() -> winit::dpi::LogicalSize<f64> {
    winit::dpi::LogicalSize::new(
        SCENE_WIDTH as f64 + PANEL_WIDTH as f64,
        SCENE_HEIGHT as f64,
    )
}

/// Per-frame geometry: the window, and the 3D viewport inside it (the window
/// minus the panel on the right).
#[derive(Clone, Copy)]
struct Frame {
    window: PhysicalSize,
    scene: PhysicalSize,
}

/// The arm view's Slint layer: the side panel, its renderer, and the hook
/// that draws the scene into the window before Slint draws the panel.
pub(crate) struct ArmUi {
    ui: ArmViewUi,
    adapter: Rc<UiAdapter>,
    renderer: Rc<FemtoVGRenderer>,
    /// Read by the rendering notifier; written by [`Self::render`] before
    /// Slint starts the frame.
    frame: Rc<Cell<Frame>>,
    size: PhysicalSize,
}

/// The `OpenGLInterface` Slint renders through: a share of the arm view's GL
/// stack (`'static`, as the renderer keeps it for the window's lifetime).
struct GlGlue(Rc<GlStack>);

unsafe impl OpenGLInterface for GlGlue {
    fn ensure_current(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // The arm view makes this context current when it creates it and never
        // releases it on this thread, so it is current for every frame.
        Ok(())
    }

    fn swap_buffers(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.0
            .surface
            .swap_buffers(&self.0.context)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    fn resize(
        &self,
        width: NonZeroU32,
        height: NonZeroU32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.0.surface.resize(&self.0.context, width, height);
        Ok(())
    }

    fn get_proc_address(&self, name: &CStr) -> *const c_void {
        self.0.display.get_proc_address(name)
    }
}

struct UiAdapter {
    window: Window,
    renderer: Rc<FemtoVGRenderer>,
    size: Cell<PhysicalSize>,
    redraw: Cell<bool>,
}

impl WindowAdapter for UiAdapter {
    fn window(&self) -> &Window {
        &self.window
    }

    fn size(&self) -> PhysicalSize {
        self.size.get()
    }

    fn renderer(&self) -> &dyn slint::platform::Renderer {
        &*self.renderer
    }

    fn request_redraw(&self) {
        self.redraw.set(true);
    }
}

struct UiPlatform {
    adapter: Rc<UiAdapter>,
    start: Instant,
}

impl Platform for UiPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        Ok(self.adapter.clone())
    }

    fn duration_since_start(&self) -> Duration {
        self.start.elapsed()
    }
}

impl ArmUi {
    /// Build the UI on `gl` (the arm view's context) and hook the scene into
    /// Slint's frame. Fails when Slint's platform is already taken (a second
    /// arm view); the caller then falls back to rendering the scene directly.
    pub(crate) fn new(
        gl: Rc<GlStack>,
        view: Rc<RefCell<ArmView>>,
        size: PhysicalSize,
    ) -> Result<Self, PlatformError> {
        let renderer = Rc::new(FemtoVGRenderer::new(GlGlue(Rc::clone(&gl)))?);
        let adapter = Rc::new_cyclic(|weak| UiAdapter {
            window: Window::new(weak.clone()),
            renderer: Rc::clone(&renderer),
            size: Cell::new(size),
            redraw: Cell::new(true),
        });

        slint::platform::set_platform(Box::new(UiPlatform {
            adapter: Rc::clone(&adapter),
            start: Instant::now(),
        }))
        .map_err(|e| PlatformError::Other(e.to_string()))?;

        let ui = ArmViewUi::new()?;
        ui.set_panel_width(PANEL_WIDTH);
        ui.show()?;

        let glow = Rc::new(unsafe {
            glow::Context::from_loader_function(|name| {
                let name = CString::new(name).expect("GL symbol has no NUL");
                gl.display.get_proc_address(&name)
            })
        });
        let frame = Rc::new(Cell::new(Frame {
            window: size,
            scene: size,
        }));

        let (scene_view, scene_frame) = (Rc::clone(&view), Rc::clone(&frame));
        ui.window()
            .set_rendering_notifier(move |state, _api| {
                if !matches!(state, slint::RenderingState::BeforeRendering) {
                    return;
                }
                let frame = scene_frame.get();
                scene_view.borrow_mut().render_scene(winit::dpi::PhysicalSize::new(
                    frame.scene.width,
                    frame.scene.height,
                ));
                // femtovg sets its viewport once per frame, before the clear;
                // MuJoCo narrowed it for the scene, so put it back for the
                // panel.
                unsafe {
                    glow.viewport(
                        0,
                        0,
                        frame.window.width as i32,
                        frame.window.height as i32,
                    )
                };
            })
            .map_err(|e| PlatformError::Other(e.to_string()))?;

        Ok(Self {
            ui,
            adapter,
            renderer,
            frame,
            size,
        })
    }

    pub(crate) fn resize(&mut self, size: PhysicalSize) {
        if self.size == size {
            return;
        }
        self.size = size;
        self.adapter.size.set(size);
        self.ui
            .window()
            .dispatch_event(slint::platform::WindowEvent::Resized {
                size: slint::LogicalSize::new(size.width as f32, size.height as f32),
            });
    }

    /// Physical width of the 3D viewport: the window minus the panel.
    pub(crate) fn scene_width(&self) -> u32 {
        self.scene_size().width
    }

    /// Draw the scene and the panel as one frame, then swap.
    pub(crate) fn render(&mut self) -> Result<(), PlatformError> {
        self.frame.set(Frame {
            window: self.size,
            scene: self.scene_size(),
        });
        self.renderer.render()
    }

    fn scene_size(&self) -> PhysicalSize {
        PhysicalSize::new(
            self.size.width.saturating_sub(self.panel_width()).max(1),
            self.size.height.max(1),
        )
    }

    /// The panel's width in physical pixels, matching Slint's own layout.
    fn panel_width(&self) -> u32 {
        let scale = self.ui.window().scale_factor();
        (self.ui.get_panel_width() * scale).max(0.0).round() as u32
    }
}
