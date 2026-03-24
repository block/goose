use super::SystemAutomation;
use std::path::PathBuf;
use std::process::Command;

pub struct MacOSAutomation;

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
}
