use std::path::PathBuf;
use std::process::Command;
use std::sync::LazyLock;

/// Build a binary from a package and return its path.
pub fn build_binary(package: &str, bin_name: &str) -> PathBuf {
    let output = Command::new("cargo")
        .args([
            "build",
            "-p",
            package,
            "--bin",
            bin_name,
            "--message-format=json",
        ])
        .output()
        .expect("failed to build binary");

    if !output.status.success() {
        panic!("build failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|msg| msg["reason"] == "compiler-artifact")
        .filter(|msg| msg["target"]["name"] == bin_name)
        .filter(|msg| {
            msg["target"]["kind"]
                .as_array()
                .map(|k| k.iter().any(|v| v == "bin"))
                .unwrap_or(false)
        })
        .filter_map(|msg| msg["executable"].as_str().map(PathBuf::from))
        .next()
        .expect("failed to find binary path in cargo output")
}

#[allow(dead_code)]
pub static GOOSE_BINARY: LazyLock<PathBuf> = LazyLock::new(|| build_binary("goose-cli", "goose"));
#[allow(dead_code)]
pub static CAPTURE_BINARY: LazyLock<PathBuf> =
    LazyLock::new(|| build_binary("goose-test", "capture"));
