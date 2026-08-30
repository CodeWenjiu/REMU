//! Display device: a framebuffer (a fixed memory region) rendered to a window
//! via `softbuffer`/`winit` on a background thread.
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
//! Pixel format is **0RGB** (a `u32` with bytes `[B,G,R,0]`), matching
//! softbuffer's buffer layout, so a frame is a plain `u32` slice copy with no
//! per-pixel conversion.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use crate::bus::memory::MemRegionSpec;
use crate::bus::{BusError, device::DeviceAccess};

/// Hard-coded framebuffer size (kept simple; resolution is fixed for now).
pub(super) const FB_WIDTH: usize = 800;
pub(super) const FB_HEIGHT: usize = 600;
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

/// User event sent from the simulator thread to the render thread whenever the
/// guest finishes drawing a frame (via the MMIO control register).
#[derive(Debug, Clone, Copy)]
pub(super) struct FrameDone;

pub(super) struct Display {
    /// Non-owning pointer into the framebuffer region of `Bus::Memory`.
    /// Stored as `usize` so it is `Send` and can cross the thread boundary.
    fb: Option<usize>,
    /// Signal for the render thread to stop (dropped when Bus is destroyed).
    stop: Option<Arc<AtomicBool>>,
    /// Proxy used to wake the render thread when a frame is complete.
    proxy: Option<winit::event_loop::EventLoopProxy<FrameDone>>,
    /// Handle to the render thread.
    render_thread: Option<thread::JoinHandle<()>>,
}

impl Display {
    pub(super) fn new() -> Self {
        Self {
            fb: None,
            stop: None,
            proxy: None,
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
        // The render thread creates the event loop and returns its proxy so the
        // simulator thread can wake it on frame-done.
        let (tx, rx) = std::sync::mpsc::channel();
        let handle = thread::spawn(move || {
            let _ = render_loop(fb_addr, stop_clone, tx);
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
/// 0RGB pixels) into the window via softbuffer.
fn render_loop(
    fb_addr: usize,
    stop: Arc<AtomicBool>,
    proxy_tx: std::sync::mpsc::Sender<winit::event_loop::EventLoopProxy<FrameDone>>,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::num::NonZeroU32;
    use std::rc::Rc;

    use softbuffer::{Context, Surface};
    use winit::application::ApplicationHandler;
    use winit::event::WindowEvent;
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
    }
    // The raw framebuffer pointer is only read while the Bus/Memory outlives it
    // (Display drops before Memory). `Send` is safe: the pointer is not
    // dereferenced after Memory is gone.
    unsafe impl Send for App {}

    impl ApplicationHandler<FrameDone> for App {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            if matches!(self.state, AppState::Initial) {
                let attrs = Window::default_attributes()
                    .with_title("remu display")
                    .with_inner_size(winit::dpi::LogicalSize::new(
                        FB_WIDTH as f64,
                        FB_HEIGHT as f64,
                    ));
                let window = Rc::new(event_loop.create_window(attrs).expect("create window"));
                let mut surface =
                    Surface::new(&self.context, window.clone()).expect("create surface");
                let _ = surface.resize(
                    NonZeroU32::new(FB_WIDTH as u32).unwrap(),
                    NonZeroU32::new(FB_HEIGHT as u32).unwrap(),
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
                WindowEvent::RedrawRequested => {
                    // Split borrows: `surface` borrows state mutably, `stop`/`fb`
                    // are independent fields read here for the blit.
                    let stop = &self.stop;
                    let fb = self.fb;
                    blit(fb, stop, surface);
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
    ) {
        // Don't touch the framebuffer once stop is set: the main thread
        // may be tearing down `Memory` (freeing the region) right after.
        if stop.load(Ordering::Relaxed) {
            return;
        }
        if let Ok(mut buffer) = surface.buffer_mut() {
            // Both the framebuffer and the softbuffer are 0RGB `u32`
            // arrays, so a plain slice copy is a zero-conversion blit.
            let src = unsafe { std::slice::from_raw_parts(fb, FB_WIDTH * FB_HEIGHT) };
            buffer.copy_from_slice(src);
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
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}

impl DeviceAccess for Display {
    fn name(&self) -> &str {
        "display"
    }

    fn size(&self) -> usize {
        8
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
        // Simple status register: offset 0 = framebuffer ready (always 1).
        Ok(match offset {
            0 => 1,
            _ => 0,
        })
    }

    fn write_8(&mut self, offset: usize, _value: u8) -> Result<(), BusError> {
        // Writing to the control register signals a completed frame: wake the
        // render thread so it blits the new frame.
        let _ = offset;
        if let Some(proxy) = &self.proxy {
            let _ = proxy.send_event(FrameDone);
        }
        Ok(())
    }

    fn read_32(&mut self, offset: usize) -> Result<u32, BusError> {
        if offset >= self.size() {
            return Err(BusError::UnsupportedAccessWidth(
                32,
                std::backtrace::Backtrace::capture(),
            ));
        }
        let b0 = self.read_8(offset)? as u32;
        let b1 = self.read_8(offset + 1)? as u32;
        let b2 = self.read_8(offset + 2)? as u32;
        let b3 = self.read_8(offset + 3)? as u32;
        Ok(b0 | (b1 << 8) | (b2 << 16) | (b3 << 24))
    }

    fn write_32(&mut self, _offset: usize, _value: u32) -> Result<(), BusError> {
        // Frame-done control: wake the render thread to blit the new frame.
        if let Some(proxy) = &self.proxy {
            let _ = proxy.send_event(FrameDone);
        }
        Ok(())
    }
}
