//! Draw digits on a 28x28 canvas; save as `.txt` + `.bin` matching `remu_app/mnist/test_images` samples.
//! `.bin` layout matches `Inference::parse_image_binary`: 8 reserved bytes, byte `[8]` = label, `[9..793]` row-major pixels.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![expect(rustdoc::missing_crate_level_docs)]

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use eframe::egui::{
    self, Color32, Pos2, Rect, Sense, Stroke, StrokeKind, vec2,
    PointerButton,
};

const GRID: usize = 28;
const BIN_LEN: usize = 8 + 1 + GRID * GRID; // 793

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
            if let Some(name) = ent.file_name().to_str() {
                if let Some(n) = parse_saved_index(name) {
                    max_ix = Some(max_ix.map_or(n, |m| m.max(n)));
                }
            }
        }
    }
    max_ix.map_or(0, |m| m.saturating_add(1))
}

fn write_txt(path: &Path, image_index: u32, label: u8, pixels: &[u8; GRID * GRID]) -> std::io::Result<()> {
    let mut f = File::create(path)?;
    writeln!(f, "Image Index: {}", image_index)?;
    writeln!(f, "True Label: {}", label)?;
    writeln!(f, "Image Data (28x28):")?;
    for row in 0..GRID {
        for col in 0..GRID {
            let v = pixels[row * GRID + col];
            write!(f, "{:>3}", v)?;
            if col + 1 < GRID {
                f.write_all(b" ")?;
            }
        }
        writeln!(f)?;
    }
    Ok(())
}

fn write_bin(path: &Path, label: u8, pixels: &[u8; GRID * GRID]) -> std::io::Result<()> {
    let mut buf = [0u8; BIN_LEN];
    buf[8] = label;
    buf[9..].copy_from_slice(pixels);
    fs::write(path, &buf)
}

fn paint_stamp(
    pixels: &mut [u8; GRID * GRID],
    cx: i32,
    cy: i32,
    radius: i32,
    ink: bool,
) {
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy > radius * radius {
                continue;
            }
            let x = cx + dx;
            let y = cy + dy;
            if (0..GRID as i32).contains(&x) && (0..GRID as i32).contains(&y) {
                let i = (y as usize) * GRID + (x as usize);
                if ink {
                    pixels[i] = (pixels[i].saturating_add(85)).min(255);
                } else {
                    pixels[i] = pixels[i].saturating_sub(96);
                }
            }
        }
    }
}

fn pointer_to_cell(pos: Pos2, rect: Rect) -> Option<(i32, i32)> {
    if !rect.contains(pos) {
        return None;
    }
    let w = rect.width() / GRID as f32;
    let h = rect.height() / GRID as f32;
    let gx = ((pos.x - rect.min.x) / w).floor() as i32;
    let gy = ((pos.y - rect.min.y) / h).floor() as i32;
    if (0..GRID as i32).contains(&gx) && (0..GRID as i32).contains(&gy) {
        Some((gx, gy))
    } else {
        None
    }
}

fn main() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([560.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native(
        "MNIST digit capture",
        options,
        Box::new(|_cc| Ok(Box::<MnistDrawApp>::default())),
    )
}

struct MnistDrawApp {
    pixels: [u8; GRID * GRID],
    label: u8,
    brush_radius: i32,
    status: String,
    canvas_size: f32,
}

impl Default for MnistDrawApp {
    fn default() -> Self {
        Self {
            pixels: [0u8; GRID * GRID],
            label: 0,
            brush_radius: 1,
            status: "Hold left button to draw, right to erase. Save writes to remu_app/mnist/test_images/.".into(),
            canvas_size: 560.0,
        }
    }
}

impl eframe::App for MnistDrawApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Label (0-9):");
                ui.add(egui::DragValue::new(&mut self.label).range(0..=9).speed(0.2));
                ui.separator();
                ui.label("Brush radius:");
                ui.add(egui::Slider::new(&mut self.brush_radius, 0..=3));
                ui.separator();
                if ui.button("Clear canvas").clicked() {
                    self.pixels.fill(0);
                    self.status = "Canvas cleared.".into();
                }
                if ui.button("Save to test_images").clicked() {
                    self.status = match self.save_pair() {
                        Ok(msg) => msg,
                        Err(e) => format!("Save failed: {e}"),
                    };
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(&self.status);
            ui.add_space(8.0);

            let size = egui::vec2(self.canvas_size, self.canvas_size);
            let (response, painter) = ui.allocate_painter(size, Sense::click_and_drag());
            let rect = response.rect;

            for row in 0..GRID {
                for col in 0..GRID {
                    let v = self.pixels[row * GRID + col];
                    let cell = Rect::from_min_size(
                        rect.min + vec2(col as f32 * rect.width() / GRID as f32, row as f32 * rect.height() / GRID as f32),
                        vec2(rect.width() / GRID as f32, rect.height() / GRID as f32),
                    );
                    painter.rect_filled(cell, 0.0, Color32::from_gray(v));
                }
            }
            painter.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::GRAY), StrokeKind::Inside);

            if let Some(pos) = response.interact_pointer_pos() {
                let primary = ctx.input(|i| i.pointer.button_down(PointerButton::Primary));
                let secondary = ctx.input(|i| i.pointer.button_down(PointerButton::Secondary));
                if primary {
                    if let Some((gx, gy)) = pointer_to_cell(pos, rect) {
                        paint_stamp(&mut self.pixels, gx, gy, self.brush_radius, true);
                    }
                }
                if secondary {
                    if let Some((gx, gy)) = pointer_to_cell(pos, rect) {
                        paint_stamp(&mut self.pixels, gx, gy, self.brush_radius, false);
                    }
                }
            }

            ui.add_space(8.0);
            ui.label("Tip: MNIST-style light strokes on dark background; draw multiple passes to thicken. Save writes matching .txt and .bin; rebuild the mnist app to embed new .bin files.");
        });

        ctx.request_repaint_after(std::time::Duration::from_millis(32));
    }
}

impl MnistDrawApp {
    fn save_pair(&self) -> Result<String, String> {
        let dir = test_images_dir();
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

        let ix = next_save_index(&dir);
        let base = format!("saved_image_{:05}", ix);
        let txt_path = dir.join(format!("{base}.txt"));
        let bin_path = dir.join(format!("{base}.bin"));

        write_txt(&txt_path, ix, self.label, &self.pixels).map_err(|e| e.to_string())?;
        write_bin(&bin_path, self.label, &self.pixels).map_err(|e| e.to_string())?;

        Ok(format!(
            "Saved: {} and {} (index={}, label={})",
            txt_path.display(),
            bin_path.display(),
            ix,
            self.label
        ))
    }
}
