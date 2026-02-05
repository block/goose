//! Dynamic preamble generation based on enabled extensions
//!
//! The preamble (system prompt) is regenerated each turn to reflect
//! the current state of available and enabled extensions.

use super::EnabledExtensions;

/// Generate the system preamble based on current extension state.
///
/// This creates a dynamic system prompt that includes:
/// - List of all available extensions (from catalog)
/// - Which extensions are currently enabled
/// - Instructions from enabled extensions
pub fn generate_preamble(
    extensions: &EnabledExtensions,
    base_preamble: Option<&str>,
) -> String {
    let mut preamble = String::new();

    // Start with base preamble if provided
    if let Some(base) = base_preamble {
        preamble.push_str(base);
        preamble.push_str("\n\n");
    }

    // Add extension management section
    preamble.push_str("# Extensions\n\n");
    preamble.push_str("You can enable and disable extensions using the `platform__enable_extension` and `platform__disable_extension` tools.\n\n");

    // List available extensions
    preamble.push_str("## Available Extensions\n\n");
    
    let catalog = extensions.catalog().read();
    let mut configs: Vec<_> = catalog.list().collect();
    configs.sort_by(|a, b| a.name.cmp(&b.name));

    for config in configs {
        let status = if extensions.is_enabled(&config.name) {
            "✓ enabled"
        } else {
            "○ available"
        };
        preamble.push_str(&format!(
            "- **{}** [{}]: {}\n",
            config.name, status, config.description
        ));
    }
    drop(catalog); // Release the read lock

    // Add instructions from enabled extensions
    let enabled_with_instructions: Vec<_> = extensions
        .iter()
        .filter_map(|(name, ext)| ext.instructions().map(|i| (name, i)))
        .collect();

    if !enabled_with_instructions.is_empty() {
        preamble.push_str("\n## Enabled Extension Instructions\n\n");
        for (name, instructions) in enabled_with_instructions {
            preamble.push_str(&format!("### {}\n\n{}\n\n", name, instructions));
        }
    }

    preamble
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::ExtensionCatalog;
    use std::sync::Arc;
    use parking_lot::RwLock;

    #[test]
    fn test_generate_preamble_empty() {
        let catalog = Arc::new(RwLock::new(ExtensionCatalog::new()));
        let extensions = EnabledExtensions::new(catalog);

        let preamble = generate_preamble(&extensions, None);
        
        assert!(preamble.contains("# Extensions"));
        assert!(preamble.contains("## Available Extensions"));
    }

    #[test]
    fn test_generate_preamble_with_base() {
        let catalog = Arc::new(RwLock::new(ExtensionCatalog::new()));
        let extensions = EnabledExtensions::new(catalog);

        let preamble = generate_preamble(&extensions, Some("You are a helpful assistant."));
        
        assert!(preamble.starts_with("You are a helpful assistant."));
        assert!(preamble.contains("# Extensions"));
    }

    #[test]
    fn test_generate_preamble_with_extensions() {
        let toml = r#"
[extensions.browser]
kind = "mcp"
command = "npx"
args = ["-y", "@anthropic/mcp-browser"]
description = "Web browsing and automation"

[extensions.memory]
kind = "mcp"
command = "uvx"
args = ["mcp-memory"]
description = "Persistent memory across sessions"
"#;

        let catalog = ExtensionCatalog::from_toml(toml).unwrap();
        let catalog = Arc::new(RwLock::new(catalog));
        let extensions = EnabledExtensions::new(catalog);

        let preamble = generate_preamble(&extensions, None);
        
        assert!(preamble.contains("**browser**"));
        assert!(preamble.contains("**memory**"));
        assert!(preamble.contains("○ available"));
    }
}
