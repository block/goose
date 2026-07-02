use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use std::io::Write;
use std::process::{Command, Stdio};

/// Most terminals cap OSC 52 payloads around 100KB of base64; beyond that the
/// copy silently truncates, which is worse than failing.
const OSC52_MAX_BYTES: usize = 100_000;

/// Copy text to the system clipboard, returning the mechanism that worked.
/// Tries the platform's clipboard command first, then falls back to the
/// OSC 52 escape sequence so copying still works over SSH.
pub fn copy(text: &str) -> Result<&'static str> {
    for (name, command, args) in platform_commands() {
        if copy_via_command(command, args, text).is_ok() {
            return Ok(name);
        }
    }
    copy_via_osc52(text)
}

fn platform_commands() -> &'static [(&'static str, &'static str, &'static [&'static str])] {
    if cfg!(target_os = "macos") {
        &[("pbcopy", "pbcopy", &[])]
    } else if cfg!(target_os = "windows") {
        &[("clip", "clip", &[])]
    } else {
        &[
            ("wl-copy", "wl-copy", &[]),
            ("xclip", "xclip", &["-selection", "clipboard"]),
            ("xsel", "xsel", &["--clipboard", "--input"]),
        ]
    }
}

fn copy_via_command(command: &str, args: &[&str], text: &str) -> Result<()> {
    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("failed to open stdin for {command}"))?
        .write_all(text.as_bytes())?;
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("{command} exited with {status}"))
    }
}

fn copy_via_osc52(text: &str) -> Result<&'static str> {
    if text.len() > OSC52_MAX_BYTES {
        return Err(anyhow!(
            "no clipboard tool found and the response is too large for the OSC 52 fallback"
        ));
    }
    let mut stdout = std::io::stdout();
    stdout.write_all(osc52_sequence(text).as_bytes())?;
    stdout.flush()?;
    Ok("OSC 52")
}

fn osc52_sequence(text: &str) -> String {
    let encoded = general_purpose::STANDARD.encode(text.as_bytes());
    format!("\x1b]52;c;{encoded}\x07")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osc52_sequence_encodes_payload() {
        assert_eq!(osc52_sequence("hi"), "\x1b]52;c;aGk=\x07");
    }

    #[test]
    fn osc52_rejects_oversized_payloads() {
        let big = "a".repeat(OSC52_MAX_BYTES + 1);
        assert!(copy_via_osc52(&big).is_err());
    }
}
