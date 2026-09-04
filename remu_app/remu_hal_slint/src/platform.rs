//! Software-renderer Slint platform backed by the remu framebuffer + input
//! devices. Shared by host and embedded (remu_hal exposes the same API).
//!
//! The render path writes Slint's per-line pixels into the remu framebuffer
//! via `remu_hal::put_pixel` and signals frame completion via `frame_done`.
//! Input is polled from `remu_hal::read_key` / `read_mouse` and translated to
//! Slint `WindowEvent`s.

use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::string::ToString;

use remu_hal::{
    MTIME_TICK_HZ, fb_base, frame_done, put_pixel, read_disp_h, read_disp_w, read_key, read_mouse,
    read_mtime,
};
use slint::platform::{
    Platform, WindowAdapter, WindowEvent,
    software_renderer::{LineBufferProvider, MinimalSoftwareWindow, Rgb565Pixel},
};

/// Size of the scratch line buffer used when rendering (one framebuffer row).
const LINE_MAX: usize = 2048;

/// A Slint application instance backed by the remu display.
///
/// Call [`SlintApp::init`] once to install the platform, then pump events and
/// redraw from your main loop with [`SlintApp::update`].
pub struct SlintApp {
    window: Rc<MinimalSoftwareWindow>,
    /// Width/height of the last known display (logical).
    width: u32,
    height: u32,
    /// Last mouse button state, to detect press/release edges.
    last_left: bool,
    /// Last key down state, to detect key press/release edges.
    last_key_down: bool,
    /// Last key text, to detect new printable keys (host snapshots miss the
    /// down edge, so we detect presses by text change).
    last_key_text: u32,
    /// When set to a text code point, any key event whose text equals this
    /// value is treated as already consumed (a stale snapshot from before the
    /// UI loop resumed) and not dispatched, until a different key is seen.
    skip_until_text: Option<u32>,
}

impl SlintApp {
    /// Install the Slint platform and return the app handle.
    pub fn init() -> Self {
        // NewBuffer forces a full redraw each frame. ReusedBuffer (the default)
        // only repaints the dirty region, which leaves the rest of the remu
        // framebuffer at its initial (black) content until an interaction
        // triggers a repaint — so the first frame would only show part of the
        // window. Full redraws are slightly slower but always correct here.
        let window = MinimalSoftwareWindow::new(
            slint::platform::software_renderer::RepaintBufferType::NewBuffer,
        );
        slint::platform::set_platform(Box::new(RemuPlatform {
            window: window.clone(),
        }))
        .expect("slint platform already set");
        // Size the window to the current display (logical pixels).
        let w = read_disp_w().max(1) as u32;
        let h = read_disp_h().max(1) as u32;
        window.set_size(slint::PhysicalSize::new(w, h));
        Self {
            window,
            width: w,
            height: h,
            last_left: false,
            last_key_down: false,
            last_key_text: 0,
            skip_until_text: None,
        }
    }

    /// Forget any in-flight key event so a stale snapshot isn't re-delivered to
    /// Slint. Call this before re-entering the UI loop after the window was
    /// driven by something else (e.g. a NES game loop reading the raw device) —
    /// otherwise a lingering key (like Escape) is dispatched to Slint as a
    /// fresh press.
    pub fn resync_key_state(&mut self) {
        let key = read_key();
        // Treat the current (possibly stale) key as consumed, and keep
        // consuming it until a different key arrives. `last_key_text` stays a
        // sentinel so the first *new* key is recognized.
        self.last_key_text = u32::MAX;
        self.last_key_down = false;
        self.last_left = false;
        self.skip_until_text = key.valid.then_some(key.text);
    }

