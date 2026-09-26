//! Draw digits on a 28x28 canvas; save as `.txt` + `.bin` matching `remu_app/mnist/test_images` samples.
//! `.bin` layout matches `Inference::parse_image_binary`: 8 reserved bytes, byte `[8]` = label, `[9..793]` row-major pixels.
//!
//! The UI is Slint (`ui/mnist_draw.slint`); this file owns the pixels and the
//! file formats, and maps pointer positions onto grid cells.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![expect(rustdoc::missing_crate_level_docs)]

use std::cell::RefCell;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use slint::{ComponentHandle as _, Model as _, ModelRc, VecModel};

#[allow(unreachable_pub)]
mod ui {
    slint::include_modules!();
}
use ui::MnistDraw;

const GRID: usize = 28;
const CELLS: usize = GRID * GRID;
const BIN_LEN: usize = 8 + 1 + CELLS; // 793
/// Ink a fully covered cell gains per brush stamp; stamps overlap along a drag,
/// so one pass already reaches white.
const INK_STEP: f32 = 85.0;
/// Erasing removes slightly more ink than one stamp adds.
const ERASE_STEP: f32 = 96.0;
/// Brush positions along a drag are stamped at most this far apart (in cells),
/// so a fast stroke stays connected instead of leaving a trail of dots.
const STAMP_SPACING: f32 = 0.25;
/// Upper bound on the stamps between two pointer events (a drag can jump far).
const MAX_STAMPS: u32 = 256;

fn main() -> Result<(), slint::PlatformError> {
    env_logger::init();

    let ui = MnistDraw::new()?;
    // The canvas renders from this model; `App::pixels` stays the source of truth.
    let gray = Rc::new(VecModel::from(vec![0i32; CELLS]));
    ui.set_pixels(ModelRc::from(gray.clone()));

    let app = Rc::new(RefCell::new(App {
        pixels: [0u8; CELLS],
        gray,
        cell: ui.get_cell_size(),
        last: None,
    }));

    {
        let app = Rc::clone(&app);
        let weak = ui.as_weak();
        ui.on_paint(move |x, y, ink| {
            let Some(ui) = weak.upgrade() else { return };
            let radius = ui.get_brush_radius();
            app.borrow_mut().paint(x, y, ink, radius);
        });
    }
    {
        let app = Rc::clone(&app);
        ui.on_stroke_end(move || app.borrow_mut().end_stroke());
    }
    {
        let app = Rc::clone(&app);
        ui.on_clear_canvas(move || app.borrow_mut().clear());
    }
    {
        let app = Rc::clone(&app);
        let weak = ui.as_weak();
        ui.on_save_image(move || {
            let Some(ui) = weak.upgrade() else { return };
            let label = ui.get_label().clamp(0, 9) as u8;
            let status = match save_pair(&app.borrow().pixels, label, &test_images_dir()) {
                Ok(msg) => msg,
                Err(e) => format!("Save failed: {e}"),
            };
            ui.set_status(status.into());
        });
    }

    ui.run()
}

/// Canvas pixels plus the gray-level model the UI renders from.
struct App {
    pixels: [u8; CELLS],
    gray: Rc<VecModel<i32>>,
    /// Canvas cell size in logical pixels (declared by the `.slint`).
    cell: f32,
    /// Where the previous `paint` of the current stroke was (grid units), so a
    /// drag can be stamped continuously. `None` outside a stroke.
    last: Option<(f32, f32)>,
}

impl App {
    fn paint(&mut self, x: f32, y: f32, ink: bool, radius: f32) {
        let to = (x / self.cell, y / self.cell);
        let mut touched = Vec::new();
        for (px, py) in stroke_points(self.last, to) {
            touched.extend(paint_stamp(&mut self.pixels, px, py, radius, ink));
        }
        self.last = Some(to);
        for i in touched {
            self.gray.set_row_data(i, self.pixels[i] as i32);
        }
    }

    /// Pointer released: the next `paint` is the start of a new stroke.
    fn end_stroke(&mut self) {
        self.last = None;
    }

    fn clear(&mut self) {
        self.pixels.fill(0);
        for i in 0..CELLS {
            self.gray.set_row_data(i, 0);
        }
    }
}

