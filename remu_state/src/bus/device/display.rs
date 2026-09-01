//! Display device: a framebuffer (a fixed memory region) rendered to a window.
//!
//! The framebuffer lives in `Bus::Memory` (software writes pixels directly);
//! this device only owns a raw pointer to that region and hands it to the shared
//! window host, whose render thread blits it. Keeping the pointer (instead of a
//! copy) matches a real display controller reading a memory-mapped framebuffer.
//!
//! Rendering is **event-driven**: the guest draws a frame, then writes the MMIO
//! control register; that write wakes the render thread (via the window host's
//! `EventLoopProxy`) which blits the completed frame. No polling, so the blit
//! rate tracks the guest's frame rate exactly.
//!
//! The shared window host (in `window`) owns the winit event loop and fans out
//! mouse/keyboard events to their own devices; the display device is purely the
//! framebuffer + resolution interface.
//!
//! Pixel format is **0RGB** (a `u32` with bytes `[B,G,R,0]`), matching
//! softbuffer's buffer layout, so a frame is a plain `u32` slice copy with no
//! per-pixel conversion.

use std::sync::Arc;

use crate::bus::device::window::{DisplayState, WindowHost};
use crate::bus::memory::MemRegionSpec;
use crate::bus::{BusError, device::DeviceContext};

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
pub(super) const REG_DISP_W: usize = 20; // read: current display width (logical px)
pub(super) const REG_DISP_H: usize = 24; // read: current display height (logical px)
pub(super) const DISPLAY_REG_SIZE: usize = 28;

pub(super) struct Display {
    /// Non-owning pointer into the framebuffer region of `Bus::Memory`.
    fb: Option<usize>,
    /// The shared window host (owns the render thread + shared display state).
    window: Option<Arc<WindowHost>>,
}

impl Display {
    pub(super) fn new() -> Self {
        Self {
            fb: None,
            window: None,
        }
    }
}

impl super::DeviceAccess for Display {
    fn name(&self) -> &str {
        "display"
    }

    fn size(&self) -> usize {
        DISPLAY_REG_SIZE
    }

    fn attach_context(&mut self, ctx: &DeviceContext) {
        self.window = Some(Arc::clone(ctx.window()));
    }

    fn extra_mem_regions(&self) -> Vec<MemRegionSpec> {
        vec![MemRegionSpec {
            name: "fb".to_string(),
            region: FB_BASE..FB_BASE + FB_SIZE,
        }]
    }

    fn attach_mem_region(&mut self, _base: usize, ptr: *mut u8, _size: usize) {
        self.fb = Some(ptr as usize);
        // Give the window host the framebuffer so the render thread can blit it.
        if let Some(w) = &self.window {
            w.set_fb(ptr);
        }
    }

    fn read_8(&mut self, offset: usize) -> Result<u8, BusError> {
        // Byte lanes of the 32-bit registers (little-endian).
        Ok(match offset {
            REG_STATUS => self.read_status() as u8,
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
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }
        Ok(())
    }

    fn read_32(&mut self, offset: usize) -> Result<u32, BusError> {
        match offset {
            REG_STATUS => Ok(self.read_status()),
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
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }
        Ok(())
    }
}

impl Display {
    /// Status register: bit 0 = framebuffer ready (always 1 once attached),
    /// bit 1 = window alive.
    fn read_status(&self) -> u32 {
        let fb_ready = self.fb.is_some() as u32;
        let alive = self.window.as_ref().map(|w| w.alive() as u32).unwrap_or(0);
        fb_ready | (alive << 1)
    }

    fn read_disp_w(&self) -> u32 {
        self.window
            .as_ref()
            .map(|w| w.disp().lock().unwrap().disp_w as u32)
            .unwrap_or(DisplayState::initial().disp_w as u32)
    }

    fn read_disp_h(&self) -> u32 {
        self.window
            .as_ref()
            .map(|w| w.disp().lock().unwrap().disp_h as u32)
            .unwrap_or(DisplayState::initial().disp_h as u32)
    }
}
