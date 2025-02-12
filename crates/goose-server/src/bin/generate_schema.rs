use goose_server::openapi;
use std::env;
use std::fs;

fn main() {
    let schema = openapi::generate_schema();

    // Get the current working directory
    let current_dir = env::current_dir().unwrap();
    let output_path = current_dir.join("ui").join("desktop").join("openapi.json");

    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    fs::write(&output_path, schema).unwrap();
    println!(
        "Successfully generated OpenAPI schema at {}",
        output_path.display()
    );
}
