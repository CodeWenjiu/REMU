//! Display device: a framebuffer (a fixed memory region) rendered to a window
//! via `softbuffer`/`winit` on a background thread, plus a **mouse input**
//! channel: pointer position and buttons are captured from the window and
//! exposed to the guest as MMIO read registers.
//!
//! The framebuffer lives in `Bus::Memory` (software writes pixels directly);
//! this device only owns a raw pointer to that region and a render thread that
//! blits it to the window. Keeping the pointer (instead of a copy) matches a
//! real display controller reading a memory-mapped framebuffer.
//!
//! Rendering is **event-driven**: the guest draws a frame, then writes the MMIO
//! control register; that write wakes the render thread (via `EventLoopProxy`)
//! which blits the completed frame at a frame boundary. No polling, so the blit
//! rate tracks the guest's frame rate exactly (no dropped frames / flicker).
//!
//! Mouse input flows the other way: winit events update a shared `MouseState`;
//! the guest reads it via MMIO registers (mouse_x, mouse_y, mouse_buttons).
//!
//! Pixel format is **0RGB** (a `u32` with bytes `[B,G,R,0]`), matching
//! softbuffer's buffer layout, so a frame is a plain `u32` slice copy with no
//! per-pixel conversion.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use crate::bus::memory::MemRegionSpec;
use crate::bus::{BusError, device::DeviceAccess};

/// Framebuffer capacity (fixed, generous). The window may be any size up to
/// this; only the active display region (see `disp_w`/`disp_h`) is blitted.
/// 2048×2048 ≈ 16 MiB of 0RGB pixels.
pub(super) const FB_WIDTH: usize = 2048;
pub(super) const FB_HEIGHT: usize = 2048;
/// Bytes per pixel: 4 (0RGB as a `u32`).
pub(super) const FB_BYTES_PER_PIXEL: usize = 4;
/// Raw pixel data size; rounded up to a page multiple below.
pub(super) const FB_RAW_SIZE: usize = FB_WIDTH * FB_HEIGHT * FB_BYTES_PER_PIXEL;
/// Page size used by `MemoryEntry`.
pub(super) const FB_PAGE: usize = 4096;
/// Allocated region size: raw size rounded up to a whole page.
pub(super) const FB_SIZE: usize = (FB_RAW_SIZE + FB_PAGE - 1) / FB_PAGE * FB_PAGE;

/// Base address of the framebuffer memory region (declared via `extra_mem_regions`).
pub(super) const FB_BASE: usize = 0x8900_0000;

/// MMIO register offsets for the display device.
pub(super) const REG_STATUS: usize = 0; // read: 1 = framebuffer ready
pub(super) const REG_CTRL: usize = 4; // write: frame-done
pub(super) const REG_MOUSE_X: usize = 8; // read: mouse x (framebuffer px)
pub(super) const REG_MOUSE_Y: usize = 12; // read: mouse y
pub(super) const REG_MOUSE_BUTTONS: usize = 16; // read: bitmask (1=left,2=right,4=mid)
pub(super) const REG_DISP_W: usize = 20; // read: current display width (logical px)
pub(super) const REG_DISP_H: usize = 24; // read: current display height (logical px)
pub(super) const DISPLAY_REG_SIZE: usize = 28;

/// User event sent from the simulator thread to the render thread whenever the
/// guest finishes drawing a frame (via the MMIO control register).
#[derive(Debug, Clone, Copy)]
pub(super) struct FrameDone;

/// Shared mouse state, written by the render thread on winit events and read
/// by the simulator thread when the guest polls the MMIO registers. Coordinates
/// are framebuffer pixels; buttons is a bitmask (1=left, 2=right, 4=middle).
#[derive(Default)]
pub(super) struct MouseState {
    pub x: i32,
    pub y: i32,
    pub buttons: u32,
    /// Set when the mouse has moved at least once (so the guest can distinguish
    /// "no mouse yet" from "mouse at (0,0)").
    pub valid: bool,
}

/// Shared display state: the currently active display resolution (logical
/// pixels) of the window. Written by the render thread on resize/scale changes,
/// read by the simulator thread when the guest queries `REG_DISP_W/H`.
/// Coordinates are relative to the framebuffer origin (top-left).
#[derive(Clone, Copy, Default)]
pub(super) struct DisplayState {
    pub disp_w: usize,
    pub disp_h: usize,
}

