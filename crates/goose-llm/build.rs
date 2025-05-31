fn main() {
    // Only rerun if relevant files change
    println!("cargo:rerun-if-changed=src/wasm.rs");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/model.rs");
    
    // Check if we're compiling for wasm32
    #[cfg(target_arch = "wasm32")]
    {
        // Add any wasm-specific build steps here if needed
    }
}