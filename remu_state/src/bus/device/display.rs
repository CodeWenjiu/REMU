//! Display device: a framebuffer (a fixed memory region) rendered to a window
//! via `pixels`/`winit` on a background thread.
//!
//! The framebuffer lives in `Bus::Memory` (software writes pixels directly);
//! this device only owns a raw pointer to that region and a render thread that
//! periodically blits it to the window. Keeping the pointer (instead of a copy)
//! matches a real display controller reading a memory-mapped framebuffer.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use crate::bus::memory::MemRegionSpec;
use crate::bus::{BusError, device::DeviceAccess};

/// Hard-coded framebuffer size (kept simple; resolution is fixed for now).
pub(super) const FB_WIDTH: usize = 800;
pub(super) const FB_HEIGHT: usize = 600;
/// 4 bytes per pixel (RGBA).
pub(super) const FB_BYTES_PER_PIXEL: usize = 4;
/// Raw pixel data size; rounded up to a page multiple below.
pub(super) const FB_RAW_SIZE: usize = FB_WIDTH * FB_HEIGHT * FB_BYTES_PER_PIXEL;
/// Page size used by `MemoryEntry`.
pub(super) const FB_PAGE: usize = 4096;
/// Allocated region size: raw size rounded up to a whole page.
pub(super) const FB_SIZE: usize = (FB_RAW_SIZE + FB_PAGE - 1) / FB_PAGE * FB_PAGE;

/// Base address of the framebuffer memory region (declared via `extra_mem_regions`).
pub(super) const FB_BASE: usize = 0x8900_0000;

pub(super) struct Display {
    /// Non-owning pointer into the framebuffer region of `Bus::Memory`.
    /// Stored as `usize` so it is `Send` and can cross the thread boundary.
    fb: Option<usize>,
    /// Signal for the render thread to stop (dropped when Bus is destroyed).
    stop: Option<Arc<AtomicBool>>,
    /// Handle to the render thread.
    render_thread: Option<thread::JoinHandle<()>>,
}

impl Display {
    pub(super) fn new() -> Self {
        Self {
            fb: None,
            stop: None,
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
        let handle = thread::spawn(move || {
            let _ = render_loop(fb_addr, stop_clone);
        });
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

/// Runs the winit event loop, reading the framebuffer (`fb_addr` as `*const u8`)
/// into the window. Uses `Arc<Window>` so `Pixels<'static>` is possible, which
/// satisfies winit's `ApplicationHandler: 'static` requirement.
fn render_loop(fb_addr: usize, stop: Arc<AtomicBool>) -> Result<(), Box<dyn std::error::Error>> {
    use pixels::{Pixels, SurfaceTexture};
    use winit::application::ApplicationHandler;
    use winit::event::WindowEvent;
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
    use winit::platform::wayland::EventLoopBuilderExtWayland as _;
    use winit::window::{Window, WindowId};

    let fb_ptr = fb_addr as *const u8;

    struct App {
        window: Option<Arc<Window>>,
        pixels: Option<Pixels<'static>>,
        fb: *const u8,
        stop: Arc<AtomicBool>,
    }
    // The raw framebuffer pointer is only read while the Bus/Memory outlives it
    // (Display drops before Memory). `Send` is safe: the pointer is not
    // dereferenced after Memory is gone.
    unsafe impl Send for App {}

    impl ApplicationHandler for App {
        fn resumed(&mut self, el: &ActiveEventLoop) {
            if self.window.is_none() {
                let attrs = Window::default_attributes()
                    .with_title("remu display")
                    .with_inner_size(winit::dpi::LogicalSize::new(
                        FB_WIDTH as f64,
                        FB_HEIGHT as f64,
                    ));
                let window = Arc::new(el.create_window(attrs).expect("create window"));
                let texture =
                    SurfaceTexture::new(FB_WIDTH as u32, FB_HEIGHT as u32, window.clone());
                let pixels =
                    Pixels::new(FB_WIDTH as u32, FB_HEIGHT as u32, texture).expect("create pixels");
                self.window = Some(window);
                self.pixels = Some(pixels);
            }
        }

        fn window_event(&mut self, _el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
            match event {
                WindowEvent::CloseRequested => {
                    self.stop.store(true, Ordering::Relaxed);
                }
                WindowEvent::RedrawRequested => {
                    // Don't touch the framebuffer once stop is set: the main thread
                    // may be tearing down `Memory` (freeing the region) right after.
                    if !self.stop.load(Ordering::Relaxed) {
                        if let Some(pixels) = &mut self.pixels {
                            // The framebuffer is RGBA; pixels.frame_mut() is also RGBA,
                            // so we can copy it verbatim. Copy only the raw pixel data
                            // (FB_RAW_SIZE), NOT the page-aligned region size (FB_SIZE).
                            let frame = pixels.frame_mut();
                            unsafe {
                                std::ptr::copy_nonoverlapping(
                                    self.fb,
                                    frame.as_mut_ptr(),
                                    FB_RAW_SIZE,
                                );
                            }
                            let _ = pixels.render();
                        }
                    }
                    if let Some(w) = &self.window {
                        w.request_redraw();
                    }
                }
                _ => {}
            }
        }

        fn about_to_wait(&mut self, el: &ActiveEventLoop) {
            if self.stop.load(Ordering::Relaxed) {
                el.exit();
            }
        }
    }

    let event_loop = EventLoop::builder()
        .with_any_thread(true) // allow creating the loop on a background thread
        .build()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App {
        window: None,
        pixels: None,
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

    fn write_8(&mut self, _offset: usize, _value: u8) -> Result<(), BusError> {
        // No writable control registers yet.
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
        Ok(())
    }
}
