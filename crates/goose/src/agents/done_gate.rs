use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub message: String,
    pub details: Option<String>,
}

impl CheckResult {
    pub fn pass(name: &str, message: &str) -> Self {
        Self {
            name: name.to_string(),
            passed: true,
            message: message.to_string(),
            details: None,
        }
    }

    pub fn fail(name: &str, message: &str) -> Self {
        Self {
            name: name.to_string(),
            passed: false,
            message: message.to_string(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: &str) -> Self {
        self.details = Some(details.to_string());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GateResult {
    Done,
    ReEnterFix { check_name: String, message: String },
    Failed { reason: String },
}

pub trait DoneCheck: Send + Sync {
    fn name(&self) -> &str;
    fn check(&self, workspace: &Path) -> Result<CheckResult>;
}

pub struct BuildSucceeds {
    command: String,
}

impl BuildSucceeds {
    pub fn cargo() -> Self {
        Self {
            command: "cargo build".to_string(),
        }
    }

    pub fn npm() -> Self {
        Self {
            command: "npm run build".to_string(),
        }
    }

    pub fn custom(cmd: &str) -> Self {
        Self {
            command: cmd.to_string(),
        }
    }
}

impl DoneCheck for BuildSucceeds {
    fn name(&self) -> &str {
        "build_succeeds"
    }

    fn check(&self, workspace: &Path) -> Result<CheckResult> {
        let parts: Vec<&str> = self.command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(CheckResult::fail(self.name(), "Empty build command"));
        }

        let output = Command::new(parts[0])
            .args(&parts[1..])
            .current_dir(workspace)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                Ok(CheckResult::pass(self.name(), "Build succeeded"))
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                Ok(CheckResult::fail(self.name(), "Build failed").with_details(&stderr))
            }
            Err(e) => Ok(CheckResult::fail(
                self.name(),
                &format!("Failed to run build: {}", e),
            )),
        }
    }
}

pub struct TestsPass {
    command: String,
}

impl TestsPass {
    pub fn cargo() -> Self {
        Self {
            command: "cargo test".to_string(),
        }
    }

    pub fn npm() -> Self {
        Self {
            command: "npm test".to_string(),
        }
    }

    pub fn pytest() -> Self {
        Self {
            command: "pytest".to_string(),
        }
    }

    pub fn custom(cmd: &str) -> Self {
        Self {
            command: cmd.to_string(),
        }
    }
}

impl DoneCheck for TestsPass {
    fn name(&self) -> &str {
        "tests_pass"
    }

    fn check(&self, workspace: &Path) -> Result<CheckResult> {
        let parts: Vec<&str> = self.command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(CheckResult::fail(self.name(), "Empty test command"));
        }

        let output = Command::new(parts[0])
            .args(&parts[1..])
            .current_dir(workspace)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                Ok(CheckResult::pass(self.name(), "All tests passed"))
            }
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let details = format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr);
                Ok(CheckResult::fail(self.name(), "Tests failed").with_details(&details))
            }
            Err(e) => Ok(CheckResult::fail(
                self.name(),
                &format!("Failed to run tests: {}", e),
            )),
        }
    }
}

pub struct LinterPasses {
    command: String,
}

impl LinterPasses {
    pub fn cargo_fmt() -> Self {
        Self {
            command: "cargo fmt --check".to_string(),
        }
    }

    pub fn eslint() -> Self {
        Self {
            command: "npx eslint .".to_string(),
        }
    }

    pub fn custom(cmd: &str) -> Self {
        Self {
            command: cmd.to_string(),
        }
    }
}

impl DoneCheck for LinterPasses {
    fn name(&self) -> &str {
        "linter_passes"
    }

    fn check(&self, workspace: &Path) -> Result<CheckResult> {
        let parts: Vec<&str> = self.command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(CheckResult::fail(self.name(), "Empty lint command"));
        }

        let output = Command::new(parts[0])
            .args(&parts[1..])
            .current_dir(workspace)
            .output();

        match output {
            Ok(out) if out.status.success() => Ok(CheckResult::pass(self.name(), "Linting passed")),
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                Ok(CheckResult::fail(self.name(), "Linting failed").with_details(&stdout))
            }
            Err(e) => Ok(CheckResult::fail(
                self.name(),
                &format!("Failed to run linter: {}", e),
            )),
        }
    }
}

pub struct NoStubMarkers;

