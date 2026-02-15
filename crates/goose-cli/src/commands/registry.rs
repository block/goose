use anyhow::Result;
use console::style;
use goose::registry::manifest::{RegistryEntry, RegistryEntryKind};
use goose::registry::sources::local::LocalRegistrySource;
use goose::registry::RegistryManager;

fn kind_from_str(s: &str) -> Option<RegistryEntryKind> {
    match s.to_lowercase().as_str() {
        "tool" | "tools" => Some(RegistryEntryKind::Tool),
        "skill" | "skills" => Some(RegistryEntryKind::Skill),
        "agent" | "agents" => Some(RegistryEntryKind::Agent),
        "recipe" | "recipes" => Some(RegistryEntryKind::Recipe),
        _ => None,
    }
}

fn default_manager() -> Result<RegistryManager> {
    let mut manager = RegistryManager::new();
    let local = LocalRegistrySource::from_default_paths()?;
    manager.add_source(Box::new(local));
    Ok(manager)
}

fn print_entry(entry: &RegistryEntry, verbose: bool) {
    let kind_icon = match entry.kind {
        RegistryEntryKind::Tool => "\u{1f527}",
        RegistryEntryKind::Skill => "\u{1f4dd}",
        RegistryEntryKind::Agent => "\u{1f916}",
        RegistryEntryKind::Recipe => "\u{1f4e6}",
    };

    println!(
        "  {} {} {}",
        kind_icon,
        style(&entry.name).bold(),
        style(format!("{:?}", entry.kind)).dim()
    );

    if !entry.description.is_empty() {
        println!("    {}", entry.description);
    }

    if verbose {
        if let Some(version) = &entry.version {
            println!("    Version: {}", version);
        }
        if let Some(author) = &entry.author {
            if let Some(name) = &author.name {
                println!("    Author: {}", name);
            }
        }
        if let Some(uri) = &entry.source_uri {
            println!("    Source: {}", uri);
        }
        if !entry.tags.is_empty() {
            println!("    Tags: {}", entry.tags.join(", "));
        }
    }
}

fn print_entries_json(entries: &[RegistryEntry]) -> Result<()> {
    let json = serde_json::to_string_pretty(entries)?;
    println!("{}", json);
    Ok(())
}

pub async fn handle_search(
    query: &str,
    kind: Option<&str>,
    format: &str,
    verbose: bool,
) -> Result<()> {
    let manager = default_manager()?;
    let kind_filter = kind.and_then(kind_from_str);
    let results = manager.search(Some(query), kind_filter).await?;

    if format == "json" {
        return print_entries_json(&results);
    }

    if results.is_empty() {
        println!("{}", style("No entries found.").yellow());
        return Ok(());
    }

    println!(
        "{}",
        style(format!("Found {} entries:", results.len())).green()
    );
    println!();
    for entry in &results {
        print_entry(entry, verbose);
    }

    Ok(())
}

pub async fn handle_list(kind: Option<&str>, format: &str, verbose: bool) -> Result<()> {
    let manager = default_manager()?;
    let kind_filter = kind.and_then(kind_from_str);
    let results = manager.list(kind_filter).await?;

    if format == "json" {
        return print_entries_json(&results);
    }

    if results.is_empty() {
        println!("{}", style("Registry is empty.").yellow());
        return Ok(());
    }

    println!(
        "{}",
        style(format!("{} entries in registry:", results.len())).green()
    );
    println!();
    for entry in &results {
        print_entry(entry, verbose);
    }

    Ok(())
}

