use goose_acp::server::GooseAcpAgent;
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let schemas = GooseAcpAgent::custom_method_schemas();
    let json = serde_json::to_string_pretty(&schemas).expect("failed to serialize schemas");

    let package_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let output_path = PathBuf::from(package_dir).join("acp-schema.json");

    fs::write(&output_path, format!("{json}\n")).expect("failed to write schema file");
    eprintln!(
        "Generated ACP custom method schemas at {}",
        output_path.display()
    );

    println!("{json}");
}
