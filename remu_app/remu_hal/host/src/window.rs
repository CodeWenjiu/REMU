//! Lazily-started winit/softbuffer event-loop thread (the render backend).
//!
//! The thread blits the framebuffer to a window and feeds window-size, mouse
//! and keyboard state back into [`Shared`]. It is started on first use
//! (`shared()`), and `wake()` (called by `frame_done`) requests a redraw.

use std::sync::{
    Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};

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

/// Lazily-created render backend (window thread + shared state).
struct Backend {
    shared: &'static Shared,
    /// Set when the window is closed, so a subsequent `frame_done` no-ops.
    closed: AtomicBool,
}

static BACKEND: OnceLock<Backend> = OnceLock::new();

/// Proxy used to wake the render thread from `frame_done`.
static PROXY: OnceLock<winit::event_loop::EventLoopProxy<()>> = OnceLock::new();

/// Get the render backend, starting the window thread on first use.
fn backend() -> &'static Backend {
    BACKEND.get_or_init(|| {
        let shared: &'static Shared = Box::leak(Box::new(Shared::new()));
        // Spawn the event loop on a background thread (Linux: any thread ok).
        std::thread::spawn(move || {
            if let Err(e) = render_loop(shared) {
                eprintln!("display backend error: {e}");
            }
            // Fallback: once the render thread exits, the window is gone — mark
            // it closed so `display_alive` reflects it (aligns with remu_state's
            // `WindowHost::alive`, which is also set false on thread exit). This
            // covers exits that aren't a user `CloseRequested` (e.g. compositor
            // death, loop error).
            if let Some(b) = BACKEND.get() {
                b.closed.store(true, Ordering::Relaxed);
            }
        });
        Backend {
            shared,
            closed: AtomicBool::new(false),
        }
    })
}

/// The shared window state, starting the backend on first use.
#[inline]
pub(crate) fn shared() -> &'static Shared {
    backend().shared
}

/// Wake the render thread so it blits the updated framebuffer. The proxy is
/// registered in `render_loop`; if it isn't ready yet the send fails and we
/// just skip this frame.
#[inline]
pub(crate) fn wake() {
    if backend().closed.load(Ordering::Relaxed) {
        return;
    }
    if let Some(proxy) = PROXY.get() {
        let _ = proxy.send_event(());
    }
}

/// Whether the window is currently alive (not closed yet).
#[inline]
pub(crate) fn alive() -> bool {
    !backend().closed.load(Ordering::Relaxed)
}

fn render_loop(shared: &'static Shared) -> Result<(), Box<dyn std::error::Error>> {
    use std::num::NonZeroU32;
    use std::rc::Rc;

    use softbuffer::{Context, Surface};
    use winit::application::ApplicationHandler;
    use winit::event::{ElementState, WindowEvent};
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
        shared: &'static Shared,
        window: Option<Rc<Window>>,
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
            let mut d = self.shared.disp.lock().unwrap();
            d.width = wl.clamp(1, FB_WIDTH);
            d.height = wh.clamp(1, FB_HEIGHT);
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
                    let d = self.shared.disp.lock().unwrap();
                    (d.width, d.height)
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
                    // Mark closed so frame_done no-ops; the event loop exits next
                    // about_to_wait via the dropped window / no more events.
                    if let Some(b) = BACKEND.get() {
                        b.closed.store(true, Ordering::Relaxed);
                    }
                }
                WindowEvent::Resized(_physical) => {
                    self.update_disp();
                    if let AppState::Running { surface } = &mut self.state {
                        let (dw, dh) = {
                            let d = self.shared.disp.lock().unwrap();
                            (d.width, d.height)
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
                        let (dw, dh) = {
                            let d = self.shared.disp.lock().unwrap();
                            (d.width, d.height)
                        };
                        blit(surface, dw, dh);
                    }
                }
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
                        let d = self.shared.disp.lock().unwrap();
                        (d.width, d.height)
                    };
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

        fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: ()) {
            // A frame is complete (from `frame_done`): request a redraw so the
            // updated framebuffer is blitted. The blit happens in
            // RedrawRequested.
            let AppState::Running { surface } = &self.state else {
                return;
            };
            surface.window().request_redraw();
        }

        fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
            // If the window closed, exit the loop (backend() will recreate on
            // next use). Otherwise wait for frame_done to wake us.
            if BACKEND
                .get()
                .map(|b| b.closed.load(Ordering::Relaxed))
                .unwrap_or(false)
            {
                event_loop.exit();
            } else {
                event_loop.set_control_flow(ControlFlow::Wait);
            }
        }
    }

    /// Blit the host framebuffer to the window.
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

    let event_loop = EventLoop::<()>::with_user_event()
        .with_any_thread(true) // background thread is fine on Linux
        .build()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    // Register the proxy before the loop runs so frame_done can wake us.
    let _ = PROXY.set(event_loop.create_proxy());
    let context = Context::new(event_loop.owned_display_handle())?;
    let mut app = App {
        context,
        state: AppState::Initial,
        shared,
        window: None,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
