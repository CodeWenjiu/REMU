//! Shared winit window host backing the display / mouse / keyboard devices.
//!
//! All remu windows share one winit event loop on one render thread (winit
//! only allows a single `EventLoop` per process). Windows open on demand: the
//! display window when a framebuffer-backed display device attaches, and the
//! armview monitor window when the arm device requests it. The display
//! window's events are fanned out into three independent shared states:
//!
//! - `disp`   — active display resolution (written by resize, read by display)
//! - `mouse`  — pointer position + buttons (written by winit, read by mouse)
//! - `keyboard` — keycode + state + modifiers (written by winit, read by keyboard)
//!
//! The framebuffer itself is **not** owned here: it lives in `Bus::Memory`, and
//! the render thread reads it through a raw pointer (set via `set_fb`). No locks
//! on the framebuffer path — the guest writes it directly, the render thread
//! reads it at frame boundaries.
//!
//! Startup is lazy: the host is only created when a window-backed device is
//! instantiated, so a run that never needs a window never opens one.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use remu_state_mujoco::{ArmSnapshot, ArmWindow, ModelSource};

use super::keyboard::KeyboardState;
use super::mouse::MouseState;

/// Framebuffer capacity (matches the display device).
pub(super) const FB_WIDTH: usize = 2048;
pub(super) const FB_HEIGHT: usize = 2048;

/// Shared active display resolution, written by the render thread on resize and
/// read by the display device via MMIO.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct DisplayState {
    pub disp_w: usize,
    pub disp_h: usize,
}

impl DisplayState {
    pub(super) fn initial() -> Self {
        Self {
            disp_w: FB_WIDTH,
            disp_h: FB_HEIGHT,
        }
    }
}

/// Commands sent to the render thread by device threads.
enum HostCommand {
    /// A guest frame is complete: redraw the display window.
    RedrawDisplay,
    /// A display device attached its framebuffer: open the display window
    /// (no-op when already open).
    OpenDisplay,
    /// The arm device requested its monitor window: open armview (no-op when
    /// already open). The request itself (xml + snapshot) is stored in
    /// `WindowHost::arm_request`.
    OpenArmView,
    /// Teardown: exit the event loop (wakes it out of its blocking wait).
    Shutdown,
}

/// A pending armview request: which model to mirror and where the arm device
/// publishes its state.
struct ArmViewRequest {
    source: ModelSource,
    snapshot: Arc<Mutex<ArmSnapshot>>,
}

