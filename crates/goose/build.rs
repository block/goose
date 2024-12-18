use std::env;
use std::process::Command;

fn main() {
    // Get the workspace root directory
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir)
        .parent() // go up to crates/
        .unwrap()
        .parent() // go up to workspace root
        .unwrap();

    println!("cargo:rerun-if-changed=../../download_tokenizers.sh");
    println!("cargo:rerun-if-changed=../../tokenizer_files");

    // Run the download_tokenizers.sh script
    let status = Command::new("./download_tokenizers.sh")
        .current_dir(workspace_root)
        .status()
        .expect("Failed to execute download_tokenizers.sh");

    if !status.success() {
        panic!("Failed to run download_tokenizers.sh");
    }
}
