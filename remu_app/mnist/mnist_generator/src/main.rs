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
/// One paint pass over a cell; a second pass on the same cell adds up to white.
const INK_STEP: u8 = 85;
/// Erasing removes slightly more than one pass paints.
const ERASE_STEP: u8 = 96;

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
    }));

    {
        let app = Rc::clone(&app);
        let weak = ui.as_weak();
        ui.on_paint(move |x, y, ink| {
            let Some(ui) = weak.upgrade() else { return };
            let radius = ui.get_brush_radius().round() as i32;
            app.borrow_mut().paint(x, y, ink, radius);
        });
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
}

impl App {
    fn paint(&mut self, x: f32, y: f32, ink: bool, radius: i32) {
        let Some((gx, gy)) = cell_of(x, y, self.cell) else {
            return;
        };
        let touched = paint_stamp(&mut self.pixels, gx, gy, radius, ink);
        for i in touched {
            self.gray.set_row_data(i, self.pixels[i] as i32);
        }
    }

    fn clear(&mut self) {
        self.pixels.fill(0);
        for i in 0..CELLS {
            self.gray.set_row_data(i, 0);
        }
    }
}

/// Canvas-relative logical pixels → grid cell, or `None` outside the canvas.
fn cell_of(x: f32, y: f32, cell: f32) -> Option<(i32, i32)> {
    if !(0.0..GRID as f32 * cell).contains(&x) || !(0.0..GRID as f32 * cell).contains(&y) {
        return None;
    }
    Some(((x / cell).floor() as i32, (y / cell).floor() as i32))
}

/// Paint (or erase) a filled disc of `radius` around a cell; returns the touched
/// row-major indices so only those cells are refreshed in the view.
fn paint_stamp(pixels: &mut [u8; CELLS], cx: i32, cy: i32, radius: i32, ink: bool) -> Vec<usize> {
    let mut touched = Vec::new();
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy > radius * radius {
                continue;
            }
            let x = cx + dx;
            let y = cy + dy;
            if !(0..GRID as i32).contains(&x) || !(0..GRID as i32).contains(&y) {
                continue;
            }
            let i = y as usize * GRID + x as usize;
            let value = if ink {
                // Saturates at 255, so repeated passes reach at most white.
                pixels[i].saturating_add(INK_STEP)
            } else {
                pixels[i].saturating_sub(ERASE_STEP)
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
        "Saved: {} and {} (index={}, label={})",
        txt_path.display(),
        bin_path.display(),
        ix,
        label
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

        assert_eq!(paint_stamp(&mut pixels, 5, 5, 0, true), vec![i]);
        assert_eq!(pixels[i], INK_STEP);
        let _ = paint_stamp(&mut pixels, 5, 5, 0, true);
        let _ = paint_stamp(&mut pixels, 5, 5, 0, true);
        assert_eq!(pixels[i], 255);

        let _ = paint_stamp(&mut pixels, 5, 5, 0, false);
        assert_eq!(pixels[i], 255 - ERASE_STEP);
    }

    #[test]
    fn brush_stamps_a_clipped_disc() {
        let mut pixels = [0u8; CELLS];

        // Radius 1 at the top-left corner clips to the three in-grid cells of
        // the disc: the neighbours stay outside (dx²+dy² = 2 > 1).
        let touched = paint_stamp(&mut pixels, 0, 0, 1, true);
        assert_eq!(touched, vec![0, 1, GRID]);
    }

    #[test]
    fn pointer_maps_to_cells_and_clips_outside() {
        let cell = 18.0;
        assert_eq!(cell_of(0.0, 0.0, cell), Some((0, 0)));
        assert_eq!(cell_of(17.9, 17.9, cell), Some((0, 0)));
        assert_eq!(cell_of(18.0, 0.0, cell), Some((1, 0)));
        assert_eq!(cell_of(503.9, 503.9, cell), Some((27, 27)));
        assert_eq!(cell_of(504.0, 10.0, cell), None);
        assert_eq!(cell_of(10.0, 504.0, cell), None);
        assert_eq!(cell_of(-1.0, 0.0, cell), None);
    }
}