pub async fn handle_info(name: &str, kind: Option<&str>) -> Result<()> {
    let manager = default_manager()?;
    let kind_filter = kind.and_then(kind_from_str);
    let entry = manager.get(name, kind_filter).await?;

    match entry {
        Some(e) => {
            println!("{}", style(format!("Registry Entry: {}", e.name)).bold());
            println!("  Kind: {:?}", e.kind);
            if !e.description.is_empty() {
                println!("  Description: {}", e.description);
            }
            if let Some(version) = &e.version {
                println!("  Version: {}", version);
            }
            if let Some(author) = &e.author {
                if let Some(name) = &author.name {
                    println!("  Author: {}", name);
                }
                if let Some(contact) = &author.contact {
                    println!("  Contact: {}", contact);
                }
            }
            if let Some(uri) = &e.source_uri {
                println!("  Source: {}", uri);
            }
            if let Some(path) = &e.local_path {
                println!("  Local path: {}", path.display());
            }
            if !e.tags.is_empty() {
                println!("  Tags: {}", e.tags.join(", "));
            }
            println!();
            println!("  Detail: {:?}", e.detail);
            Ok(())
        }
        None => {
            println!(
                "{}",
                style(format!("Entry '{}' not found in registry.", name)).red()
            );
            Ok(())
        }
    }
}

pub async fn handle_sources() -> Result<()> {
    let manager = default_manager()?;
    let sources = manager.source_names();

    println!("{}", style("Configured registry sources:").bold());
    println!();
    for (i, name) in sources.iter().enumerate() {
        println!("  {}. {}", i + 1, style(name).cyan());
    }

    Ok(())
}

pub async fn handle_add(name: &str, kind_str: Option<&str>) -> Result<()> {
    use goose::registry::install::{install_entry, is_installed};

    let kind = kind_str.and_then(kind_from_str);
    let manager = default_manager()?;

    // Search for the entry
    let entries = manager.search(Some(name), kind).await?;
    let entry = entries.into_iter().find(|e| e.name == name);

    match entry {
        Some(entry) => {
            if is_installed(&entry.name, entry.kind) {
                println!(
                    "{} {} is already installed",
                    style("✓").green(),
                    style(&entry.name).cyan()
                );
                return Ok(());
            }

            let path = install_entry(&entry)?;
            println!(
                "{} Installed {} ({}) to {}",
                style("✓").green(),
                style(&entry.name).cyan(),
                style(format!("{}", entry.kind)).dim(),
                style(path.display()).dim()
            );
            Ok(())
        }
        None => {
            println!("{} No entry found matching '{}'", style("✗").red(), name);
            if kind_str.is_some() {
                println!("  Try without --kind to search across all types");
            }
            Ok(())
        }
    }
}

pub async fn handle_remove(name: &str, kind_str: &str) -> Result<()> {
    use goose::registry::install::{is_installed, remove_entry};

    let kind = kind_from_str(kind_str).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown kind '{}'. Use: tool, skill, agent, or recipe",
            kind_str
        )
    })?;

    if !is_installed(name, kind) {
        println!(
            "{} {} ({}) is not installed",
            style("✗").yellow(),
            style(name).cyan(),
            kind_str,
        );
        return Ok(());
    }

    remove_entry(name, kind)?;
    println!(
        "{} Removed {} ({})",
        style("✓").green(),
        style(name).cyan(),
        kind_str,
    );
    Ok(())
}

pub async fn handle_validate(path: &str) -> Result<()> {
    use goose::registry::publish::validate_for_publish;
    use std::path::Path;

    let manifest_path = Path::new(path);
    if !manifest_path.exists() {
        anyhow::bail!("File not found: {}", path);
    }

    match validate_for_publish(manifest_path) {
        Ok(issues) => {
            if issues.is_empty() {
                println!("{} Manifest is valid for publishing!", style("✓").green());
            } else {
                println!("{} Manifest has issues:", style("⚠").yellow());
                for issue in &issues {
                    println!("  {} {}", style("•").yellow(), issue);
                }
            }
            Ok(())
        }
        Err(e) => {
            println!("{} Failed to validate manifest: {}", style("✗").red(), e);
            Ok(())
        }
    }
}

