use std::path::PathBuf;

fn main() {
    // Ensure dist-web directory exists so include_dir! does not panic.
    // In CI or fresh checkouts the web build may not have run yet.
    let dist_web = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ui/desktop/dist-web");
    if !dist_web.exists() {
        std::fs::create_dir_all(&dist_web).expect("failed to create dist-web placeholder");
    }
}