/// Points to stamp for a drag from `from` (grid units, cell = 1) to `to`: `to`
/// alone when a stroke starts, otherwise the segment sampled every
/// [`STAMP_SPACING`] cells so fast drags stay connected.
fn stroke_points(from: Option<(f32, f32)>, to: (f32, f32)) -> Vec<(f32, f32)> {
    let Some((fx, fy)) = from else {
        return vec![to];
    };
    let (dx, dy) = (to.0 - fx, to.1 - fy);
    let steps = ((dx * dx + dy * dy).sqrt() / STAMP_SPACING).ceil() as u32;
    let steps = steps.clamp(1, MAX_STAMPS);
    (1..=steps)
        .map(|s| {
            let t = s as f32 / steps as f32;
            (fx + dx * t, fy + dy * t)
        })
        .collect()
}

/// Paint (or erase) the disc of `radius` cells centred on a grid position;
/// returns the touched row-major indices so only those cells are refreshed.
///
/// The brush is continuous: a cell centre closer than `radius` is inked fully
/// and the next cell out fades linearly, so fractional radii and sub-cell
/// positions give a soft edge instead of a staircase.
fn paint_stamp(pixels: &mut [u8; CELLS], cx: f32, cy: f32, radius: f32, ink: bool) -> Vec<usize> {
    let mut touched = Vec::new();
    let reach = radius + 1.0;
    let lo = |c: f32| ((c - reach).floor() as i32).max(0);
    let hi = |c: f32| ((c + reach).ceil() as i32).min(GRID as i32 - 1);
    for gy in lo(cy)..=hi(cy) {
        for gx in lo(cx)..=hi(cx) {
            let dx = gx as f32 + 0.5 - cx;
            let dy = gy as f32 + 0.5 - cy;
            let coverage = (reach - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
            if coverage == 0.0 {
                continue;
            }
            let step = if ink { INK_STEP } else { ERASE_STEP } * coverage;
            let i = gy as usize * GRID + gx as usize;
            let value = if ink {
                // Saturates at 255, so repeated passes reach at most white.
                pixels[i].saturating_add(step.round() as u8)
            } else {
                pixels[i].saturating_sub(step.round() as u8)
            };
            if value != pixels[i] {
                pixels[i] = value;
                touched.push(i);
            }
        }
    }
    touched
}

fn test_images_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_images")
}

/// `saved_image_00000.txt` / `.bin` → index 0
fn parse_saved_index(file_name: &str) -> Option<u32> {
    let rest = file_name.strip_prefix("saved_image_")?;
    let num = rest
        .strip_suffix(".txt")
        .or_else(|| rest.strip_suffix(".bin"))?;
    num.parse().ok()
}

fn next_save_index(dir: &Path) -> u32 {
    let mut max_ix: Option<u32> = None;
    if let Ok(entries) = fs::read_dir(dir) {
        for ent in entries.flatten() {
            if let Some(name) = ent.file_name().to_str()
                && let Some(n) = parse_saved_index(name)
            {
                max_ix = Some(max_ix.map_or(n, |m| m.max(n)));
            }
        }
    }
    max_ix.map_or(0, |m| m.saturating_add(1))
}

/// The `.txt` sample format: header lines plus 28 rows of `{:>3}` values.
fn encode_txt(image_index: u32, label: u8, pixels: &[u8; CELLS]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Image Index: {}", image_index);
    let _ = writeln!(out, "True Label: {}", label);
    let _ = writeln!(out, "Image Data (28x28):");
    for row in 0..GRID {
        for col in 0..GRID {
            let _ = write!(out, "{:>3}", pixels[row * GRID + col]);
            if col + 1 < GRID {
                out.push(' ');
            }
        }
        out.push('\n');
    }
    out
}

/// The `.bin` sample format: 8 reserved bytes, label, then row-major pixels.
fn encode_bin(label: u8, pixels: &[u8; CELLS]) -> [u8; BIN_LEN] {
    let mut buf = [0u8; BIN_LEN];
    buf[8] = label;
    buf[9..].copy_from_slice(pixels);
    buf
}