pub async fn handle_init(name: Option<String>, description: Option<String>) -> Result<()> {
    use goose::registry::publish::init_manifest;

    let agent_name = name.unwrap_or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| "my-agent".to_string())
    });

    let desc = description.unwrap_or_else(|| format!("A goose agent: {}", agent_name));

    let dir = std::env::current_dir()?;
    let path = init_manifest(&dir, &agent_name, &desc)?;

    println!(
        "{} Created manifest: {}",
        style("✓").green(),
        style(path.display()).cyan()
    );
    println!();
    println!("  Edit the manifest to configure your agent, then validate with:");
    println!("  {}", style("goose registry validate agent.yaml").dim());

    Ok(())
}

pub async fn handle_agent_info(name: &str, mode: Option<&str>) -> Result<()> {
    use goose::registry::manifest::RegistryEntryDetail;

    let manager = default_manager()?;
    let kind = Some(goose::registry::manifest::RegistryEntryKind::Agent);
    let entry = manager.get(name, kind).await?;

    match entry {
        Some(entry) => {
            println!(
                "{} {} {}",
                style("🤖").bold(),
                style(&entry.name).bold().cyan(),
                entry
                    .version
                    .as_deref()
                    .map(|v| format!("v{v}"))
                    .unwrap_or_default()
            );
            if !entry.description.is_empty() {
                println!("  {}", entry.description);
            }
            if let Some(author) = &entry.author {
                if let Some(name) = &author.name {
                    println!("  Author: {}", style(name).dim());
                }
            }
            if let Some(license) = &entry.license {
                println!("  License: {}", style(license).dim());
            }
            if let Some(uri) = &entry.source_uri {
                println!("  Source: {}", style(uri).dim());
            }

            if let RegistryEntryDetail::Agent(ref detail) = entry.detail {
                if !detail.capabilities.is_empty() {
                    println!();
                    println!("  {}", style("Capabilities:").underlined());
                    for cap in &detail.capabilities {
                        println!("    • {cap}");
                    }
                }
                if !detail.domains.is_empty() {
                    println!("  {}", style("Domains:").underlined());
                    for d in &detail.domains {
                        println!("    • {d}");
                    }
                }
                if !detail.recommended_models.is_empty() {
                    println!("  {}", style("Recommended models:").underlined());
                    for m in &detail.recommended_models {
                        println!("    • {m}");
                    }
                }
                if !detail.required_extensions.is_empty() {
                    println!("  {}", style("Required extensions:").underlined());
                    for ext in &detail.required_extensions {
                        println!("    • {ext}");
                    }
                }

                // Show modes
                if !detail.modes.is_empty() {
                    println!();
                    println!("  {}", style("Modes:").bold().underlined());
                    let default = detail.default_mode.as_deref();
                    for m in &detail.modes {
                        let is_default = default == Some(m.slug.as_str());
                        let marker = if is_default { " (default)" } else { "" };
                        println!(
                            "    {} {}{}",
                            style(&m.name).bold(),
                            style(&m.slug).dim(),
                            style(marker).yellow()
                        );
                        if !m.description.is_empty() {
                            println!("      {}", m.description);
                        }
                        if !m.tool_groups.is_empty() {
                            let groups: Vec<String> = m
                                .tool_groups
                                .iter()
                                .map(|tg| match tg {
                                    goose::registry::manifest::ToolGroupAccess::Full(g) => {
                                        g.clone()
                                    }
                                    goose::registry::manifest::ToolGroupAccess::Restricted {
                                        group,
                                        file_regex,
                                    } => {
                                        format!("{group} ({file_regex})")
                                    }
                                })
                                .collect();
                            println!("      Tools: {}", style(groups.join(", ")).dim());
                        }
                    }

                    // Show specific mode details if requested
                    if let Some(mode_slug) = mode {
                        if let Some(m) = detail.modes.iter().find(|m| m.slug == mode_slug) {
                            println!();
                            println!(
                                "  {}",
                                style(format!("Mode: {} ({})", m.name, m.slug)).bold()
                            );
                            if let Some(ref instructions) = m.instructions {
                                println!("  {}", style("Instructions:").underlined());
                                // Show first 500 chars
                                let preview = if instructions.chars().count() > 500 {
                                    let truncated: String =
                                        instructions.chars().take(500).collect();
                                    format!("{truncated}...")
                                } else {
                                    instructions.clone()
                                };
                                for line in preview.lines() {
                                    println!("    {line}");
                                }
                            }
                            if let Some(ref file) = m.instructions_file {
                                println!("  Instructions file: {}", style(file).dim());
                            }
                        } else {
                            println!();
                            println!("  {} Mode '{}' not found", style("⚠").yellow(), mode_slug);
                        }
                    }
                }
            }

            println!();
            Ok(())
        }
        None => {
            println!("{} Agent '{}' not found", style("✗").red(), name);
            Ok(())
        }
    }
}