impl DisplayState {
    /// Initial value until the window reports its real size.
    fn initial() -> Self {
        Self {
            disp_w: FB_WIDTH,
            disp_h: FB_HEIGHT,
        }
    }
}

pub(super) struct Display {
    /// Non-owning pointer into the framebuffer region of `Bus::Memory`.
    /// Stored as `usize` so it is `Send` and can cross the thread boundary.
    fb: Option<usize>,
    /// Signal for the render thread to stop (dropped when Bus is destroyed).
    stop: Option<Arc<AtomicBool>>,
    /// Proxy used to wake the render thread when a frame is complete.
    proxy: Option<winit::event_loop::EventLoopProxy<FrameDone>>,
    /// Shared mouse input state (render thread writes, simulator reads).
    mouse: Arc<std::sync::Mutex<MouseState>>,
    /// Shared active display resolution (render thread writes, simulator reads).
    disp: Arc<std::sync::Mutex<DisplayState>>,
    /// Handle to the render thread.
    render_thread: Option<thread::JoinHandle<()>>,
}

impl Display {
    pub(super) fn new() -> Self {
        Self {
            fb: None,
            stop: None,
            proxy: None,
            mouse: Arc::new(std::sync::Mutex::new(MouseState::default())),
            disp: Arc::new(std::sync::Mutex::new(DisplayState::initial())),
            render_thread: None,
        }
    }

    fn spawn_render(&mut self) {
        if self.render_thread.is_some() || self.fb.is_none() {
            return;
        }
        let fb_addr = self.fb.unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop);
        let mouse = Arc::clone(&self.mouse);
        let disp = Arc::clone(&self.disp);
        // The render thread creates the event loop and returns its proxy so the
        // simulator thread can wake it on frame-done.
        let (tx, rx) = std::sync::mpsc::channel();
        let handle = thread::spawn(move || {
            let _ = render_loop(fb_addr, stop_clone, tx, mouse, disp);
        });
        // Block until the render thread hands back its proxy (it is created
        // inside `render_loop` before the event loop starts). If the thread
        // failed to build the loop, `rx.recv()` errors and we keep proxy None.
        if let Ok(proxy) = rx.recv() {
            self.proxy = Some(proxy);
        }
        self.stop = Some(stop);
        self.render_thread = Some(handle);
    }
}

impl Drop for Display {
    fn drop(&mut self) {
        if let Some(stop) = &self.stop {
            stop.store(true, Ordering::Relaxed);
        }
        if let Some(h) = self.render_thread.take() {
            let _ = h.join();
        }
    }
}

