use std::process::Command;

use anyhow::Result;

const DOWNLOAD_SCRIPT_URL: &str =
    "https://github.com/block/goose/releases/download/stable/download_cli.sh";

pub fn update(canary: bool, reconfigure: bool) -> Result<()> {
    // Get the download script from github
    let curl_output = Command::new("curl")
        .arg("-fsSL")
        .arg(DOWNLOAD_SCRIPT_URL)
        .output()?;

    if !curl_output.status.success() {
        anyhow::bail!(
            "Failed to download update script: {}",
            std::str::from_utf8(&curl_output.stderr)?
        );
    }

    let shell_str = std::str::from_utf8(&curl_output.stdout)?;

    let update = Command::new("bash")
        .arg("-c")
        .arg(shell_str)
        .env("CANARY", canary.to_string())
        .env("CONFIGURE", reconfigure.to_string())
        .env("GOOSE_TERMINAL", "1")
        .spawn()?;

    update.wait_with_output()?;

    Ok(())
}
