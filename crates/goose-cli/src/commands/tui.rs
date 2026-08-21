use anyhow::{anyhow, Context, Result};
use std::path::Path;
use std::process::Command;

const TUI_NPM_SPEC_ENV: &str = "GOOSE_TUI_NPM_SPEC";
const DEFAULT_NPM_SPEC: &str = "@aaif/goose@latest";
const NPM_BIN_NAME: &str = "goose-tui";

fn resolve_npm_spec() -> String {
    std::env::var(TUI_NPM_SPEC_ENV).unwrap_or_else(|_| DEFAULT_NPM_SPEC.to_string())
}

fn build_command(spec: &str, args: &[String], goose_binary: &Path) -> Command {
    let mut cmd = Command::new("npx");
    cmd.arg("--yes")
        .arg("--package")
        .arg(spec)
        .arg("--")
        .arg(NPM_BIN_NAME)
        .args(args)
        .env("GOOSE_BINARY", goose_binary);
    cmd
}

pub fn handle_tui(args: Vec<String>) -> Result<()> {
    let spec = resolve_npm_spec();

    let goose_binary = std::env::current_exe()
        .context("could not determine current goose executable to expose as GOOSE_BINARY")?;

    let mut cmd = build_command(&spec, &args, &goose_binary);
    let descriptor = format!("npx --package {} -- {}", spec, NPM_BIN_NAME);

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
    use std::ffi::OsStr;
    #[cfg(unix)]
    use std::fs;

    #[test]
    fn resolve_npm_spec_uses_default() {
        let _guard = env_lock::lock_env([(TUI_NPM_SPEC_ENV, None::<&str>)]);

        assert_eq!(resolve_npm_spec(), DEFAULT_NPM_SPEC);
    }

    #[test]
    fn resolve_npm_spec_honors_override() {
        let _guard = env_lock::lock_env([(TUI_NPM_SPEC_ENV, Some("@example/tui@1.2.3"))]);

        assert_eq!(resolve_npm_spec(), "@example/tui@1.2.3");
    }

    #[test]
    fn build_command_forwards_arguments_and_goose_binary() {
        let args = vec!["--server".to_string(), "http://localhost:3000".to_string()];
        let goose_binary = Path::new("/trusted/bin/goose");
        let command = build_command("@example/tui@1.2.3", &args, goose_binary);

        assert_eq!(command.get_program(), OsStr::new("npx"));
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            [
                "--yes",
                "--package",
                "@example/tui@1.2.3",
                "--",
                "goose-tui",
                "--server",
                "http://localhost:3000",
            ]
            .map(OsStr::new)
        );
        assert_eq!(
            command
                .get_envs()
                .find(|(key, _)| *key == OsStr::new("GOOSE_BINARY")),
            Some((OsStr::new("GOOSE_BINARY"), Some(goose_binary.as_os_str())))
        );
    }

    #[cfg(unix)]
    #[test]
    fn npx_command_ignores_script_in_world_writable_ancestor() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let shared = temp_dir.path().join("shared");
        let executable = shared.join("victim/target/debug/goose");
        let planted_script = shared.join("ui/text/dist/tui.js");
        fs::create_dir_all(executable.parent().unwrap()).expect("create executable dir");
        fs::create_dir_all(planted_script.parent().unwrap()).expect("create script directory");
        fs::write(&planted_script, "process.exit(0)\n").expect("write planted script");

        let mut permissions = fs::metadata(&shared)
            .expect("read shared directory")
            .permissions();
        permissions.set_mode(0o1777);
        fs::set_permissions(&shared, permissions).expect("make ancestor world writable");

        let command = build_command(DEFAULT_NPM_SPEC, &[], &executable);
        assert_eq!(command.get_program(), OsStr::new("npx"));
        assert!(!command.get_args().any(|arg| arg == planted_script));
    }
}