/// Runs the winit event loop, reading the framebuffer (`fb_addr` as `*const u32`,
/// 0RGB pixels) into the window via softbuffer, and forwarding mouse events to
/// the shared `MouseState`.
fn render_loop(
    fb_addr: usize,
    stop: Arc<AtomicBool>,
    proxy_tx: std::sync::mpsc::Sender<winit::event_loop::EventLoopProxy<FrameDone>>,
    mouse: Arc<std::sync::Mutex<MouseState>>,
    disp: Arc<std::sync::Mutex<DisplayState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::num::NonZeroU32;
    use std::rc::Rc;

    use softbuffer::{Context, Surface};
    use winit::application::ApplicationHandler;
    use winit::event::{ElementState, MouseButton, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle};
    use winit::platform::wayland::EventLoopBuilderExtWayland as _;
    use winit::window::{Window, WindowId};

    let fb_ptr = fb_addr as *const u32;

    enum AppState {
        Initial,
        Running {
            surface: Surface<OwnedDisplayHandle, Rc<Window>>,
        },
    }

    struct App {
        context: Context<OwnedDisplayHandle>,
        state: AppState,
        fb: *const u32,
        stop: Arc<AtomicBool>,
        mouse: Arc<std::sync::Mutex<MouseState>>,
        disp: Arc<std::sync::Mutex<DisplayState>>,
        window: Option<Rc<Window>>,
    }
    // The raw framebuffer pointer is only read while the Bus/Memory outlives it
    // (Display drops before Memory). `Send` is safe: the pointer is not
    // dereferenced after Memory is gone.
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
            let mut d = self.disp.lock().unwrap();
            d.disp_w = wl.min(FB_WIDTH).max(1);
            d.disp_h = wh.min(FB_HEIGHT).max(1);
        }
    }

    impl ApplicationHandler<FrameDone> for App {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            if matches!(self.state, AppState::Initial) {
                let attrs = Window::default_attributes()
                    .with_title("remu display")
                    // Start with a modest window; the framebuffer is much larger.
                    .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0));
                let window = Rc::new(event_loop.create_window(attrs).expect("create window"));
                self.window = Some(Rc::clone(&window));
                self.update_disp();
                let (dw, dh) = {
                    let d = self.disp.lock().unwrap();
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
            let AppState::Running { surface } = &mut self.state else {
                return;
            };
            match event {
                WindowEvent::CloseRequested => {
                    self.stop.store(true, Ordering::Relaxed);
                }
                // The window changed size (or its scale factor changed): update
                // the active display resolution and resize the softbuffer surface
                // to the LOGICAL size. The framebuffer region is logical pixels,
                // so a 1:1 blit then stays proportional; the compositor maps
                // logical→physical (scale factor), avoiding distortion.
                WindowEvent::Resized(_physical) => {
                    self.update_disp();
                    let (dw, dh) = {
                        let d = self.disp.lock().unwrap();
                        (d.disp_w, d.disp_h)
                    };
                    if let AppState::Running { surface } = &mut self.state {
                        let _ = surface.resize(
                            NonZeroU32::new(dw.max(1) as u32).unwrap(),
                            NonZeroU32::new(dh.max(1) as u32).unwrap(),
                        );
                    }
                }
                WindowEvent::ScaleFactorChanged { .. } => self.update_disp(),
                WindowEvent::RedrawRequested => {
                    // Split borrows: `surface` borrows state mutably, `stop`/`fb`
                    // are independent fields read here for the blit.
                    let stop = &self.stop;
                    let fb = self.fb;
                    let disp_w = self.disp.lock().unwrap().disp_w;
                    let disp_h = self.disp.lock().unwrap().disp_h;
                    blit(fb, stop, surface, disp_w, disp_h);
                }
                // ── Mouse input: write into the shared state for the guest. ──
                WindowEvent::CursorMoved { position, .. } => {
                    // Normalize the cursor to the active display region. The
                    // window may not be exactly FB_WIDTH×FB_HEIGHT (Wayland can
                    // resize / scale it), so map by the window's CURRENT logical
                    // size onto the current display region. This keeps the cursor
                    // aligned with the framebuffer content at any window size.
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
                        let d = self.disp.lock().unwrap();
                        (d.disp_w, d.disp_h)
                    };
                    let mut m = self.mouse.lock().unwrap();
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
                    let mut m = self.mouse.lock().unwrap();
                    match state {
                        ElementState::Pressed => m.buttons |= bit,
                        ElementState::Released => m.buttons &= !bit,
                    }
                }
                _ => {}
            }
        }

        fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: FrameDone) {
            // A frame is complete: request a redraw so the updated framebuffer
            // is blitted. (The actual blit happens in RedrawRequested.)
            let AppState::Running { surface } = &self.state else {
                return;
            };
            surface.window().request_redraw();
        }

        fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
            if self.stop.load(Ordering::Relaxed) {
                event_loop.exit();
            } else {
                // Fully sleep; the simulator thread wakes us with a FrameDone
                // event when the guest finishes a frame. No polling.
                event_loop.set_control_flow(ControlFlow::Wait);
            }
        }
    }

    /// Blit the framebuffer to the window. `fb` is the raw framebuffer pointer,
    /// `stop` the teardown flag. Takes refs/values so it doesn't need `&mut self`.
    fn blit(
        fb: *const u32,
        stop: &Arc<AtomicBool>,
        surface: &mut Surface<OwnedDisplayHandle, Rc<Window>>,
        disp_w: usize,
        disp_h: usize,
    ) {
        // Don't touch the framebuffer once stop is set: the main thread
        // may be tearing down `Memory` (freeing the region) right after.
        if stop.load(Ordering::Relaxed) {
            return;
        }
        if let Ok(mut buffer) = surface.buffer_mut() {
            // Copy only the active display region (disp_w×disp_h) from the top
            // left of the framebuffer. The framebuffer is larger; the guest is
            // expected to render only into this region.
            let w = disp_w.min(FB_WIDTH);
            let h = disp_h.min(FB_HEIGHT);
            let buf_w = buffer.width().get() as usize;
            let buf_h = buffer.height().get() as usize;
            let cw = w.min(buf_w);
            let ch = h.min(buf_h);
            // The framebuffer rows are FB_WIDTH apart; the softbuffer rows are
            // buf_w apart. Row-by-row copy with raw pointers avoids slice
            // bounds checks on the hot path.
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

    let event_loop = EventLoop::<FrameDone>::with_user_event()
        .with_any_thread(true) // allow creating the loop on a background thread
        .build()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    // Hand the proxy back to the Display struct so the simulator thread can
    // wake us. This must happen before the loop runs.
    let _ = proxy_tx.send(event_loop.create_proxy());
    let context = Context::new(event_loop.owned_display_handle())?;
    let mut app = App {
        context,
        state: AppState::Initial,
        fb: fb_ptr,
        stop,
        mouse,
        disp,
        window: None,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}

impl DeviceAccess for Display {
    fn name(&self) -> &str {
        "display"
    }

    fn size(&self) -> usize {
        DISPLAY_REG_SIZE
    }

    fn extra_mem_regions(&self) -> Vec<MemRegionSpec> {
        vec![MemRegionSpec {
            name: "fb".to_string(),
            region: FB_BASE..FB_BASE + FB_SIZE,
        }]
    }

    fn attach_mem_region(&mut self, _base: usize, ptr: *mut u8, _size: usize) {
        self.fb = Some(ptr as usize);
        self.spawn_render();
    }

    fn read_8(&mut self, offset: usize) -> Result<u8, BusError> {
        // Byte lanes of the 32-bit registers (little-endian).
        Ok(match offset {
            REG_STATUS => 1,
            _ if (REG_MOUSE_X..REG_MOUSE_X + 4).contains(&offset) => {
                (self.read_mouse_x() >> (8 * (offset - REG_MOUSE_X))) as u8
            }
            _ if (REG_MOUSE_Y..REG_MOUSE_Y + 4).contains(&offset) => {
                (self.read_mouse_y() >> (8 * (offset - REG_MOUSE_Y))) as u8
            }
            _ if (REG_MOUSE_BUTTONS..REG_MOUSE_BUTTONS + 4).contains(&offset) => {
                (self.read_mouse_buttons() >> (8 * (offset - REG_MOUSE_BUTTONS))) as u8
            }
            _ if (REG_DISP_W..REG_DISP_W + 4).contains(&offset) => {
                (self.read_disp_w() >> (8 * (offset - REG_DISP_W))) as u8
            }
            _ if (REG_DISP_H..REG_DISP_H + 4).contains(&offset) => {
                (self.read_disp_h() >> (8 * (offset - REG_DISP_H))) as u8
            }
            _ => 0,
        })
    }

    fn write_8(&mut self, offset: usize, _value: u8) -> Result<(), BusError> {
        // Writing to the control register signals a completed frame: wake the
        // render thread so it blits the new frame.
        if offset == REG_CTRL {
            if let Some(proxy) = &self.proxy {
                let _ = proxy.send_event(FrameDone);
            }
        }
        Ok(())
    }

    fn read_32(&mut self, offset: usize) -> Result<u32, BusError> {
        match offset {
            REG_STATUS => Ok(1),
            REG_MOUSE_X => Ok(self.read_mouse_x()),
            REG_MOUSE_Y => Ok(self.read_mouse_y()),
            REG_MOUSE_BUTTONS => Ok(self.read_mouse_buttons()),
            REG_DISP_W => Ok(self.read_disp_w()),
            REG_DISP_H => Ok(self.read_disp_h()),
            _ => Err(BusError::UnsupportedAccessWidth(
                32,
                std::backtrace::Backtrace::capture(),
            )),
        }
    }

    fn write_32(&mut self, offset: usize, _value: u32) -> Result<(), BusError> {
        // Frame-done control: wake the render thread to blit the new frame.
        if offset == REG_CTRL {
            if let Some(proxy) = &self.proxy {
                let _ = proxy.send_event(FrameDone);
            }
        }
        Ok(())
    }
}

impl Display {
    fn read_mouse_x(&self) -> u32 {
        self.mouse.lock().unwrap().x as u32
    }

    fn read_mouse_y(&self) -> u32 {
        self.mouse.lock().unwrap().y as u32
    }

    fn read_mouse_buttons(&self) -> u32 {
        self.mouse.lock().unwrap().buttons
    }

    fn read_disp_w(&self) -> u32 {
        self.disp.lock().unwrap().disp_w as u32
    }

    fn read_disp_h(&self) -> u32 {
        self.disp.lock().unwrap().disp_h as u32
    }
}