    /// Pump keyboard/mouse input from the remu devices, run timers/animations,
    /// and redraw. Call this each frame.
    pub fn update(&mut self) {
        // Keep the Slint window sized to the current display. The display size
        // may start at the framebuffer capacity (2048×2048) before the window
        // reports its real size; re-syncing every frame and forcing a redraw
        // ensures the picture always covers the whole window.
        let w = read_disp_w().max(1) as u32;
        let h = read_disp_h().max(1) as u32;
        if w != self.width || h != self.height {
            self.width = w;
            self.height = h;
            self.window.set_size(slint::PhysicalSize::new(w, h));
        }
        // Always ask for a redraw so the first frame and any resize render the
        // full window (Slint's dirty-region tracking would otherwise leave the
        // rest of the framebuffer at its initial content).
        self.window.request_redraw();

        // ── Keyboard ──
        // The remu keyboard reports the *latest* event as a snapshot; on the
        // host the winit thread can write a press+release between our polls, so
        // `down` is almost never observed. Detect new printable keys by text
        // change: any fresh non-empty text counts as a press (matching how a
        // real key tap lands). Non-printable keys are delivered on the down
        // edge when we do catch it.
        let key = read_key();
        if let Some(skip) = self.skip_until_text {
            if key.valid && key.text == skip {
                // Still the stale snapshot (e.g. the Escape that quit a NES
                // game). Keep consuming it without dispatching; once a
                // different key arrives we resume normal delivery.
                self.last_key_down = key.valid && key.down;
                self.last_left = read_mouse().buttons & 1 != 0;
            } else {
                self.skip_until_text = None;
            }
        }
        if self.skip_until_text.is_none()
            && key.valid
            && key.text != 0
            && key.text != self.last_key_text
        {
            self.last_key_text = key.text;
            // `key.text` is a full unicode code point (Slint key codes live in
            // the private-use block, e.g. 0xF701 for the down arrow). Build the
            // string from the code point rather than truncating to u8.
            if let Some(c) = char::from_u32(key.text) {
                let text: slint::SharedString = c.to_string().into();
                let _ = self
                    .window
                    .try_dispatch_event(WindowEvent::KeyPressed { text });
            }
        } else if key.valid && key.down && !self.last_key_down {
            // Non-printable (or repeated) press: deliver a KeyPressed with empty text.
            let _ = self.window.try_dispatch_event(WindowEvent::KeyPressed {
                text: slint::SharedString::default(),
            });
        } else if key.valid && !key.down && self.last_key_down {
            let _ = self.window.try_dispatch_event(WindowEvent::KeyReleased {
                text: slint::SharedString::default(),
            });
        }
        self.last_key_down = key.valid && key.down;

        // ── Mouse ──
        let mouse = read_mouse();
        let pos = slint::LogicalPosition::new(mouse.x as f32, mouse.y as f32);
        let _ = self
            .window
            .try_dispatch_event(WindowEvent::PointerMoved { position: pos });
        let left = mouse.buttons & 1 != 0;
        if left && !self.last_left {
            let _ = self.window.try_dispatch_event(WindowEvent::PointerPressed {
                position: pos,
                button: slint::platform::PointerEventButton::Left,
            });
        } else if !left && self.last_left {
            let _ = self
                .window
                .try_dispatch_event(WindowEvent::PointerReleased {
                    position: pos,
                    button: slint::platform::PointerEventButton::Left,
                });
        }
        self.last_left = left;

        // ── Timers / animations / redraw ──
        slint::platform::update_timers_and_animations();
        self.window.draw_if_needed(|renderer| {
            renderer.render_by_line(FbLineBuffer {});
        });
        frame_done();
    }
}

/// The `Platform` implementation handed to Slint.
struct RemuPlatform {
    window: Rc<MinimalSoftwareWindow>,
}

impl Platform for RemuPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.window.clone())
    }

    fn duration_since_start(&self) -> core::time::Duration {
        // remu mtime ticks at MTIME_TICK_HZ; convert to a Duration.
        let ticks = read_mtime();
        let nanos = ticks.saturating_mul(1_000_000_000) / MTIME_TICK_HZ;
        core::time::Duration::from_nanos(nanos)
    }

    fn run_event_loop(&self) -> Result<(), slint::PlatformError> {
        // The app drives the loop via `SlintApp::update()`.
        Ok(())
    }
}

/// Renders Slint's per-line pixels into the remu framebuffer.
struct FbLineBuffer;

impl LineBufferProvider for FbLineBuffer {
    type TargetPixel = Rgb565Pixel;

    fn process_line(
        &mut self,
        line: usize,
        range: core::ops::Range<usize>,
        render_fn: impl FnOnce(&mut [Self::TargetPixel]),
    ) {
        // Render the requested range into a scratch buffer.
        let len = range.len().min(LINE_MAX);
        let mut scratch = [Rgb565Pixel(0); LINE_MAX];
        render_fn(&mut scratch[..len]);

        // Convert RGB565 -> 0RGB and write into the framebuffer.
        let fb = fb_base() as *mut u32;
        for (i, px) in scratch[..len].iter().enumerate() {
            let rgb = px.0;
            let r = ((rgb >> 11) & 0x1f) as u32;
            let g = ((rgb >> 5) & 0x3f) as u32;
            let b = (rgb & 0x1f) as u32;
            // Expand 5/6-bit channels to 8-bit.
            let r8 = (r * 255 + 15) / 31;
            let g8 = (g * 255 + 31) / 63;
            let b8 = (b * 255 + 15) / 31;
            let pixel = (r8 << 16) | (g8 << 8) | b8;
            put_pixel(fb, range.start + i, line, pixel);
        }
    }
}
