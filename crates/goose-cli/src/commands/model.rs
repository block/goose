use anyhow::Result;
use goose::config::Config;
use goose::providers::providers;
use std::io::{self, IsTerminal, Write};

/// Output format for the model list command
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    Text,
    Json,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("text") {
            Ok(OutputFormat::Text)
        } else if s.eq_ignore_ascii_case("json") {
            Ok(OutputFormat::Json)
        } else {
            Err(format!("Invalid format: {}. Expected 'text' or 'json'", s))
        }
    }
}

/// A model entry for output
#[derive(Debug, Clone, serde::Serialize)]
pub struct ModelEntry {
    pub provider: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<usize>,
    pub is_default: bool,
}

/// Handle the model list command
pub async fn handle_list(
    filter: Option<&str>,
    provider_filter: Option<&str>,
    verbose: bool,
    format: OutputFormat,
    all: bool,
) -> Result<()> {
    let all_providers = providers().await;
    let mut entries: Vec<ModelEntry> = Vec::new();

    // Determine which provider(s) to show
    let effective_provider_filter: Option<String> = if let Some(pf) = provider_filter {
        // Explicit --provider flag takes precedence
        Some(pf.to_string())
    } else if all {
        // --all shows everything
        None
    } else {
        // Default: only show configured provider
        match Config::global().get_goose_provider() {
            Ok(p) => Some(p),
            Err(_) => {
                let msg = "No provider configured. Use '--all' to see all models or run 'goose configure'";
                match format {
                    OutputFormat::Json => println!(r#"{{"error": "{}"}}"#, msg),
                    OutputFormat::Text => eprintln!("{}", msg),
                }
                return Ok(());
            }
        }
    };

    // Precompute lowercased filter to avoid repeated allocations
    let filter_lower = filter.map(|f| f.to_lowercase());

    for (metadata, _provider_type) in all_providers {
        // Filter by provider
        if let Some(ref pf) = effective_provider_filter {
            if !metadata.name.eq_ignore_ascii_case(pf) {
                continue;
            }
        }

        for model_info in &metadata.known_models {
            // Filter by model name substring if specified
            if let Some(ref f) = filter_lower {
                if !model_info.name.to_lowercase().contains(f) {
                    continue;
                }
            }

            entries.push(ModelEntry {
                provider: metadata.name.clone(),
                model: model_info.name.clone(),
                context: Some(model_info.context_limit),
                is_default: model_info.name == metadata.default_model,
            });
        }
    }

    // Sort by provider, then model
    entries.sort_by(|a, b| {
        a.provider
            .cmp(&b.provider)
            .then_with(|| a.model.cmp(&b.model))
    });

    // Only show default marker when viewing a single provider
    let show_default_marker = effective_provider_filter.is_some();

    // Handle empty results consistently across formats
    if entries.is_empty() {
        match format {
            OutputFormat::Json => println!("[]"),
            OutputFormat::Text => eprintln!("No models found"),
        }
        return Ok(());
    }

    // Output based on format
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&entries)?);
        }
        OutputFormat::Text => {
            print_text_output(&entries, verbose, show_default_marker)?;
        }
    }

    Ok(())
}

fn print_text_output(
    entries: &[ModelEntry],
    verbose: bool,
    show_default_marker: bool,
) -> Result<()> {
    let mut stdout = io::stdout();
    let is_tty = stdout.is_terminal();

    if verbose {
        // Calculate column widths for alignment (add 2 for "* " prefix on defaults if showing marker)
        let prefix_width = if show_default_marker { 2 } else { 0 };
        let model_width = entries.iter().map(|e| e.model.len()).max().unwrap_or(20) + prefix_width;
        let provider_width = entries.iter().map(|e| e.provider.len()).max().unwrap_or(10);

        if is_tty {
            println!(
                "{:<model_width$}  {:<provider_width$}  {:>10}",
                "MODEL", "PROVIDER", "CONTEXT"
            );
        }

        for entry in entries {
            let context_str = entry
                .context
                .map(|c| format!("{}", c))
                .unwrap_or_else(|| "-".to_string());

            let model_display = match (show_default_marker, entry.is_default) {
                (true, true) => format!("* {}", entry.model),
                (true, false) => format!("  {}", entry.model),
                (false, _) => entry.model.clone(),
            };

            writeln!(
                stdout,
                "{:<model_width$}  {:<provider_width$}  {:>10}",
                model_display, entry.provider, context_str
            )?;
        }
    } else {
        // Simple format: provider:model per line
        for entry in entries {
            writeln!(stdout, "{}:{}", entry.provider, entry.model)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_from_str() {
        assert_eq!("text".parse::<OutputFormat>().unwrap(), OutputFormat::Text);
        assert_eq!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert_eq!("JSON".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert!("invalid".parse::<OutputFormat>().is_err());
    }

    #[test]
    fn test_model_entry_serialization() {
        let entry = ModelEntry {
            provider: "anthropic".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            context: Some(200000),
            is_default: true,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("anthropic"));
        assert!(json.contains("claude-sonnet-4-20250514"));
        assert!(json.contains("200000"));
        assert!(json.contains("is_default"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_model_entry_optional_fields_skipped() {
        let entry = ModelEntry {
            provider: "test".to_string(),
            model: "test-model".to_string(),
            context: None,
            is_default: false,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(!json.contains("context"));
    }
}
