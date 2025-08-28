use goose_server::openapi;
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let schema = openapi::generate_schema();

    let package_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let output_path = PathBuf::from(package_dir)
        .join("..")
        .join("..")
        .join("ui")
        .join("desktop")
        .join("openapi.json");

    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    fs::write(&output_path, &schema).unwrap();
    eprintln!(
        "Successfully generated OpenAPI schema at {}",
        output_path.canonicalize().unwrap().display()
    );

    // Output the schema to stdout for piping
    println!("{}", schema);
}
