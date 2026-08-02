use std::{
    env::{self},
    ffi::{OsStr, OsString},
    path::PathBuf,
};

use anyhow::{Context, Result};

use crate::config::Config;

pub struct SearchPaths {
    paths: Vec<PathBuf>,
}

impl SearchPaths {
    pub fn builder() -> Self {
        let mut paths = Config::global()
            .get_goose_search_paths()
            .unwrap_or_default();

        paths.push("~/.local/bin".into());

        #[cfg(unix)]
        {
            paths.push("/usr/local/bin".into());
        }

        if cfg!(target_os = "macos") {
            paths.push("/opt/homebrew/bin".into());
            paths.push("/opt/local/bin".into());
        }

        Self {
            paths: paths
                .into_iter()
                .map(|s| PathBuf::from(&*shellexpand::tilde(&s)))
                .collect(),
        }
    }

    pub fn with_npm(mut self) -> Self {
        if cfg!(windows) {
            if let Some(appdata) = dirs::data_dir() {
                self.paths.push(appdata.join("npm"));
            }
        } else if let Some(home) = dirs::home_dir() {
            self.paths.push(home.join(".npm-global/bin"));
        }
        self
    }

    pub fn path(self) -> Result<OsString> {
        env::join_paths(
            self.paths.into_iter().chain(
                env::var_os("PATH")
                    .as_ref()
                    .map(env::split_paths)
                    .into_iter()
                    .flatten(),
            ),
        )
        .map_err(Into::into)
    }

    pub fn resolve<N>(self, name: N) -> Result<PathBuf>
    where
        N: AsRef<OsStr>,
    {
        let name = name.as_ref();
        let path = self.path()?;
        let found = which::which_in_global(name, Some(path.clone()))?
            .next()
            .with_context(|| {
                format!(
                    "could not resolve command '{}': file does not exist",
                    name.to_string_lossy()
                )
            })?;

        // On Windows, `which` may return an extensionless shell-script shim
        // (e.g. the npm-generated `claude-agent-acp`) which is not a valid
        // Win32 executable. Spawning it via `std::process::Command` fails
        // with "%1 is not a valid Win32 application", and the error can be
        // swallowed causing a silent hang (see issue #10872).
        //
        // When the resolved path lacks an executable extension, prefer the
        // PATHEXT sibling (`.cmd`, `.bat`, `.exe`) in the same directory.
        #[cfg(windows)]
        {
            if found.extension().is_none() {
                for ext in ["cmd", "bat", "exe"] {
                    let candidate = found.with_extension(ext);
                    if candidate.is_file() {
                        tracing::debug!(
                            original = %found.display(),
                            resolved = %candidate.display(),
                            "resolved Windows executable extension"
                        );
                        return Ok(candidate);
                    }
                }
            }
        }

        Ok(found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_preserves_existing_path() {
        let search_paths = SearchPaths::builder();
        let combined_path = search_paths.path().unwrap();

        if let Some(existing_path) = env::var_os("PATH") {
            let combined_str = combined_path.to_string_lossy();
            let existing_str = existing_path.to_string_lossy();

            assert!(combined_str.contains(&existing_str.to_string()));
        }
    }

    #[test]
    fn test_resolve_nonexistent_executable() {
        let search_paths = SearchPaths::builder();

        let result = search_paths.resolve("nonexistent_executable_12345_abcdef");

        assert!(
            result.is_err(),
            "Resolving nonexistent executable should return an error"
        );
    }

    #[test]
    fn test_resolve_common_executable() {
        let search_paths = SearchPaths::builder();

        #[cfg(unix)]
        let test_executable = "sh";

        #[cfg(windows)]
        let test_executable = "cmd";

        search_paths
            .resolve(test_executable)
            .expect("should resolve sh (or cmd on Windows)");
    }

    #[cfg(windows)]
    #[test]
    fn resolve_extensionless_shim_prefers_cmd() {
        use std::fs;
        let dir = tempfile::tempdir().unwrap();

        // Simulate an npm global install layout: extensionless shell script
        // plus a .cmd wrapper that Windows shells actually use.
        fs::write(dir.path().join("test-shim"), "#!/bin/sh\necho hello\n").unwrap();
        fs::write(dir.path().join("test-shim.cmd"), "@echo hello\r\n").unwrap();

        let found = which::which_in_global("test-shim", Some(dir.path().as_os_str()))
            .unwrap()
            .next()
            .expect("should find test-shim");

        // Mirror the resolve() fix: if the resolved path has no extension,
        // the .cmd sibling should exist and be preferred.
        if found.extension().is_none() {
            let cmd = found.with_extension("cmd");
            assert!(cmd.is_file(), ".cmd sibling should exist");
        }
    }
}