pub async fn handle_agent_modes(name: &str) -> Result<()> {
    use goose::registry::manifest::RegistryEntryDetail;

    let manager = default_manager()?;
    let kind = Some(goose::registry::manifest::RegistryEntryKind::Agent);
    let entry = manager.get(name, kind).await?;

    match entry {
        Some(entry) => {
            if let RegistryEntryDetail::Agent(ref detail) = entry.detail {
                if detail.modes.is_empty() {
                    println!(
                        "{} Agent '{}' has no modes defined",
                        style("ℹ").blue(),
                        style(&entry.name).cyan()
                    );
                    return Ok(());
                }

                println!(
                    "{} Modes for {}:",
                    style("🤖").bold(),
                    style(&entry.name).bold().cyan()
                );
                let default = detail.default_mode.as_deref();
                for m in &detail.modes {
                    let is_default = default == Some(m.slug.as_str());
                    let marker = if is_default {
                        format!(" {}", style("(default)").yellow())
                    } else {
                        String::new()
                    };
                    println!(
                        "  {} {}{marker}",
                        style(&m.slug).bold(),
                        style(&m.name).dim()
                    );
                    if !m.description.is_empty() {
                        println!("    {}", m.description);
                    }
                    if !m.tool_groups.is_empty() {
                        let groups: Vec<String> = m
                            .tool_groups
                            .iter()
                            .map(|tg| match tg {
                                goose::registry::manifest::ToolGroupAccess::Full(g) => g.clone(),
                                goose::registry::manifest::ToolGroupAccess::Restricted {
                                    group,
                                    file_regex,
                                } => {
                                    format!("{group} ({file_regex})")
                                }
                            })
                            .collect();
                        println!("    Tools: {}", style(groups.join(", ")).dim());
                    }
                }
            } else {
                println!("{} '{}' is not an agent", style("✗").red(), name);
            }
            Ok(())
        }
        None => {
            println!("{} Agent '{}' not found", style("✗").red(), name);
            Ok(())
        }
    }
}

pub async fn handle_agent_run(name: &str, prompt: &str, mode: Option<&str>) -> Result<()> {
    use goose::agent_manager::client::AgentClientManager;
    use goose::registry::manifest::RegistryEntryDetail;

    let manager = default_manager()?;
    let entry = manager
        .get(
            name,
            Some(goose::registry::manifest::RegistryEntryKind::Agent),
        )
        .await?;

    let entry = match entry {
        Some(e) => e,
        None => {
            println!(
                "{} Agent '{}' not found in registry",
                style("✗").red(),
                name
            );
            return Ok(());
        }
    };

    let distribution = match &entry.detail {
        RegistryEntryDetail::Agent(detail) => match &detail.distribution {
            Some(dist) => dist.clone(),
            None => {
                println!(
                    "{} Agent '{}' has no distribution info",
                    style("✗").red(),
                    name
                );
                return Ok(());
            }
        },
        _ => {
            println!("{} '{}' is not an agent", style("✗").red(), name);
            return Ok(());
        }
    };

    let agent_manager = AgentClientManager::default();

    println!(
        "{} Connecting to agent '{}'...",
        style("⟳").cyan(),
        style(name).bold()
    );

    agent_manager
        .connect_with_distribution(name.to_string(), &distribution)
        .await?;

    println!("{} Connected", style("✓").green());

    // Create a session
    use goose::agent_manager::{NewSessionRequest, SessionModeId};
    let cwd = std::env::current_dir().unwrap_or_default();
    let session_resp = agent_manager
        .new_session(name, NewSessionRequest::new(cwd))
        .await?;
    let session_id = session_resp.session_id;

    // Set mode if requested
    if let Some(mode_id) = mode {
        use goose::agent_manager::SetSessionModeRequest;
        let mode_req = SetSessionModeRequest::new(
            session_id.clone(),
            SessionModeId::from(mode_id.to_string()),
        );
        agent_manager.set_mode(name, mode_req).await?;
        println!(
            "{} Mode '{}' set",
            style("✓").green(),
            style(mode_id).bold()
        );
    }

    let result = agent_manager
        .prompt_agent_text(name, &session_id, prompt)
        .await?;

    println!();
    println!("{}", style("Agent Response:").bold().green());
    println!("{}", result);

    agent_manager.shutdown_all().await;

    Ok(())
}

