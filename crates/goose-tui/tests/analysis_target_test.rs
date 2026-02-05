use goose_tui::analysis_target::detect_analysis_target;
use tempfile::TempDir;

#[test]
fn detect_analysis_target_finds_rust_workspace_crates() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("Cargo.toml"), "[workspace]\nmembers = []").unwrap();
    std::fs::create_dir(dir.path().join("crates")).unwrap();

    let (target, depth) = detect_analysis_target(dir.path());

    assert_eq!(target, dir.path().join("crates"));
    assert_eq!(depth, 3);
}

#[test]
fn detect_analysis_target_finds_monorepo_packages() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("pnpm-workspace.yaml"),
        "packages:\n  - packages/*",
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("packages")).unwrap();

    let (target, depth) = detect_analysis_target(dir.path());

    assert_eq!(target, dir.path().join("packages"));
    assert_eq!(depth, 3);
}

#[test]
fn detect_analysis_target_finds_src_directory() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();

    let (target, depth) = detect_analysis_target(dir.path());

    assert_eq!(target, dir.path().join("src"));
    assert_eq!(depth, 3);
}

#[test]
fn detect_analysis_target_falls_back_to_cwd() {
    let dir = TempDir::new().unwrap();

    let (target, depth) = detect_analysis_target(dir.path());

    assert_eq!(target, dir.path().to_path_buf());
    assert_eq!(depth, 1);
}

#[test]
fn detect_analysis_target_monorepo_without_known_dirs_uses_shallow_depth() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("Cargo.toml"), "[workspace]\nmembers = []").unwrap();

    let (target, depth) = detect_analysis_target(dir.path());

    assert_eq!(target, dir.path().to_path_buf());
    assert_eq!(depth, 1);
}

#[test]
fn detect_analysis_target_prefers_monorepo_dirs_over_src() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("Cargo.toml"), "[workspace]\nmembers = []").unwrap();
    std::fs::create_dir(dir.path().join("crates")).unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();

    let (target, _) = detect_analysis_target(dir.path());

    assert_eq!(target, dir.path().join("crates"));
}