impl NoStubMarkers {
    const STUB_PATTERNS: &'static [&'static str] = &[
        "TODO",
        "FIXME",
        "STUB",
        "unimplemented!()",
        "todo!()",
        "pass  #",
        "raise NotImplementedError",
        "throw new Error('Not implemented')",
        "// ...",
        "/* ... */",
    ];
}

impl DoneCheck for NoStubMarkers {
    fn name(&self) -> &str {
        "no_stub_markers"
    }

    fn check(&self, workspace: &Path) -> Result<CheckResult> {
        let mut found_stubs = Vec::new();

        fn scan_file(path: &Path, patterns: &[&str], found: &mut Vec<String>) -> Result<()> {
            if let Ok(content) = std::fs::read_to_string(path) {
                for (line_num, line) in content.lines().enumerate() {
                    for pattern in patterns {
                        if line.contains(pattern) {
                            found.push(format!(
                                "{}:{}: {}",
                                path.display(),
                                line_num + 1,
                                line.trim()
                            ));
                        }
                    }
                }
            }
            Ok(())
        }

        fn scan_dir(dir: &Path, patterns: &[&str], found: &mut Vec<String>) -> Result<()> {
            if !dir.is_dir() {
                return Ok(());
            }

            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.starts_with('.')
                    || name == "node_modules"
                    || name == "target"
                    || name == "venv"
                {
                    continue;
                }

                if path.is_dir() {
                    scan_dir(&path, patterns, found)?;
                } else if path.is_file() {
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if matches!(
                        ext,
                        "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "go" | "java" | "kt"
                    ) {
                        scan_file(&path, patterns, found)?;
                    }
                }
            }
            Ok(())
        }

        scan_dir(workspace, Self::STUB_PATTERNS, &mut found_stubs)?;

        if found_stubs.is_empty() {
            Ok(CheckResult::pass(self.name(), "No stub markers found"))
        } else {
            let details = found_stubs.join("\n");
            Ok(CheckResult::fail(
                self.name(),
                &format!("Found {} stub markers", found_stubs.len()),
            )
            .with_details(&details))
        }
    }
}

pub struct DoneGate {
    checks: Vec<Box<dyn DoneCheck>>,
}

impl DoneGate {
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    pub fn with_check<C: DoneCheck + 'static>(mut self, check: C) -> Self {
        self.checks.push(Box::new(check));
        self
    }

    pub fn rust_defaults() -> Self {
        Self::new()
            .with_check(BuildSucceeds::cargo())
            .with_check(TestsPass::cargo())
            .with_check(LinterPasses::cargo_fmt())
            .with_check(NoStubMarkers)
    }

    pub fn node_defaults() -> Self {
        Self::new()
            .with_check(BuildSucceeds::npm())
            .with_check(TestsPass::npm())
            .with_check(LinterPasses::eslint())
            .with_check(NoStubMarkers)
    }

    pub fn python_defaults() -> Self {
        Self::new()
            .with_check(TestsPass::pytest())
            .with_check(NoStubMarkers)
    }

    pub fn verify(&self, workspace: &Path) -> Result<(GateResult, Vec<CheckResult>)> {
        let mut results = Vec::new();

        for check in &self.checks {
            info!("Running check: {}", check.name());
            let result = check.check(workspace)?;

            if !result.passed {
                warn!("Check failed: {} - {}", result.name, result.message);
                results.push(result.clone());
                return Ok((
                    GateResult::ReEnterFix {
                        check_name: result.name.clone(),
                        message: result.message.clone(),
                    },
                    results,
                ));
            }

            info!("Check passed: {}", check.name());
            results.push(result);
        }

        Ok((GateResult::Done, results))
    }

    pub fn check_count(&self) -> usize {
        self.checks.len()
    }
}

impl Default for DoneGate {
    fn default() -> Self {
        Self::rust_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_no_stub_markers_pass() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("main.rs");
        fs::write(&file, "fn main() { println!(\"Hello\"); }").unwrap();

        let check = NoStubMarkers;
        let result = check.check(dir.path()).unwrap();
        assert!(result.passed);
    }

    #[test]
    fn test_no_stub_markers_fail() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("main.rs");
        fs::write(&file, "fn main() { todo!(); }").unwrap();

        let check = NoStubMarkers;
        let result = check.check(dir.path()).unwrap();
        assert!(!result.passed);
        assert!(result.details.unwrap().contains("todo!()"));
    }

    #[test]
    fn test_done_gate_builder() {
        let gate = DoneGate::new()
            .with_check(NoStubMarkers)
            .with_check(BuildSucceeds::cargo());

        assert_eq!(gate.check_count(), 2);
    }
}
