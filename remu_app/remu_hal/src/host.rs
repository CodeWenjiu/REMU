//! Host (x86_64) equivalents of embedded HAL items.

use core::fmt;
use std::sync::{
    Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};

/// Stdout writer, API-compatible with `Uart16550`.
pub struct Stdout;

impl Stdout {
    #[inline]
    pub const fn default_base() -> Self {
        Stdout
    }
}

impl fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        use std::io::Write;
        std::io::stdout()
            .write_all(s.as_bytes())
            .map_err(|_| fmt::Error)
    }
}

/// No-op heap init on host (global allocator already set up by std).
#[inline]
pub fn init() {}

/// MTIME tick frequency (host: 1000 ticks/sec — `read_mtime` returns ms).
pub const MTIME_TICK_HZ: u64 = 1000;

/// Read CLINT mtime (host: milliseconds since process start).
#[inline]
pub fn read_mtime() -> u64 {
    use std::sync::OnceLock;
    use std::time::Instant;
    // Monotonic ms since process start — good enough for animation timing.
    static START: OnceLock<Instant> = OnceLock::new();
    let _ = START.get_or_init(Instant::now);
    START.get().unwrap().elapsed().as_millis() as u64
}

// ── Display device (host backend: a real softbuffer/winit window) ──
//
// The embedded app talks to the display via MMIO registers and a fixed
// framebuffer region (`FB_BASE`). On host there is no MMIO, so we back those
// same calls with:
//   - a real `[u32; FB_WIDTH*FB_HEIGHT]` buffer (so `FB_BASE` is a genuine
//     writable address and existing app code works unchanged),
//   - a lazily-spawned winit event loop thread that blits the framebuffer to a
//     window and feeds mouse/window-size state back to the app.
//
// Only the functions in this section are reachable by apps; everything below
// is the render backend.

/// Framebuffer capacity (matches the display device).
pub const FB_WIDTH: usize = 2048;
pub const FB_HEIGHT: usize = 2048;

/// The host framebuffer backing store (heap-allocated, writable).
/// The app writes through a raw pointer derived from `fb_base()`.
pub(crate) static FRAMEBUFFER: OnceLock<Box<[u32]>> = OnceLock::new();

/// Address of the host framebuffer (used as `fb_base`).
#[inline]
pub(crate) fn fb_addr() -> usize {
    // Allocate on first use; never freed. The boxed slice is writable heap
    // memory, so raw-pointer writes by the app are valid.
    FRAMEBUFFER
        .get_or_init(|| vec![0u32; FB_WIDTH * FB_HEIGHT].into_boxed_slice())
        .as_ptr() as usize
}

/// Base address of the display framebuffer (canonical runtime API).
#[inline]
pub fn fb_base() -> usize {
    fb_addr()
}

/// Write a single 0RGB pixel into the framebuffer (bounds-checked to capacity).
///
/// `v` is a 0RGB u32: 0x00RRGGBB (XRGB).
#[inline]
pub fn put_pixel(fb: *mut u32, x: usize, y: usize, v: u32) {
    if x < FB_WIDTH && y < FB_HEIGHT {
        unsafe {
            *fb.add(y * FB_WIDTH + x) = v;
        }
    }
}

/// Active display resolution in framebuffer pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DisplaySize {
    /// Width in framebuffer pixels.
    pub width: usize,
    /// Height in framebuffer pixels.
    pub height: usize,
}

/// Mouse position (framebuffer pixels) and button state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MouseState {
    /// Cursor X in framebuffer pixels.
    pub x: usize,
    /// Cursor Y in framebuffer pixels.
    pub y: usize,
    /// Button bitmask (bit 0=left, 1=right, 2=middle).
    pub buttons: u32,
}

/// Key state (keycode + press/release + text char).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyState {
    /// Physical key code (winit `PhysicalKey::Code` value).
    pub code: u32,
    /// Whether the last event was a press (true) or release (false).
    pub down: bool,
    /// Last text character (if printable), else 0.
    pub text: u32,
    /// Set once a key event has occurred.
    pub valid: bool,
    /// Live NES joypad button bitmask (render thread maintains).
    pub buttons: u8,
}

