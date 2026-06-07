use anyhow::{Context, Result};
use goose::conversation::message::Message;
use goose::conversation::Conversation;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

fn detect_editor() -> String {
    std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| {
            if cfg!(windows) {
                "notepad".to_string()
            } else {
                "vi".to_string()
            }
        })
}

pub fn edit_conversation(conversation: &Conversation) -> Result<Conversation> {
    let yaml = serde_yaml::to_string(conversation.messages())?;

    let mut tmp = NamedTempFile::with_suffix(".yaml")?;
    tmp.write_all(yaml.as_bytes())?;
    tmp.flush()?;

    let editor = detect_editor();
    let path = tmp.path().to_path_buf();

    let status = Command::new(&editor)
        .arg(&path)
        .status()
        .with_context(|| format!("failed to launch editor '{editor}'"))?;

    if !status.success() {
        anyhow::bail!("editor exited with non-zero status: {status}");
    }

    let edited = std::fs::read_to_string(&path)?;
    let messages: Vec<Message> =
        serde_yaml::from_str(&edited).context("invalid YAML — session unchanged")?;

    Ok(Conversation::new_unvalidated(messages))
}
