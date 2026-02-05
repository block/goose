use std::path::Path;
use tokio::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW_FLAG: u32 = 0x08000000;

#[allow(unused_variables)]
pub fn configure_command_no_window(command: &mut Command) {
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW_FLAG);
}

/// Create a new command, wrapping it in a shell on Windows if it's a script (.cmd, .bat, .ps1)
pub fn create_command(path: &Path) -> Command {
    if cfg!(windows) {
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        if extension.eq_ignore_ascii_case("cmd") || extension.eq_ignore_ascii_case("bat") {
            let mut cmd = Command::new("cmd.exe");
            cmd.arg("/c");
            cmd.arg(path);
            cmd
        } else if extension.eq_ignore_ascii_case("ps1") {
            let mut cmd = Command::new("powershell.exe");
            cmd.arg("-ExecutionPolicy");
            cmd.arg("Bypass");
            cmd.arg("-File");
            cmd.arg(path);
            cmd
        } else {
            Command::new(path)
        }
    } else {
        Command::new(path)
    }
}
