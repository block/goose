use anyhow::Result;
use goose::config::ExtensionConfig;

#[derive(Debug)]
pub struct InputConfig {
    pub contents: Option<String>,
    pub extensions_override: Option<Vec<ExtensionConfig>>,
    pub additional_system_prompt: Option<String>,
}

pub fn parse_key_val(s: &str) -> Result<(String, String), String> {
    match s.split_once('=') {
        Some((key, value)) => Ok((key.to_string(), value.to_string())),
        None => Err(format!("invalid KEY=VALUE: {}", s)),
    }
}
