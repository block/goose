use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

const TUI_NPM_SPEC_ENV: &str = "GOOSE_TUI_NPM_SPEC";
const TUI_REL_PATH: &str = "ui/text/dist/tui.js";
const DEFAULT_NPM_SPEC: &str = "@aaif/goose@latest";
const NPM_BIN_NAME: &str = "goose-tui";

enum TuiSource {
    LocalScript(PathBuf),
    Npx(String),
}

fn find_local_script() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    find_local_script_from(&exe)
}

fn find_local_script_from(exe: &Path) -> Option<PathBuf> {
    let exe_dir = exe.parent().unwrap_or_else(|| Path::new("."));

    let mut dir = Some(exe_dir.to_path_buf());
    for _ in 0..6 {
        if let Some(d) = dir.clone() {
            let candidate = d.join(TUI_REL_PATH);
            if candidate.is_file() {
                return Some(candidate);
            }
            dir = d.parent().map(Path::to_path_buf);
        }
    }

    None
}

fn resolve_source() -> TuiSource {
    if let Some(script) = find_local_script() {
        return TuiSource::LocalScript(script);
    }
    let spec = std::env::var(TUI_NPM_SPEC_ENV).unwrap_or_else(|_| DEFAULT_NPM_SPEC.to_string());
    TuiSource::Npx(spec)
}

fn build_command(source: &TuiSource, args: &[String]) -> Result<Command> {
    match source {
        TuiSource::LocalScript(script) => {
            let mut cmd = Command::new("node");
            cmd.arg(script).args(args);
            Ok(cmd)
        }
        TuiSource::Npx(spec) => {
            let mut cmd = Command::new("npx");
            cmd.arg("--yes")
                .arg("--package")
                .arg(spec)
                .arg("--")
                .arg(NPM_BIN_NAME)
                .args(args);
            Ok(cmd)
        }
    }
}

pub fn handle_tui(args: Vec<String>) -> Result<()> {
    let source = resolve_source();

    let goose_binary = std::env::current_exe()
        .context("could not determine current goose executable to expose as GOOSE_BINARY")?;

    let mut cmd = build_command(&source, &args)?;
    cmd.env("GOOSE_BINARY", &goose_binary);

    let descriptor = match &source {
        TuiSource::LocalScript(p) => format!("node {}", p.display()),
        TuiSource::Npx(spec) => format!("npx --package {} -- {}", spec, NPM_BIN_NAME),
    };

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = cmd.exec();
        Err(anyhow!("failed to exec TUI ({descriptor}): {err}"))
    }

    #[cfg(not(unix))]
    {
        let status = cmd
            .status()
            .with_context(|| format!("failed to run `{descriptor}`"))?;
        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn find_local_script_ignores_unrelated_directories() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let executable = temp_dir.path().join("install/bin/goose");
        let planted_script = temp_dir.path().join("checkout").join(TUI_REL_PATH);
        fs::create_dir_all(planted_script.parent().unwrap()).expect("create script directory");
        fs::write(&planted_script, "process.exit(0)\n").expect("write planted script");

        assert_eq!(find_local_script_from(&executable), None);
    }

    #[test]
    fn find_local_script_accepts_executable_ancestor() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let executable = temp_dir.path().join("target/debug/goose");
        let bundled_script = temp_dir.path().join(TUI_REL_PATH);
        fs::create_dir_all(bundled_script.parent().unwrap()).expect("create script directory");
        fs::write(&bundled_script, "process.exit(0)\n").expect("write bundled script");

        assert_eq!(
            find_local_script_from(&executable).as_deref(),
            Some(bundled_script.as_path())
        );
    }
}
