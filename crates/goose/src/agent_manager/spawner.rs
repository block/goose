use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use crate::registry::manifest::{
    AgentDistribution, BinaryTarget, DockerDistribution, PackageDistribution,
};

/// A spawned agent process with stdio handles for ACP communication.
#[derive(Debug)]
pub struct SpawnedAgent {
    pub child: Child,
    pub stdin: ChildStdin,
    pub stdout: ChildStdout,
}

impl SpawnedAgent {
    /// Gracefully shut down the agent process.
    pub async fn shutdown(mut self) -> Result<()> {
        drop(self.stdin);
        drop(self.stdout);

        tokio::select! {
            status = self.child.wait() => {
                status?;
                Ok(())
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                self.child.kill().await?;
                Ok(())
            }
        }
    }
}

/// Spawn an ACP agent from a distribution specification.
///
/// Tries distribution strategies in priority order:
/// 1. Platform-specific binary
/// 2. npx (Node.js)
/// 3. uvx (Python)
/// 4. cargo (Rust)
/// 5. docker
pub async fn spawn_agent(dist: &AgentDistribution) -> Result<SpawnedAgent> {
    if let Some(target) = resolve_binary_target(&dist.binary) {
        return spawn_binary(target).await;
    }
    if let Some(npx) = &dist.npx {
        return spawn_package("npx", npx).await;
    }
    if let Some(uvx) = &dist.uvx {
        return spawn_package("uvx", uvx).await;
    }
    if let Some(cargo) = &dist.cargo {
        return spawn_cargo(cargo).await;
    }
    if let Some(docker) = &dist.docker {
        return spawn_docker(docker).await;
    }
    bail!("No suitable distribution target found for the current platform")
}

fn resolve_binary_target(binaries: &HashMap<String, BinaryTarget>) -> Option<&BinaryTarget> {
    let platform_key = current_platform_key();
    binaries.get(&platform_key)
}

pub fn current_platform_key() -> String {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        other => other,
    };
    let arch = std::env::consts::ARCH;
    format!("{os}-{arch}")
}

async fn spawn_binary(target: &BinaryTarget) -> Result<SpawnedAgent> {
    let mut cmd = Command::new(&target.cmd);
    cmd.args(&target.args);
    for (k, v) in &target.env {
        cmd.env(k, v);
    }
    spawn_with_stdio(cmd).await.context("spawning binary agent")
}

async fn spawn_package(runner: &str, pkg: &PackageDistribution) -> Result<SpawnedAgent> {
    let mut cmd = Command::new(runner);
    cmd.arg(&pkg.package);
    if let Some(args) = &pkg.args {
        cmd.args(args);
    }
    for (k, v) in &pkg.env {
        cmd.env(k, v);
    }
    spawn_with_stdio(cmd)
        .await
        .with_context(|| format!("spawning {runner} agent"))
}

async fn spawn_cargo(pkg: &PackageDistribution) -> Result<SpawnedAgent> {
    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("--package").arg(&pkg.package).arg("--");
    if let Some(args) = &pkg.args {
        cmd.args(args);
    }
    for (k, v) in &pkg.env {
        cmd.env(k, v);
    }
    spawn_with_stdio(cmd).await.context("spawning cargo agent")
}

async fn spawn_docker(docker: &DockerDistribution) -> Result<SpawnedAgent> {
    let image = match &docker.tag {
        Some(tag) => format!("{}:{}", docker.image, tag),
        None => docker.image.clone(),
    };
    let mut cmd = Command::new("docker");
    cmd.arg("run").arg("--rm").arg("-i");
    for (k, v) in &docker.env {
        cmd.arg("-e").arg(format!("{k}={v}"));
    }
    cmd.arg(&image);
    spawn_with_stdio(cmd).await.context("spawning docker agent")
}

async fn spawn_with_stdio(mut cmd: Command) -> Result<SpawnedAgent> {
    cmd.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit());

    let mut child = cmd.spawn()?;

    let stdin = child.stdin.take().expect("stdin was piped");
    let stdout = child.stdout.take().expect("stdout was piped");

    Ok(SpawnedAgent {
        child,
        stdin,
        stdout,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_key_is_reasonable() {
        let key = current_platform_key();
        assert!(
            key.contains('-'),
            "platform key should be os-arch, got: {key}"
        );
        let parts: Vec<&str> = key.split('-').collect();
        assert_eq!(parts.len(), 2);
        assert!(
            ["linux", "darwin", "windows"].contains(&parts[0]),
            "unexpected os: {}",
            parts[0]
        );
        assert!(
            ["x86_64", "aarch64"].contains(&parts[1]),
            "unexpected arch: {}",
            parts[1]
        );
    }

    #[test]
    fn no_distribution_returns_error() {
        let dist = AgentDistribution {
            binary: HashMap::new(),
            npx: None,
            uvx: None,
            cargo: None,
            docker: None,
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(spawn_agent(&dist));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No suitable distribution target"));
    }

    #[test]
    fn resolve_binary_target_matches_platform() {
        let key = current_platform_key();
        let target = BinaryTarget {
            archive: "https://example.com/agent.tar.gz".into(),
            cmd: "/usr/local/bin/agent".into(),
            args: vec!["--stdio".into()],
            env: HashMap::new(),
        };
        let mut binaries = HashMap::new();
        binaries.insert(key.clone(), target);
        assert!(resolve_binary_target(&binaries).is_some());

        let mut wrong = HashMap::new();
        wrong.insert(
            "fake-platform".into(),
            BinaryTarget {
                archive: String::new(),
                cmd: String::new(),
                args: vec![],
                env: HashMap::new(),
            },
        );
        assert!(resolve_binary_target(&wrong).is_none());
    }

    #[tokio::test]
    async fn spawn_echo_agent() {
        // Spawn 'cat' as a trivial agent — it echoes stdin to stdout
        let cmd_name = if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "cat"
        };
        let dist = AgentDistribution {
            binary: {
                let mut m = HashMap::new();
                let key = current_platform_key();
                m.insert(
                    key,
                    BinaryTarget {
                        archive: String::new(),
                        cmd: cmd_name.into(),
                        args: vec![],
                        env: HashMap::new(),
                    },
                );
                m
            },
            npx: None,
            uvx: None,
            cargo: None,
            docker: None,
        };
        let agent = spawn_agent(&dist).await.expect("should spawn cat");
        agent.shutdown().await.expect("should shut down cleanly");
    }
}
