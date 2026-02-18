use super::SystemAutomation;
use std::path::PathBuf;
use std::process::Command;

pub struct MacOSAutomation;

impl MacOSAutomation {
    /// Check if the Peekaboo CLI tool is available on PATH
    pub fn is_peekaboo_available() -> bool {
        Command::new("which")
            .arg("peekaboo")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Run a Peekaboo CLI command with the given arguments.
    /// Returns (stdout, stderr, success).
    pub fn run_peekaboo(args: &[&str]) -> std::io::Result<(String, String, bool)> {
        let output = Command::new("peekaboo").args(args).output()?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        Ok((stdout, stderr, output.status.success()))
    }
}

impl SystemAutomation for MacOSAutomation {
    fn execute_system_script(&self, script: &str) -> std::io::Result<String> {
        let output = Command::new("osascript").arg("-e").arg(script).output()?;

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn get_shell_command(&self) -> (&'static str, &'static str) {
        ("bash", "-c")
    }

    fn get_temp_path(&self) -> PathBuf {
        PathBuf::from("/tmp")
    }

    fn has_peekaboo(&self) -> bool {
        MacOSAutomation::is_peekaboo_available()
    }
}