/// The window host: owns the winit event loop thread and fans out input events
/// into shared states read by the display / mouse / keyboard devices. Its
/// lifetime is tied to the owning `Bus` (each DUT bus holds an `Arc`); the
/// render thread is joined when the last reference is dropped.
pub(crate) struct WindowHost {
    /// Raw framebuffer pointer into `Bus::Memory` (set by display's
    /// `attach_mem_region`). No locking on the hot path.
    fb: Mutex<Option<usize>>,
    /// Shared display resolution (render thread writes, display reads).
    disp: Arc<Mutex<DisplayState>>,
    /// Shared mouse state (render thread writes, mouse device reads).
    pub(super) mouse: Arc<Mutex<MouseState>>,
    /// Shared keyboard state (render thread writes, keyboard device reads).
    pub(super) keyboard: Arc<Mutex<KeyboardState>>,
    /// Pending armview request, taken by the loop thread when it opens the
    /// window; recorded before the loop thread is guaranteed to be listening.
    arm_request: Mutex<Option<ArmViewRequest>>,
    /// Whether the armview monitor window is (still) expected to be open
    /// (false once the user closed it, or opening it failed). Read by the arm
    /// device's status register.
    armview_alive: AtomicBool,
    /// Proxy used to wake the render thread on frame-done.
    proxy: Mutex<Option<winit::event_loop::EventLoopProxy<HostCommand>>>,
    /// Signal for the render thread to stop.
    stop: Arc<AtomicBool>,
    /// Whether the display window is open (false after the user closes it).
    alive: AtomicBool,
    /// Render thread handle.
    thread: Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl WindowHost {
    /// Create a window host and start its render thread.
    pub(crate) fn new() -> Arc<WindowHost> {
        let host = Arc::new(WindowHost {
            fb: Mutex::new(None),
            disp: Arc::new(Mutex::new(DisplayState::initial())),
            mouse: Arc::new(Mutex::new(MouseState::default())),
            keyboard: Arc::new(Mutex::new(KeyboardState::default())),
            arm_request: Mutex::new(None),
            armview_alive: AtomicBool::new(true),
            proxy: Mutex::new(None),
            stop: Arc::new(AtomicBool::new(false)),
            alive: AtomicBool::new(true),
            thread: Mutex::new(None),
        });
        let host_for_thread = Arc::clone(&host);
        let handle = std::thread::spawn(move || {
            let ok = render_loop(&host_for_thread);
            // Render thread exited (loop ended): mark not alive so devices can
            // observe the window is gone.
            host_for_thread.alive.store(false, Ordering::Relaxed);
            host_for_thread
                .armview_alive
                .store(false, Ordering::Relaxed);
            if let Err(e) = ok {
                eprintln!("window backend error: {e}");
            }
        });
        *host.thread.lock().unwrap() = Some(handle);
        host
    }

    /// Set the framebuffer pointer (called by display's `attach_mem_region`).
    pub(crate) fn set_fb(&self, ptr: *mut u8) {
        *self.fb.lock().unwrap() = Some(ptr as usize);
        // The display window opens on demand (first framebuffer): wake the
        // loop in case it started before this device attached.
        if let Some(proxy) = self.proxy.lock().unwrap().as_ref() {
            let _ = proxy.send_event(HostCommand::OpenDisplay);
        }
    }

    /// Raw framebuffer pointer for the render thread.
    pub(super) fn fb_addr(&self) -> Option<usize> {
        *self.fb.lock().unwrap()
    }

    pub(super) fn disp(&self) -> &Arc<Mutex<DisplayState>> {
        &self.disp
    }

    pub(super) fn mouse(&self) -> &Arc<Mutex<MouseState>> {
        &self.mouse
    }

    pub(super) fn keyboard(&self) -> &Arc<Mutex<KeyboardState>> {
        &self.keyboard
    }

    /// Wake the render thread to blit a finished frame (from display's CTRL write).
    pub(crate) fn request_display_redraw(&self) {
        if let Some(proxy) = self.proxy.lock().unwrap().as_ref() {
            let _ = proxy.send_event(HostCommand::RedrawDisplay);
        }
    }

    /// Queue the armview monitor window (called by the arm device): it opens
    /// once on the loop thread and renders whatever the device publishes into
    /// `snapshot`.
    pub(crate) fn open_arm_view(&self, source: ModelSource, snapshot: Arc<Mutex<ArmSnapshot>>) {
        *self.arm_request.lock().unwrap() = Some(ArmViewRequest { source, snapshot });
        if let Some(proxy) = self.proxy.lock().unwrap().as_ref() {
            let _ = proxy.send_event(HostCommand::OpenArmView);
        }
    }

    /// Whether the display window is currently open.
    pub(crate) fn alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    /// Whether the armview monitor window is currently open.
    pub(crate) fn armview_alive(&self) -> bool {
        self.armview_alive.load(Ordering::Relaxed)
    }

    /// Signal the render thread to stop and join it (closes the windows).
    /// Safe to call more than once; idempotent. Used by `Bus::drop` so the
    /// window closes with the bus rather than lingering to process end.
    pub(crate) fn request_shutdown(&self) {
        self.stop.store(true, Ordering::Relaxed);
        // Wake the loop out of its blocking `Wait` so it observes the stop.
        if let Some(proxy) = self.proxy.lock().unwrap().as_ref() {
            let _ = proxy.send_event(HostCommand::Shutdown);
        }
        if let Some(h) = self.thread.lock().unwrap().take() {
            let _ = h.join();
        }
    }
}

impl Drop for WindowHost {
    fn drop(&mut self) {
        self.request_shutdown();
    }
}

// ── Render backend (softbuffer + winit event loop) ──

fn render_loop(host: &Arc<WindowHost>) -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::HashMap;
    use std::num::NonZeroU32;
    use std::rc::Rc;

    use softbuffer::{Context, Surface};
    use winit::application::ApplicationHandler;
    use winit::event::{ElementState, MouseButton, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle};
    use winit::platform::wayland::EventLoopBuilderExtWayland as _;
    use winit::window::{Window, WindowId};

