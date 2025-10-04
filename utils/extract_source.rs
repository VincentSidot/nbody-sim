//! Utility function to extract all the source code into a single file.

use std::fs;
use std::io::Write;
use std::path::Path;

const OUT_FILE: &str = "all_source_code.rs";

fn extract_source_code(dir: &Path, ext: &str, out_file: &mut dyn Write) -> std::io::Result<()> {
    println!(
        "Extracting source code from '{}' into '{}'",
        dir.display(),
        OUT_FILE
    );

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            extract_source_code(&path, ext, out_file)?;
        } else if let Some(read_ext) = path.extension() {
            if read_ext == ext {
                let content = fs::read_to_string(&path)?;
                writeln!(out_file, "// File: {}\n", path.display())?;
                writeln!(out_file, "{}", content)?;
                writeln!(out_file, "\n")?;
            }
        }
    }

    Ok(())
}

fn main() {
    let rust_dir = Path::new("src");
    let shader_dir = Path::new("shaders");
    let out_file = Path::new(OUT_FILE);
    let mut out_file = fs::File::create(out_file).expect("Failed to create output file");

    // Extract Rust source files
    if let Err(e) = extract_source_code(rust_dir, "rs", &mut out_file) {
        eprintln!("Error extracting source code: {}", e);
    }
    // Extract wgsl files
    if let Err(e) = extract_source_code(&shader_dir, "wgsl", &mut out_file) {
        eprintln!("Error extracting shader code: {}", e);
    }
    println!("Source code extraction completed successfully.");
}