pub async fn handle_orchestrate(request: &str, use_llm: bool) -> Result<()> {
    use goose::agents::orchestrator_agent::{set_orchestrator_enabled, OrchestratorAgent};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // LLM orchestration is now enabled by default
    // Only disable if explicitly requested (--no-llm would set this)
    if !use_llm {
        set_orchestrator_enabled(false);
    }

    let provider = None;

    let orchestrator = OrchestratorAgent::new(Arc::new(Mutex::new(provider)));
    let plan = orchestrator.route(request).await;

    let primary = plan.primary_routing();
    println!(
        "{} {}",
        style("Orchestrator Routing Decision").bold().cyan(),
        if use_llm { "(LLM)" } else { "(keyword)" }
    );
    println!();
    println!(
        "  {} {} / {}",
        style("→").green(),
        style(&primary.agent_name).bold(),
        style(&primary.mode_slug).bold()
    );
    println!(
        "  {} {:.0}%",
        style("Confidence:").dim(),
        primary.confidence * 100.0
    );
    println!("  {} {}", style("Reasoning:").dim(), primary.reasoning);

    if plan.is_compound {
        println!();
        println!(
            "{} Compound request detected — {} sub-tasks:",
            style("⚡").yellow(),
            plan.tasks.len()
        );
        for (i, sub) in plan.tasks.iter().enumerate() {
            println!(
                "  {}. {} / {} — {}",
                i + 1,
                style(&sub.routing.agent_name).bold(),
                style(&sub.routing.mode_slug).bold(),
                sub.sub_task_description
            );
        }
    }

    Ok(())
}

pub async fn handle_orchestrator_status() -> Result<()> {
    use goose::agents::orchestrator_agent::OrchestratorAgent;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let orchestrator = OrchestratorAgent::new(Arc::new(Mutex::new(None)));
    let is_llm_enabled = goose::agents::orchestrator_agent::is_orchestrator_enabled();

    println!("{}", style("Orchestrator Status").bold().cyan());
    println!();
    println!(
        "  {} {}",
        style("Mode:").dim(),
        if is_llm_enabled {
            style("LLM-based routing (default)").green()
        } else {
            style("Keyword matching (fallback)").yellow()
        }
    );
    println!(
        "  {} GOOSE_ORCHESTRATOR_DISABLED={}",
        style("Env:").dim(),
        if is_llm_enabled {
            "false (orchestrator active)"
        } else {
            "true (fallback to keyword routing)"
        }
    );
    println!();

    println!("{}", style("Agent Catalog:").bold());
    let catalog_text = orchestrator.build_catalog_text();
    for line in catalog_text.lines() {
        println!("  {}", line);
    }

    let slots = orchestrator.slots();
    println!();
    println!(
        "{} {} agents registered, {} modes total",
        style("Summary:").bold(),
        slots.len(),
        slots.iter().map(|s| s.modes.len()).sum::<usize>()
    );

    Ok(())
}
