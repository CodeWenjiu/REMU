//! Lazily-started winit event-loop thread (the render backend), shared by the
//! display window and the armview monitor.
//!
//! The thread blits the framebuffer to the display window and renders the
//! armview monitor (MuJoCo GL via [`ArmWindow`]); it feeds display
//! window-size, mouse and keyboard state back into [`Shared`]. It is started
//! on first use: `wake()` (called by `frame_done`) requests a display redraw,
//! and `request_arm_view` (called by the arm backend's first use) opens the
//! monitor. winit allows one `EventLoop` per process, so both windows live
//! here.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use remu_state_mujoco::{ArmSnapshot, ArmWindow, ModelSource};

use crate::display::{DisplaySize, FB_HEIGHT, FB_WIDTH, fb_base};
use crate::keyboard::{KeyState, key_kind_from_winit, key_to_button};
use crate::mouse::{MouseState, button_bit};

/// Shared window state: resolution + mouse + keyboard. Written by the render
/// thread, read by the app via the device accessors.
pub(crate) struct Shared {
    pub(crate) disp: Mutex<DisplaySize>,
    pub(crate) mouse: Mutex<MouseState>,
    pub(crate) keyboard: Mutex<KeyState>,
}

impl Shared {
    fn new() -> Self {
        Self {
            disp: Mutex::new(DisplaySize {
                width: FB_WIDTH,
                height: FB_HEIGHT,
            }),
            mouse: Mutex::new(MouseState::default()),
            keyboard: Mutex::new(KeyState::default()),
        }
    }
}

/// Commands sent to the render thread by device code.
enum Wake {
    /// A guest frame is complete: (open and) redraw the display window.
    RedrawDisplay,
    /// The arm backend requested its monitor window (model source + snapshot
    /// are stored in `Backend::arm_request`).
    OpenArmView,
}

/// Lazily-created render backend (window thread + shared state).
struct Backend {
    shared: &'static Shared,
    /// Set once the display window is closed, so a later `frame_done` no-ops
    /// and `display_alive()` reports false.
    display_closed: AtomicBool,
    /// Whether the display window has been asked for (first frame/display use).
    display_wanted: AtomicBool,
    /// Set once the armview monitor is closed or fails to open: the app's
    /// `armview_alive()` reports false.
    arm_closed: AtomicBool,
    /// Pending armview request (model source + published-state handle).
    arm_request: Mutex<Option<(ModelSource, Arc<Mutex<ArmSnapshot>>)>>,
}

static BACKEND: OnceLock<Backend> = OnceLock::new();

/// Proxy used to wake the render thread from device code.
static PROXY: OnceLock<winit::event_loop::EventLoopProxy<Wake>> = OnceLock::new();

/// Get the render backend, starting the window thread on first use.
fn backend() -> &'static Backend {
    BACKEND.get_or_init(|| {
        let shared: &'static Shared = Box::leak(Box::new(Shared::new()));
        // Spawn the event loop on a background thread (Linux: any thread ok).
        std::thread::spawn(move || {
            if let Err(e) = render_loop(shared) {
                eprintln!("display backend error: {e}");
            }
            // Fallback: once the render thread exits, every window is gone —
            // mark them closed so `display_alive`/`armview_alive` reflect it
            // (covers exits that aren't a user `CloseRequested`).
            if let Some(b) = BACKEND.get() {
                b.display_closed.store(true, Ordering::Relaxed);
                b.arm_closed.store(true, Ordering::Relaxed);
            }
        });
        Backend {
            shared,
            display_closed: AtomicBool::new(false),
            display_wanted: AtomicBool::new(false),
            arm_closed: AtomicBool::new(false),
            arm_request: Mutex::new(None),
        }
    })
}

/// The shared window state, starting the backend on first use; any display
/// device use implies wanting the display window.
#[inline]
pub(crate) fn shared() -> &'static Shared {
    let backend = backend();
    backend.display_wanted.store(true, Ordering::Relaxed);
    backend.shared
}

/// Wake the render thread so it blits the updated framebuffer. The proxy is
/// registered in `render_loop`; if it isn't ready yet the send fails and we
/// just skip this frame.
#[inline]
pub(crate) fn wake() {
    let backend = backend();
    if backend.display_closed.load(Ordering::Relaxed) {
        return;
    }
    backend.display_wanted.store(true, Ordering::Relaxed);
    if let Some(proxy) = PROXY.get() {
        let _ = proxy.send_event(Wake::RedrawDisplay);
    }
}