/// Shared window state: resolution + mouse + keyboard. Written by the render
/// thread, read by the app via the accessors below.
struct Shared {
    disp: Mutex<DisplaySize>,
    mouse: Mutex<MouseState>,
    keyboard: Mutex<KeyState>,
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

/// Signal to the display device that the current frame is complete.
#[inline]
pub fn frame_done() {
    let b = backend();
    if b.closed.load(Ordering::Relaxed) {
        return;
    }
    // Wake the render thread so it blits the updated framebuffer. The proxy is
    // registered in `render_loop`; if it isn't ready yet the send fails and we
    // just skip this frame.
    if let Some(proxy) = PROXY.get() {
        let _ = proxy.send_event(());
    }
}

/// Whether the display window is currently alive (host: not closed).
#[inline]
pub fn display_alive() -> bool {
    !backend().closed.load(Ordering::Relaxed)
}

/// Read the current active display resolution (framebuffer pixels).
#[inline]
pub fn read_disp_size() -> DisplaySize {
    *backend().shared.disp.lock().unwrap()
}

/// Read the current active display width (framebuffer pixels).
#[inline]
pub fn read_disp_w() -> usize {
    read_disp_size().width
}

/// Read the current active display height (framebuffer pixels).
#[inline]
pub fn read_disp_h() -> usize {
    read_disp_size().height
}

/// Read the mouse position and button state (framebuffer pixels).
#[inline]
pub fn read_mouse() -> MouseState {
    *backend().shared.mouse.lock().unwrap()
}

/// Read the mouse X position (framebuffer pixels).
#[inline]
pub fn read_mouse_x() -> usize {
    read_mouse().x
}

/// Read the mouse Y position (framebuffer pixels).
#[inline]
pub fn read_mouse_y() -> usize {
    read_mouse().y
}

/// Read the mouse buttons bitmask (bit 0=left, 1=right, 2=middle).
#[inline]
pub fn read_mouse_buttons() -> u32 {
    read_mouse().buttons
}

/// Read the keyboard state (keycode + press/release + text).
#[inline]
pub fn read_key() -> KeyState {
    *backend().shared.keyboard.lock().unwrap()
}

/// Read the last key code (physical position).
#[inline]
pub fn read_key_code() -> u32 {
    read_key().code
}

/// Read whether the last key event was a press (1) or release (0).
#[inline]
pub fn read_key_down() -> u32 {
    read_key().down as u32
}

/// Read the last text character (ASCII), or 0 if non-printable.
#[inline]
pub fn read_key_text() -> u32 {
    read_key().text
}

/// Read whether any key event has occurred yet (1) or not (0).
#[inline]
pub fn read_key_valid() -> u32 {
    read_key().valid as u32
}

/// Read the live NES joypad button bitmask.
#[inline]
pub fn read_key_buttons() -> u32 {
    read_key().buttons as u32
}

// ── Render backend (softbuffer + winit event loop) ──

/// Proxy used to wake the render thread from `frame_done`.
static PROXY: OnceLock<winit::event_loop::EventLoopProxy<()>> = OnceLock::new();

fn render_loop(shared: &'static Shared) -> Result<(), Box<dyn std::error::Error>> {
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
        shared: &'static Shared,
        window: Option<Rc<Window>>,
    }

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
    /// bit, or 0. Mirrors the embedded window host mapping.
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
            let mut d = self.shared.disp.lock().unwrap();
            d.width = wl.min(FB_WIDTH).max(1);
            d.height = wh.min(FB_HEIGHT).max(1);
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
                    use winit::keyboard::{Key, NamedKey, PhysicalKey};
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
                    // `text` is None for non-printable keys (arrows, Escape,
                    // ...) and `\r` for Enter; but Slint's key bindings expect
                    // specific unicode code points (`@keys(Return)` matches
                    // '\n', arrows are in the private-use block). Map the
                    // logical key first so these are always correct, falling
                    // back to the raw text for printable keys.
                    let text_code = {
                        let mapped = if let Key::Named(named) = logical_key {
                            match named {
                                NamedKey::ArrowDown => Some(0xF701),
                                NamedKey::ArrowUp => Some(0xF700),
                                NamedKey::ArrowLeft => Some(0xF702),
                                NamedKey::ArrowRight => Some(0xF703),
                                // Slint's `Return` key maps to '\n' (0x0A).
                                NamedKey::Enter => Some('\n' as u32),
                                NamedKey::Escape => Some(0x1B),
                                _ => None,
                            }
                        } else {
                            None
                        };
                        mapped.or_else(|| text.and_then(|t| t.chars().next()).map(|c| c as u32))
                    };
                    let mut kb = self.shared.keyboard.lock().unwrap();
                    kb.code = code;
                    kb.down = matches!(state, ElementState::Pressed);
                    kb.text = text_code.unwrap_or(0);
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