    /// What a managed window renders and which inputs it owns.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum WindowKind {
        /// The device framebuffer window (softbuffer blit of `Bus::Memory`);
        /// its input events feed the guest mouse/keyboard devices.
        Display,
        /// The MuJoCo arm monitor: an OpenGL window rendered by the armview
        /// backend from the arm device's published snapshots.
        Armview,
    }

    /// A window managed by the shared loop. `surface` is the softbuffer
    /// presentation used by the display window only.
    struct ManagedWindow {
        kind: WindowKind,
        window: Rc<Window>,
        surface: Option<Surface<OwnedDisplayHandle, Rc<Window>>>,
    }

    struct App {
        context: Context<OwnedDisplayHandle>,
        host: Arc<WindowHost>,
        /// The armview monitor; drops before `windows`, which owns its window
        /// (the GL stack must never outlive the window it renders into).
        arm: Option<ArmWindow>,
        arm_id: Option<WindowId>,
        windows: HashMap<WindowId, ManagedWindow>,
        /// Window id of the display window (`None` once the user closed it).
        display: Option<WindowId>,
        /// Whether `resumed` ran (window creation is only legal after it).
        started: bool,
    }
    // `host` owns the raw framebuffer pointer, only read while Bus/Memory
    // outlives the render thread. `Send` is safe: the pointer is not
    // dereferenced after Memory is gone (WindowHost drops before Memory).
    unsafe impl Send for App {}

    /// Convert a winit button to our button-bitmask.
    fn button_bit(b: MouseButton) -> u32 {
        match b {
            MouseButton::Left => 1,
            MouseButton::Right => 2,
            MouseButton::Middle => 4,
            MouseButton::Back | MouseButton::Forward | MouseButton::Other(_) => 0,
        }
    }

    /// Map a winit key event (physical code + text) to a NES joypad button
    /// bit, or 0. Mirrors the app-side mapping in `input.rs`.
    fn key_to_button(code: u32, text: u32) -> u8 {
        match text as u8 as char {
            'z' | 'Z' => 1 << 0, // A
            'x' | 'X' => 1 << 1, // B
            'q' | 'Q' => 1 << 2, // SELECT
            'w' | 'W' => 1 << 3, // START
            // Vim-style d-pad (plus physical arrows below).
            'h' | 'H' => 1 << 6, // LEFT
            'j' | 'J' => 1 << 5, // DOWN
            'k' | 'K' => 1 << 4, // UP
            'l' | 'L' => 1 << 7, // RIGHT
            _ => match code {
                82 => 1 << 4, // UP
                79 => 1 << 5, // DOWN
                80 => 1 << 6, // LEFT
                81 => 1 << 7, // RIGHT
                _ => 0,
            },
        }
    }

    impl App {
        /// Update the shared display resolution from the display window size.
        fn update_disp(&self) {
            let (wl, wh) = self
                .display
                .and_then(|id| self.windows.get(&id))
                .map(|w| {
                    let s = w.window.scale_factor();
                    let inner = w.window.inner_size();
                    (
                        (inner.width as f64 / s) as usize,
                        (inner.height as f64 / s) as usize,
                    )
                })
                .unwrap_or((FB_WIDTH, FB_HEIGHT));
            let mut d = self.host.disp.lock().unwrap();
            d.disp_w = wl.min(FB_WIDTH).max(1);
            d.disp_h = wh.min(FB_HEIGHT).max(1);
        }

        /// Open the display window if a framebuffer-backed display device is
        /// present and the window is not open yet.
        fn maybe_open_display(&mut self, event_loop: &ActiveEventLoop) {
            if !self.started || self.display.is_some() || self.host.fb_addr().is_none() {
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
            self.display = Some(window.id());
            self.windows.insert(
                window.id(),
                ManagedWindow {
                    kind: WindowKind::Display,
                    window,
                    surface: Some(surface),
                },
            );
            self.update_disp();
            let (dw, dh) = {
                let d = self.host.disp.lock().unwrap();
                (d.disp_w, d.disp_h)
            };
            if let Some(w) = self.display.and_then(|id| self.windows.get_mut(&id))
                && let Some(surface) = w.surface.as_mut()
            {
                let _ = surface.resize(
                    NonZeroU32::new(dw.max(1) as u32).unwrap(),
                    NonZeroU32::new(dh.max(1) as u32).unwrap(),
                );
            }
        }

        /// Open the armview monitor window if the arm device requested it and
        /// it is not open yet.
        fn maybe_open_armview(&mut self, event_loop: &ActiveEventLoop) {
            if !self.started || self.arm.is_some() {
                return;
            }
            let Some(request) = self.host.arm_request.lock().unwrap().take() else {
                return;
            };
            let attrs = Window::default_attributes()
                .with_title("remu armview")
                .with_inner_size(winit::dpi::LogicalSize::new(900.0, 700.0))
                .with_min_inner_size(winit::dpi::LogicalSize::new(320.0, 240.0));
            let window = Rc::new(event_loop.create_window(attrs).expect("create window"));
            match ArmWindow::new(&window, &request.source, request.snapshot) {
                Ok(arm) => {
                    self.arm = Some(arm);
                    self.arm_id = Some(window.id());
                    self.windows.insert(
                        window.id(),
                        ManagedWindow {
                            kind: WindowKind::Armview,
                            window,
                            surface: None,
                        },
                    );
                }
                // No empty window is left behind: the dropped window closes.
                Err(e) => {
                    eprintln!("armview: window backend failed: {e}");
                    self.host.armview_alive.store(false, Ordering::Relaxed);
                }
            }
        }
    }

    impl ApplicationHandler<HostCommand> for App {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            if self.started {
                return;
            }
            self.started = true;
            // Requests recorded before the loop was up are opened here; later
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
            let Some(kind) = self.windows.get(&id).map(|w| w.kind) else {
                return;
            };
            match kind {
                WindowKind::Display => self.display_event(id, event),
                WindowKind::Armview => self.armview_event(id, event),
            }
        }

        fn user_event(&mut self, event_loop: &ActiveEventLoop, command: HostCommand) {
            match command {
                // A frame is complete (from display's CTRL write): redraw.
                HostCommand::RedrawDisplay => {
                    if let Some(w) = self.display.and_then(|id| self.windows.get(&id)) {
                        w.window.request_redraw();
                    }
                }
                HostCommand::OpenDisplay => self.maybe_open_display(event_loop),
                HostCommand::OpenArmView => self.maybe_open_armview(event_loop),
                HostCommand::Shutdown => event_loop.exit(),
            }
        }

        fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
            if self.host.stop.load(Ordering::Relaxed) {
                event_loop.exit();
                return;
            }
            // Requests that raced the loop startup (cheap no-ops otherwise).
            self.maybe_open_display(event_loop);
            self.maybe_open_armview(event_loop);
            if self.arm.is_some() {
                // Continuous armview animation: one frame per wakeup; the EGL
                // swap (vsync) paces the loop.
                if let Some(id) = self.arm_id
                    && let Some(w) = self.windows.get(&id)
                {
                    w.window.request_redraw();
                }
                event_loop.set_control_flow(ControlFlow::Poll);
            } else {
                event_loop.set_control_flow(ControlFlow::Wait);
            }
        }
    }

    impl App {
        fn display_event(&mut self, id: WindowId, event: WindowEvent) {
            match event {
                WindowEvent::CloseRequested => {
                    // The user closed the display window: it's gone for the
                    // guest (`display_alive()` reports false), but the shared
                    // loop keeps running for any other windows.
                    self.windows.remove(&id);
                    self.display = None;
                    self.host.alive.store(false, Ordering::Relaxed);
                }
                WindowEvent::Resized(_physical) => {
                    self.update_disp();
                    let (dw, dh) = {
                        let d = self.host.disp.lock().unwrap();
                        (d.disp_w, d.disp_h)
                    };
                    if let Some(w) = self.windows.get_mut(&id)
                        && let Some(surface) = w.surface.as_mut()
                    {
                        let _ = surface.resize(
                            NonZeroU32::new(dw.max(1) as u32).unwrap(),
                            NonZeroU32::new(dh.max(1) as u32).unwrap(),
                        );
                    }
                }
                WindowEvent::ScaleFactorChanged { .. } => self.update_disp(),
                WindowEvent::RedrawRequested => {
                    let fb = self.host.fb_addr().unwrap_or(0) as *const u32;
                    let (dw, dh) = {
                        let d = self.host.disp.lock().unwrap();
                        (d.disp_w, d.disp_h)
                    };
                    if let Some(w) = self.windows.get_mut(&id)
                        && let Some(surface) = w.surface.as_mut()
                    {
                        blit(fb, &self.host.stop, surface, dw, dh);
                    }
                }
                // ── Mouse input → shared state. ──
                WindowEvent::CursorMoved { position, .. } => {
                    let (wl, wh) = self
                        .windows
                        .get(&id)
                        .map(|w| {
                            let s = w.window.scale_factor();
                            let inner = w.window.inner_size();
                            (inner.width as f64 / s, inner.height as f64 / s)
                        })
                        .unwrap_or((FB_WIDTH as f64, FB_HEIGHT as f64));
                    let logical: winit::dpi::LogicalPosition<f64> = position.to_logical(
                        self.windows
                            .get(&id)
                            .map(|w| w.window.scale_factor())
                            .unwrap_or(1.0),
                    );
                    let (dw, dh) = {
                        let d = self.host.disp.lock().unwrap();
                        (d.disp_w, d.disp_h)
                    };
                    let mut m = self.host.mouse.lock().unwrap();
                    m.x = if wl > 0.0 {
                        (logical.x / wl * dw as f64).clamp(0.0, dw as f64 - 1.0) as i32
                    } else {
                        0
                    };
                    m.y = if wh > 0.0 {
                        (logical.y / wh * dh as f64).clamp(0.0, dh as f64 - 1.0) as i32
                    } else {
                        0
                    };
                    m.valid = true;
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    let bit = button_bit(button);
                    let mut m = self.host.mouse.lock().unwrap();
                    match state {
                        ElementState::Pressed => m.buttons |= bit,
                        ElementState::Released => m.buttons &= !bit,
                    }
                }
                // ── Keyboard input → shared state. ──
                WindowEvent::KeyboardInput { event, .. } => {
                    use super::keyboard::KeyKind;
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
                    // Model non-printable keys with a UI-agnostic `KeyKind`;
                    // printable keys keep their character in `text`. The
                    // app-side adapter (remu_hal) decodes the register with the
                    // matching `KeyKind` mapping.
                    let key_kind = KeyKind::from(logical_key);
                    let mut kb = self.host.keyboard.lock().unwrap();
                    kb.code = code;
                    kb.down = matches!(state, ElementState::Pressed);
                    kb.text = text
                        .and_then(|t| t.chars().next())
                        .map(|c| c as u32)
                        .unwrap_or(0);
                    kb.key_kind = key_kind as u32;
                    // Only count presses; a release leaves `seq` unchanged so
                    // the snapshot reader sees exactly one event per tap.
                    if kb.down {
                        kb.seq = kb.seq.wrapping_add(1);
                    }
                    kb.valid = true;
                    // Maintain the live joypad button bitmask so apps polling at a
                    // low frame rate still see held keys (fast press+release that
                    // fits between two polls is kept because the release clears the
                    // bit, not the whole event).
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

        /// Armview window events: close tears the GL stack down, redraw pulls
        /// the latest snapshot, and the mouse drives the free camera.
        fn armview_event(&mut self, id: WindowId, event: WindowEvent) {
            match event {
                WindowEvent::CloseRequested => {
                    // Drop the GL stack first: it must not outlive its window.
                    self.arm = None;
                    self.arm_id = None;
                    self.windows.remove(&id);
                    // The monitor is gone: the arm device's status register
                    // tells the guest (its `armview_alive()` reads false).
                    self.host.armview_alive.store(false, Ordering::Relaxed);
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

    /// Blit the framebuffer to the window. `fb` is the raw framebuffer pointer,
    /// `stop` the teardown flag.
    fn blit(
        fb: *const u32,
        stop: &Arc<AtomicBool>,
        surface: &mut Surface<OwnedDisplayHandle, Rc<Window>>,
        disp_w: usize,
        disp_h: usize,
    ) {
        // Don't touch the framebuffer once stop is set: the main thread may be
        // tearing down `Memory` (freeing the region) right after.
        if stop.load(Ordering::Relaxed) || fb.is_null() {
            return;
        }
        if let Ok(mut buffer) = surface.buffer_mut() {
            let w = disp_w.min(FB_WIDTH);
            let h = disp_h.min(FB_HEIGHT);
            let buf_w = buffer.width().get() as usize;
            let buf_h = buffer.height().get() as usize;
            let cw = w.min(buf_w);
            let ch = h.min(buf_h);
            let dst = buffer.as_mut_ptr();
            for row in 0..ch {
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        fb.add(row * FB_WIDTH),
                        dst.add(row * buf_w),
                        cw,
                    );
                }
            }
            let _ = buffer.present();
        }
    }

    let event_loop = EventLoop::<HostCommand>::with_user_event()
        .with_any_thread(true) // allow creating the loop on a background thread
        .build()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    // Register the proxy before the loop runs so display's CTRL write can wake us.
    *host.proxy.lock().unwrap() = Some(event_loop.create_proxy());
    let context = Context::new(event_loop.owned_display_handle())?;
    let mut app = App {
        context,
        host: Arc::clone(host),
        arm: None,
        arm_id: None,
        windows: HashMap::new(),
        display: None,
        started: false,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
