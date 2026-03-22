use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("embedded_images.rs");

    let test_images_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("./test_images");
    println!("cargo:rerun-if-changed={}", test_images_dir.display());

    // Get all .bin files from the test images directory
    let mut bin_files = Vec::new();

    if let Ok(entries) = fs::read_dir(&test_images_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "bin") {
                    if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                        bin_files.push(file_name.to_string());
                    }
                }
            }
        }
    }

    // Sort files to ensure consistent ordering
    bin_files.sort();

    let mut output = String::new();

    // Generate the embedded images array with direct byte arrays
    output.push_str("pub const EMBEDDED_TEST_IMAGES: &[&[u8]] = &[\n");

    for file_name in &bin_files {
        let file_path = test_images_dir.join(file_name);
        if let Ok(mut file) = File::open(&file_path) {
            let mut buffer = Vec::new();
            if file.read_to_end(&mut buffer).is_ok() {
                // Convert bytes to a Rust byte array literal
                output.push_str("    &[\n");
                for chunk in buffer.chunks(16) {
                    output.push_str("        ");
                    for &byte in chunk {
                        output.push_str(&format!("0x{:02x}, ", byte));
                    }
                    output.push_str("\n");
                }
                output.push_str("    ],\n");
            }
        }
    }

    output.push_str("];\n\n");

    // Generate file names for debugging
    output.push_str("pub const EMBEDDED_TEST_IMAGE_NAMES: &[&str] = &[\n");
    for file_name in &bin_files {
        output.push_str(&format!("    \"{}\",\n", file_name));
    }
    output.push_str("];\n\n");

    // Generate count constant
    output.push_str(&format!(
        "pub const EMBEDDED_TEST_IMAGE_COUNT: usize = {};\n",
        bin_files.len()
    ));

    // Write the generated file
    let mut file = File::create(&dest_path).unwrap();
    file.write_all(output.as_bytes()).unwrap();
}