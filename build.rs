// build.rs
use std::fs;
use std::path::Path;

fn main() {
    // Path to your patterns directory (relative to CARGO_MANIFEST_DIR)
    let patterns_dir = Path::new("patterns/conwaylife/oscillators");
    let mut entries = Vec::new();

    if patterns_dir.exists() {
        for entry in fs::read_dir(patterns_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file()
                && let Some(ext) = path.extension()
                && (ext == "rle" || ext == "txt")
                && let Some(name) = path.file_name().and_then(|s| s.to_str())
            {
                entries.push(name.to_string());
            }
        }
    }

    // Sort for deterministic ordering
    entries.sort();

    // Generate a Rust file with a static slice
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("pattern_list.rs");
    let mut content = String::from("pub const PATTERN_FILES: &[&str] = &[\n");
    for name in &entries {
        content.push_str(&format!("    \"{}\",\n", name));
    }
    content.push_str("];\n");

    fs::write(&dest_path, content).unwrap();
    println!("cargo:rerun-if-changed=patterns/");
}
