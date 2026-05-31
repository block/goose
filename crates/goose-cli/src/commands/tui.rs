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

    if let Ok(cwd) = std::env::current_dir() {
        let candidate = cwd.join(TUI_REL_PATH);
        if candidate.is_file() {
            return Some(candidate);
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