/// Whether the display window is currently alive (not closed yet).
#[inline]
pub(crate) fn alive() -> bool {
    !backend().display_closed.load(Ordering::Relaxed)
}

/// Request the armview monitor window (called by the arm backend on first
/// use); it opens on the render thread and keeps rendering the snapshots the
/// arm core publishes. Pending (not yet open) counts as alive so an app
/// polling `armview_alive` cannot exit before the window had a chance.
pub(crate) fn request_arm_view(source: ModelSource, snapshot: Arc<Mutex<ArmSnapshot>>) {
    *backend().arm_request.lock().unwrap() = Some((source, snapshot));
    if let Some(proxy) = PROXY.get() {
        let _ = proxy.send_event(Wake::OpenArmView);
    }
}

/// Whether the armview monitor is alive: true until it is closed by the user
/// or fails to open.
#[inline]
pub(crate) fn armview_alive() -> bool {
    !backend().arm_closed.load(Ordering::Relaxed)
}

fn render_loop(shared: &'static Shared) -> Result<(), Box<dyn std::error::Error>> {
    use std::num::NonZeroU32;
    use std::rc::Rc;

    use softbuffer::{Context, Surface};
    use winit::application::ApplicationHandler;
    use winit::event::{ElementState, MouseButton, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle};
    use winit::platform::wayland::EventLoopBuilderExtWayland as _;
    use winit::window::{Window, WindowId};

    /// The display window: softbuffer presentation of the remu framebuffer.
    struct DisplayWindow {
        window: Rc<Window>,
        surface: Surface<OwnedDisplayHandle, Rc<Window>>,
    }

    struct App {
        context: Context<OwnedDisplayHandle>,
        shared: &'static Shared,
        /// The display window (None before it opens / once it was closed).
        display: Option<DisplayWindow>,
        /// The armview monitor: GL renderer of the arm device's snapshots.
        arm: Option<ArmWindow>,
        arm_window: Option<Rc<Window>>,
        /// Whether any window ever opened (the loop exits once all are gone).
        had_window: bool,
        /// Whether `resumed` ran (window creation is only legal after it).
        started: bool,
    }

    impl App {
        /// Update the shared display resolution from the current window size.
        fn update_disp(&self) {
            let (wl, wh) = self
                .display
                .as_ref()
                .map(|d| {
                    let s = d.window.scale_factor();
                    let inner = d.window.inner_size();
                    (
                        (inner.width as f64 / s) as usize,
                        (inner.height as f64 / s) as usize,
                    )
                })
                .unwrap_or((FB_WIDTH, FB_HEIGHT));
            let mut d = self.shared.disp.lock().unwrap();
            d.width = wl.clamp(1, FB_WIDTH);
            d.height = wh.clamp(1, FB_HEIGHT);
        }

        /// Current display size (logical pixels), clamped to the framebuffer.
        fn disp_size(&self) -> (usize, usize) {
            let d = self.shared.disp.lock().unwrap();
            (d.width, d.height)
        }

        /// Open the display window once a frame wants to be shown.
        fn maybe_open_display(&mut self, event_loop: &ActiveEventLoop) {
            if !self.started || self.display.is_some() {
                return;
            }
            let wanted = BACKEND
                .get()
                .map(|b| b.display_wanted.load(Ordering::Relaxed))
                .unwrap_or(false);
            if !wanted {
                return;
            }
            let attrs = Window::default_attributes()
                .with_title("remu display")
                // Start with a modest window; the framebuffer is much larger.
                .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
                // Pin min=max so tiling compositors (e.g. niri) cannot
                // stretch the window to the tile size.
                .with_min_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
                .with_max_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0));
            let window = Rc::new(event_loop.create_window(attrs).expect("create window"));
            let surface = Surface::new(&self.context, window.clone()).expect("create surface");
            self.display = Some(DisplayWindow { window, surface });
            self.update_disp();
            let (dw, dh) = self.disp_size();
            if let Some(d) = self.display.as_mut() {
                let _ = d.surface.resize(
                    NonZeroU32::new(dw.max(1) as u32).unwrap(),
                    NonZeroU32::new(dh.max(1) as u32).unwrap(),
                );
            }
            self.had_window = true;
        }

        /// Open the armview monitor if the arm backend requested it.
        fn maybe_open_armview(&mut self, event_loop: &ActiveEventLoop) {
            if !self.started || self.arm.is_some() {
                return;
            }
            let Some((source, snapshot)) = backend().arm_request.lock().unwrap().take() else {
                return;
            };
            let attrs = Window::default_attributes()
                .with_title("remu armview")
                .with_inner_size(winit::dpi::LogicalSize::new(900.0, 700.0))
                .with_min_inner_size(winit::dpi::LogicalSize::new(320.0, 240.0));
            let window = Rc::new(event_loop.create_window(attrs).expect("create window"));
            match ArmWindow::new(&window, &source, snapshot) {
                Ok(arm) => {
                    self.arm = Some(arm);
                    self.arm_window = Some(window);
                    self.had_window = true;
                    backend().arm_closed.store(false, Ordering::Relaxed);
                }
                // No empty window is left behind: the dropped window closes.
                Err(e) => {
                    eprintln!("armview: window backend failed: {e}");
                    backend().arm_closed.store(true, Ordering::Relaxed);
                }
            }
        }

        /// Display window events: blit + input fan-out into [`Shared`].
        fn display_event(&mut self, event: WindowEvent) {
            match event {
                WindowEvent::CloseRequested => {
                    // The user closed the display window: it's gone for the app
                    // (`display_alive()` reports false); the loop keeps running
                    // for any other windows.
                    self.display = None;
                    backend().display_closed.store(true, Ordering::Relaxed);
                }
                WindowEvent::Resized(_physical) => {
                    self.update_disp();
                    let (dw, dh) = self.disp_size();
                    if let Some(d) = self.display.as_mut() {
                        let _ = d.surface.resize(
                            NonZeroU32::new(dw.max(1) as u32).unwrap(),
                            NonZeroU32::new(dh.max(1) as u32).unwrap(),
                        );
                    }
                }
                WindowEvent::ScaleFactorChanged { .. } => self.update_disp(),
                WindowEvent::RedrawRequested => {
                    let (dw, dh) = self.disp_size();
                    if let Some(d) = self.display.as_mut() {
                        blit(&mut d.surface, dw, dh);
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let (wl, wh) = self
                        .display
                        .as_ref()
                        .map(|d| {
                            let s = d.window.scale_factor();
                            let inner = d.window.inner_size();
                            (inner.width as f64 / s, inner.height as f64 / s)
                        })
                        .unwrap_or((FB_WIDTH as f64, FB_HEIGHT as f64));
                    let logical: winit::dpi::LogicalPosition<f64> = position.to_logical(
                        self.display
                            .as_ref()
                            .map(|d| d.window.scale_factor())
                            .unwrap_or(1.0),
                    );
                    let (dw, dh) = self.disp_size();
                    let mut m = self.shared.mouse.lock().unwrap();
                    m.x = if wl > 0.0 {
                        (logical.x / wl * dw as f64).clamp(0.0, dw as f64 - 1.0) as usize
                    } else {
                        0
                    };
                    m.y = if wh > 0.0 {
                        (logical.y / wh * dh as f64).clamp(0.0, dh as f64 - 1.0) as usize
                    } else {
                        0
                    };
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    let bit = button_bit(button);
                    let mut m = self.shared.mouse.lock().unwrap();
                    match state {
                        ElementState::Pressed => m.buttons |= bit,
                        ElementState::Released => m.buttons &= !bit,
                    }
                }
                // ── Keyboard input → shared state. ──
                WindowEvent::KeyboardInput { event, .. } => {
                    use winit::event::KeyEvent;
                    use winit::keyboard::PhysicalKey;
                    let KeyEvent {
                        physical_key,
                        logical_key,
                        state,
                        text,
                        ..
                    } = event;
                    let code = match physical_key {
                        PhysicalKey::Code(c) => c as u32,
                        PhysicalKey::Unidentified(_) => 0,
                    };
                    // Model non-printable keys with the `key_kind` encoding;
                    // printable keys keep their character in `text`. Consumers
                    // (e.g. the Slint adapter) map `key_kind` to their own key
                    // encoding.
                    let key_kind = key_kind_from_winit(logical_key);
                    let mut kb = self.shared.keyboard.lock().unwrap();
                    kb.code = code;
                    kb.down = matches!(state, ElementState::Pressed);
                    kb.text = text
                        .and_then(|t| t.chars().next())
                        .map(|c| c as u32)
                        .unwrap_or(0);
                    kb.key_kind = key_kind;
                    // Only count presses; a release leaves `seq` unchanged so
                    // the snapshot reader sees exactly one event per tap.
                    if kb.down {
                        kb.seq = kb.seq.wrapping_add(1);
                    }
                    kb.valid = true;
                    // Maintain the live joypad button bitmask so apps polling at a
                    // low frame rate still see held keys.
                    let bit = key_to_button(code, kb.text);
                    if bit != 0 {
                        if kb.down {
                            kb.buttons |= bit;
                        } else {
                            kb.buttons &= !bit;
                        }
                    }
                }
                _ => {}
            }
        }

        /// Armview events: camera interaction + per-frame render.
        fn arm_event(&mut self, event: WindowEvent) {
            match event {
                WindowEvent::CloseRequested => {
                    // Drop the GL stack first: it must not outlive its window.
                    self.arm = None;
                    self.arm_window = None;
                    // The monitor is gone: the app's `armview_alive()` reports
                    // false so it can stop instead of controlling invisibly.
                    backend().arm_closed.store(true, Ordering::Relaxed);
                }
                WindowEvent::Resized(size) => {
                    if let Some(arm) = self.arm.as_mut() {
                        arm.resize(size);
                    }
                }
                WindowEvent::RedrawRequested => {
                    if let Some(arm) = self.arm.as_mut() {
                        arm.redraw();
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    if let Some(arm) = self.arm.as_mut() {
                        arm.cursor_moved(position.x, position.y);
                    }
                }
                WindowEvent::MouseInput {
                    state,
                    button: MouseButton::Left,
                    ..
                } => {
                    if let Some(arm) = self.arm.as_mut() {
                        arm.set_left(matches!(state, ElementState::Pressed));
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    if let Some(arm) = self.arm.as_mut() {
                        arm.wheel(delta);
                    }
                }
                _ => {}
            }
        }
    }

    impl ApplicationHandler<Wake> for App {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            if self.started {
                return;
            }
            self.started = true;
            // Requests recorded before the loop was up are served here; later
            // ones arrive as user events.
            self.maybe_open_display(event_loop);
            self.maybe_open_armview(event_loop);
        }

        fn window_event(
            &mut self,
            _event_loop: &ActiveEventLoop,
            id: WindowId,
            event: WindowEvent,
        ) {
            if Some(id) == self.display.as_ref().map(|d| d.window.id()) {
                self.display_event(event);
            } else if Some(id) == self.arm_window.as_ref().map(|w| w.id()) {
                self.arm_event(event);
            }
        }

        fn user_event(&mut self, event_loop: &ActiveEventLoop, wake: Wake) {
            match wake {
                Wake::RedrawDisplay => {
                    self.maybe_open_display(event_loop);
                    if let Some(d) = &self.display {
                        d.window.request_redraw();
                    }
                }
                Wake::OpenArmView => self.maybe_open_armview(event_loop),
            }
        }

        fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
            // Requests that raced the loop startup (cheap no-ops otherwise).
            self.maybe_open_display(event_loop);
            self.maybe_open_armview(event_loop);
            if let Some(arm) = &self.arm_window {
                // Continuous armview animation: one frame per wakeup; the EGL
                // swap (vsync) paces the loop.
                arm.request_redraw();
                event_loop.set_control_flow(ControlFlow::Poll);
            } else if self.had_window && self.display.is_none() {
                // Every window that ever opened is closed now: end the loop.
                event_loop.exit();
            } else {
                event_loop.set_control_flow(ControlFlow::Wait);
            }
        }
    }

    /// Blit the host framebuffer to the display window.
    fn blit(surface: &mut Surface<OwnedDisplayHandle, Rc<Window>>, disp_w: usize, disp_h: usize) {
        if let Ok(mut buffer) = surface.buffer_mut() {
            let w = disp_w.min(FB_WIDTH);
            let h = disp_h.min(FB_HEIGHT);
            let buf_w = buffer.width().get() as usize;
            let buf_h = buffer.height().get() as usize;
            let cw = w.min(buf_w);
            let ch = h.min(buf_h);
            let src = fb_base() as *const u32;
            let dst = buffer.as_mut_ptr();
            for row in 0..ch {
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        src.add(row * FB_WIDTH),
                        dst.add(row * buf_w),
                        cw,
                    );
                }
            }
            let _ = buffer.present();
        }
    }

    let event_loop = EventLoop::<Wake>::with_user_event()
        .with_any_thread(true) // background thread is fine on Linux
        .build()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    // Register the proxy before the loop runs so device code can wake us.
    let _ = PROXY.set(event_loop.create_proxy());
    let context = Context::new(event_loop.owned_display_handle())?;
    let mut app = App {
        context,
        shared,
        display: None,
        arm: None,
        arm_window: None,
        had_window: false,
        started: false,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