fn save_pair(pixels: &[u8; CELLS], label: u8, dir: &Path) -> Result<String, String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;

    let ix = next_save_index(dir);
    let base = format!("saved_image_{:05}", ix);
    let txt_path = dir.join(format!("{base}.txt"));
    let bin_path = dir.join(format!("{base}.bin"));

    fs::write(&txt_path, encode_txt(ix, label, pixels)).map_err(|e| e.to_string())?;
    fs::write(&bin_path, encode_bin(label, pixels)).map_err(|e| e.to_string())?;

    Ok(format!(
        "Saved saved_image_{ix:05}.txt/.bin in remu_app/mnist/test_images/ (label={label})"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bin_matches_the_inference_layout() {
        let mut pixels = [0u8; CELLS];
        pixels[0] = 200;
        pixels[CELLS - 1] = 7;

        let bin = encode_bin(9, &pixels);

        assert_eq!(bin.len(), BIN_LEN);
        assert_eq!(&bin[..8], &[0u8; 8]);
        assert_eq!(bin[8], 9);
        assert_eq!(&bin[9..], &pixels[..]);
    }

    #[test]
    fn txt_has_three_header_lines_and_28_rows() {
        let mut pixels = [0u8; CELLS];
        pixels[0] = 255;

        let txt = encode_txt(3, 5, &pixels);

        assert!(txt.starts_with("Image Index: 3\nTrue Label: 5\nImage Data (28x28):\n"));
        assert_eq!(txt.lines().count(), 3 + GRID);
        let mut first_row = String::from("255");
        for _ in 1..GRID {
            first_row.push_str("   0");
        }
        assert_eq!(txt.lines().nth(3), Some(first_row.as_str()));
    }

    #[test]
    fn paint_accumulates_and_erase_removes_more_than_one_pass() {
        let mut pixels = [0u8; CELLS];
        let i = 5 * GRID + 5;
        // The cell centre of (5, 5) in grid units, where the brush is centred.
        let (cx, cy) = (5.5, 5.5);

        assert_eq!(paint_stamp(&mut pixels, cx, cy, 0.0, true), vec![i]);
        assert_eq!(pixels[i], INK_STEP as u8);
        let _ = paint_stamp(&mut pixels, cx, cy, 0.0, true);
        let _ = paint_stamp(&mut pixels, cx, cy, 0.0, true);
        assert_eq!(pixels[i], 255);

        let _ = paint_stamp(&mut pixels, cx, cy, 0.0, false);
        assert_eq!(pixels[i], 255 - ERASE_STEP as u8);
    }

    #[test]
    fn brush_stamps_a_round_footprint() {
        let mut pixels = [0u8; CELLS];

        // Radius 1 at the top-left cell centre: the solid part of the disc
        // covers the corner block, its diagonal only partially (0.59 of a cell).
        let touched = paint_stamp(&mut pixels, 0.5, 0.5, 1.0, true);
        assert_eq!(touched, vec![0, 1, GRID, GRID + 1]);
        assert_eq!(pixels[0], INK_STEP as u8);
        assert_eq!(pixels[1], INK_STEP as u8);
        assert_eq!(pixels[GRID + 1], 50);
        // Two cells out is already out of reach.
        assert_eq!(pixels[2], 0);
        assert_eq!(pixels[2 * GRID], 0);
    }

    #[test]
    fn larger_radii_fade_out_over_one_cell() {
        let mut pixels = [0u8; CELLS];

        // Radius 2.5: solid up to 2.5 cells, half ink at 3, nothing at 4.
        let _ = paint_stamp(&mut pixels, 10.5, 10.5, 2.5, true);
        let row = |dx: usize| 10 * GRID + 10 + dx;
        assert_eq!(pixels[row(2)], INK_STEP as u8);
        assert_eq!(pixels[row(3)], 43);
        assert_eq!(pixels[row(4)], 0);
    }

    #[test]
    fn stroke_points_connect_fast_drags() {
        // A new stroke stamps only where it starts.
        assert_eq!(stroke_points(None, (3.0, 4.0)), vec![(3.0, 4.0)]);

        // A jump is sampled so consecutive stamps overlap.
        let pts = stroke_points(Some((0.5, 0.5)), (5.5, 0.5));
        assert_eq!(pts.first(), Some(&(0.5 + STAMP_SPACING, 0.5)));
        assert_eq!(pts.last(), Some(&(5.5, 0.5)));
        for pair in pts.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            assert!((b.0 - a.0).hypot(b.1 - a.1) <= STAMP_SPACING);
        }

        // Standing still still paints once.
        assert_eq!(
            stroke_points(Some((1.0, 1.0)), (1.0, 1.0)),
            vec![(1.0, 1.0)]
        );
    }

    #[test]
    fn stamping_outside_the_canvas_clips_to_the_grid() {
        let mut pixels = [0u8; CELLS];

        // Centred just off the corner: only the corner cell is within reach.
        assert_eq!(paint_stamp(&mut pixels, -0.5, -0.5, 1.0, true), vec![0]);
        // Entirely outside: nothing is touched.
        assert!(paint_stamp(&mut pixels, -5.0, 14.0, 1.0, true).is_empty());
        assert!(paint_stamp(&mut pixels, 40.0, 12.0, 1.0, true).is_empty());
    }
}
