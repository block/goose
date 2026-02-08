use anyhow::Result;
use goose::config::Config;
use goose::providers::base::ConfigKey;
use goose::providers::canonical::{maybe_get_canonical_model, CanonicalModel, Modality};
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

fn determine_auth_method(config_keys: &[ConfigKey]) -> &'static str {
    if config_keys.iter().any(|k| k.oauth_flow) {
        "OAuth device flow"
    } else if config_keys.iter().any(|k| k.required && k.secret) {
        "API key/secret"
    } else if config_keys.iter().any(|k| k.required && !k.secret) {
        "Config params"
    } else {
        "None"
    }
}

fn format_token_cost(
    input_cost: Option<f64>,
    output_cost: Option<f64>,
    currency: Option<&str>,
) -> Option<String> {
    let currency = currency.unwrap_or("$");
    match (input_cost, output_cost) {
        (Some(input), Some(output)) => Some(format!(
            "input {}{} / output {}{}",
            currency, input, currency, output
        )),
        (Some(input), None) => Some(format!("input {}{}", currency, input)),
        (None, Some(output)) => Some(format!("output {}{}", currency, output)),
        (None, None) => None,
    }
}

fn format_modality(modality: Modality) -> &'static str {
    match modality {
        Modality::Text => "text",
        Modality::Image => "image",
        Modality::Audio => "audio",
        Modality::Video => "video",
        Modality::Pdf => "pdf",
    }
}

fn format_modalities(modalities: &[Modality]) -> String {
    if modalities.is_empty() {
        "-".to_string()
    } else {
        modalities
            .iter()
            .copied()
            .map(format_modality)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn print_canonical_model(canonical: &CanonicalModel) {
    println!("Canonical:");
    println!("  ID: {}", canonical.id);
    println!("  Name: {}", canonical.name);
    if let Some(family) = canonical.family.as_deref() {
        println!("  Family: {}", family);
    }
    if let Some(attachment) = canonical.attachment {
        println!(
            "  Supports Attachments: {}",
            if attachment { "yes" } else { "no" }
        );
    }
    if let Some(reasoning) = canonical.reasoning {
        println!(
            "  Supports Reasoning: {}",
            if reasoning { "yes" } else { "no" }
        );
    }
    println!(
        "  Tool Call: {}",
        if canonical.tool_call { "yes" } else { "no" }
    );
    if let Some(temperature) = canonical.temperature {
        println!(
            "  Supports Temperature: {}",
            if temperature { "yes" } else { "no" }
        );
    }
    if let Some(knowledge) = canonical.knowledge.as_deref() {
        println!("  Knowledge Cutoff: {}", knowledge);
    }
    if let Some(release_date) = canonical.release_date.as_deref() {
        println!("  Release Date: {}", release_date);
    }
    if let Some(last_updated) = canonical.last_updated.as_deref() {
        println!("  Last Updated: {}", last_updated);
    }
    println!(
        "  Modalities: input [{}], output [{}]",
        format_modalities(&canonical.modalities.input),
        format_modalities(&canonical.modalities.output)
    );
    if let Some(open_weights) = canonical.open_weights {
        println!(
            "  Open Weights: {}",
            if open_weights { "yes" } else { "no" }
        );
    }
    println!("  Limits: context {}", canonical.limit.context);
    if let Some(output) = canonical.limit.output {
        println!("  Limits: output {}", output);
    }
    if let Some(input) = canonical.cost.input {
        println!("  Cost: input ${}/1M tokens", input);
    }
    if let Some(output) = canonical.cost.output {
        println!("  Cost: output ${}/1M tokens", output);
    }
    if let Some(cache_read) = canonical.cost.cache_read {
        println!("  Cost: cache read ${}/1M tokens", cache_read);
    }
    if let Some(cache_write) = canonical.cost.cache_write {
        println!("  Cost: cache write ${}/1M tokens", cache_write);
    }
}

/// Helper function to print config keys for a provider
fn print_config_keys(config_keys: &[ConfigKey], config: &Config) {
    for key in config_keys {
        let value = if key.secret {
            None
        } else {
            config.get_param::<String>(&key.name).ok()
        };
        let is_set = if key.secret {
            config.get_secret::<String>(&key.name).is_ok()
        } else {
            value.is_some()
        };
        let status = if is_set { "set" } else { "unset" };
        let requirement = if key.required { "required" } else { "optional" };
        if key.secret {
            println!("  {} ({}, secret): {}", key.name, requirement, status);
        } else if let Some(value) = value {
            println!("  {} ({}, non-secret): {}", key.name, requirement, value);
        } else {
            println!("  {} ({}, non-secret): {}", key.name, requirement, status);
        }
    }
}

/// Show currently configured provider and model
pub async fn handle_show_current(verbose: bool) -> Result<()> {
    let config = Config::global();

    let Ok(provider) = config.get_goose_provider() else {
        eprintln!("No provider configured. Run 'goose configure' first.");
        return Ok(());
    };

    let Ok(model) = config.get_goose_model() else {
        eprintln!("No model configured. Run 'goose configure' first.");
        return Ok(());
    };

    let all_providers = providers().await;
    let metadata = all_providers
        .iter()
        .find_map(|(meta, _)| meta.name.eq_ignore_ascii_case(&provider).then_some(meta));

    println!("Provider: {}", provider);
    if let Some(meta) = metadata
        .filter(|meta| meta.display_name != meta.name)
        .map(|meta| &meta.display_name)
    {
        println!("Display Name: {}", meta);
    }
    if let Some(description) = metadata
        .filter(|meta| !meta.description.is_empty())
        .map(|meta| &meta.description)
    {
        println!("Description: {}", description);
    }

    if let Some(meta) = metadata {
        println!("Provider Default Model: {}", meta.default_model);

        if !meta.model_doc_link.is_empty() {
            println!("Model Docs: {}", meta.model_doc_link);
        }

        let auth_method = determine_auth_method(&meta.config_keys);
        println!("Auth Method: {}", auth_method);

        if !meta.config_keys.is_empty() {
            println!("Config Keys:");
            print_config_keys(&meta.config_keys, config);
        }
    }

    println!("Current Model:");
    println!("  Model: {}", model);
    let model_info = metadata.and_then(|meta| meta.known_models.iter().find(|m| m.name == model));
    if let Some(model_info) = model_info {
        println!("  Context Limit: {}", model_info.context_limit);
        if let Some(token_cost) = format_token_cost(
            model_info.input_token_cost,
            model_info.output_token_cost,
            model_info.currency.as_deref(),
        ) {
            println!("  Token Cost: {}", token_cost);
        }
        if let Some(cache_control) = model_info.supports_cache_control {
            println!(
                "  Supports Cache Control: {}",
                if cache_control { "yes" } else { "no" }
            );
        }
    } else {
        println!("  Context Limit: unknown");
    }

    if verbose {
        if let Some(canonical) = maybe_get_canonical_model(&provider, &model) {
            println!();
            print_canonical_model(&canonical);
        } else {
            println!();
            println!("Canonical: not found");
        }
    }

    println!();
    println!("Use 'goose model list' to see available models.");
    println!("Use 'goose configure' to add new models.");

    Ok(())
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
        // Add space for "* " prefix when showing default marker
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
