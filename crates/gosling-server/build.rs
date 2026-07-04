// We'll generate the schema at runtime since we need access to the complete application context
fn main() {
    println!("cargo:rerun-if-changed=src/");
}
