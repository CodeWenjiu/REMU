//! Shared winit window host backing the display / mouse / keyboard devices.
//!
//! A single winit event loop + window is created once (when any of the three
//! devices needs it) and its events are fanned out into three independent
//! shared states:
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
//! Startup is lazy via [`ensure_started`], so a run that never instantiates a
//! window-backed device never opens a window.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

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
    /// Proxy used to wake the render thread on frame-done.
    proxy: Arc<Mutex<Option<winit::event_loop::EventLoopProxy<()>>>>,
    /// Signal for the render thread to stop.
    stop: Arc<AtomicBool>,
    /// Whether the window is alive (set false when the render thread exits).
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
            proxy: Arc::new(Mutex::new(None)),
            stop: Arc::new(AtomicBool::new(false)),
            alive: AtomicBool::new(true),
            thread: Mutex::new(None),
        });
        let host_for_thread = Arc::clone(&host);
        let handle = std::thread::spawn(move || {
            let ok = render_loop(&host_for_thread);
            // Render thread exited (window closed or loop ended): mark not alive
            // so devices can observe the window is gone.
            host_for_thread.alive.store(false, Ordering::Relaxed);
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
    pub(crate) fn request_redraw(&self) {
        if let Some(proxy) = self.proxy.lock().unwrap().as_ref() {
            let _ = proxy.send_event(());
        }
    }

    /// Whether the window is currently alive (render thread running).
    pub(crate) fn alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    /// Signal the render thread to stop and join it (closes the window).
    /// Safe to call more than once; idempotent. Used by `Bus::drop` so the
    /// window closes with the bus rather than lingering to process end.
    pub(crate) fn request_shutdown(&self) {
        self.stop.store(true, Ordering::Relaxed);
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
    use std::num::NonZeroU32;
    use std::rc::Rc;

    use softbuffer::{Context, Surface};
    use winit::application::ApplicationHandler;
    use winit::event::{ElementState, MouseButton, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle};
    use winit::platform::wayland::EventLoopBuilderExtWayland as _;
    use winit::window::{Window, WindowId};

    enum AppState {
        Initial,
        Running {
            surface: Surface<OwnedDisplayHandle, Rc<Window>>,
        },
    }

    struct App {
        context: Context<OwnedDisplayHandle>,
        state: AppState,
        host: Arc<WindowHost>,
        window: Option<Rc<Window>>,
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
        /// Update the shared display resolution from the current window size.
        fn update_disp(&self) {
            let (wl, wh) = self
                .window
                .as_ref()
                .map(|w| {
                    let s = w.scale_factor();
                    let inner = w.inner_size();
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
    }

    impl ApplicationHandler<()> for App {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            if matches!(self.state, AppState::Initial) {
                let attrs = Window::default_attributes()
                    .with_title("remu display")
                    // Start with a modest window; the framebuffer is much larger.
                    .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
                    // Pin min=max so tiling compositors (e.g. niri) cannot
                    // stretch the window to the tile size.
                    .with_min_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
                    .with_max_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0));
                let window = Rc::new(event_loop.create_window(attrs).expect("create window"));
                self.window = Some(Rc::clone(&window));
                self.update_disp();
                let (dw, dh) = {
                    let d = self.host.disp.lock().unwrap();
                    (d.disp_w, d.disp_h)
                };
                let mut surface =
                    Surface::new(&self.context, window.clone()).expect("create surface");
                let _ = surface.resize(
                    NonZeroU32::new(dw.max(1) as u32).unwrap(),
                    NonZeroU32::new(dh.max(1) as u32).unwrap(),
                );
                self.state = AppState::Running { surface };
            }
        }

        fn window_event(
            &mut self,
            _event_loop: &ActiveEventLoop,
            _id: WindowId,
            event: WindowEvent,
        ) {
            match event {
                WindowEvent::CloseRequested => {
                    self.host.stop.store(true, Ordering::Relaxed);
                }
                WindowEvent::Resized(_physical) => {
                    self.update_disp();
                    if let AppState::Running { surface } = &mut self.state {
                        let (dw, dh) = {
                            let d = self.host.disp.lock().unwrap();
                            (d.disp_w, d.disp_h)
                        };
                        let _ = surface.resize(
                            NonZeroU32::new(dw.max(1) as u32).unwrap(),
                            NonZeroU32::new(dh.max(1) as u32).unwrap(),
                        );
                    }
                }
                WindowEvent::ScaleFactorChanged { .. } => self.update_disp(),
                WindowEvent::RedrawRequested => {
                    if let AppState::Running { surface } = &mut self.state {
                        let fb = self.host.fb_addr().unwrap_or(0) as *const u32;
                        let disp_w = self.host.disp.lock().unwrap().disp_w;
                        let disp_h = self.host.disp.lock().unwrap().disp_h;
                        blit(fb, &self.host.stop, surface, disp_w, disp_h);
                    }
                }
                // ── Mouse input → shared state. ──
                WindowEvent::CursorMoved { position, .. } => {
                    let (wl, wh) = self
                        .window
                        .as_ref()
                        .map(|w| {
                            let s = w.scale_factor();
                            let inner = w.inner_size();
                            (inner.width as f64 / s, inner.height as f64 / s)
                        })
                        .unwrap_or((FB_WIDTH as f64, FB_HEIGHT as f64));
                    let logical: winit::dpi::LogicalPosition<f64> = position.to_logical(
                        self.window
                            .as_ref()
                            .map(|w| w.scale_factor())
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
                    use winit::event::KeyEvent;
                    use winit::keyboard::PhysicalKey;
                    let KeyEvent {
                        physical_key,
                        state,
                        text,
                        ..
                    } = event;
                    let code = match physical_key {
                        PhysicalKey::Code(c) => c as u32,
                        PhysicalKey::Unidentified(_) => 0,
                    };
                    let mut kb = self.host.keyboard.lock().unwrap();
                    kb.code = code;
                    kb.down = matches!(state, ElementState::Pressed);
                    kb.text = text
                        .and_then(|t| t.chars().next())
                        .map(|c| c as u32)
                        .unwrap_or(0);
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

        fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: ()) {
            // A frame is complete (from display's CTRL write): request a redraw.
            let AppState::Running { surface } = &self.state else {
                return;
            };
            surface.window().request_redraw();
        }

        fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
            if self.host.stop.load(Ordering::Relaxed) {
                event_loop.exit();
            } else {
                event_loop.set_control_flow(ControlFlow::Wait);
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

    let event_loop = EventLoop::<()>::with_user_event()
        .with_any_thread(true) // allow creating the loop on a background thread
        .build()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    // Register the proxy before the loop runs so display's CTRL write can wake us.
    *host.proxy.lock().unwrap() = Some(event_loop.create_proxy());
    let context = Context::new(event_loop.owned_display_handle())?;
    let mut app = App {
        context,
        state: AppState::Initial,
        host: Arc::clone(host),
        window: None,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
