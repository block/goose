use crate::config::{get_enabled_extensions, ExtensionConfig};

/// Resolves which extensions to load for a new session.
///
/// Priority order:
/// 1. Recipe extensions (if defined in recipe)
/// 2. Override extensions (if provided)
/// 3. Global config (fallback)
///
/// # Arguments
/// * `recipe_extensions` - Extensions defined in the recipe (if any)
/// * `override_extensions` - Extensions provided as overrides (e.g., from hub)
pub fn resolve_extensions_for_new_session(
    recipe_extensions: Option<&[ExtensionConfig]>,
    override_extensions: Option<Vec<ExtensionConfig>>,
) -> Vec<ExtensionConfig> {
    if let Some(exts) = recipe_extensions {
        return exts.to_vec();
    }

    if let Some(exts) = override_extensions {
        return exts;
    }

    get_enabled_extensions()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ExtensionConfig;

    fn create_test_extension(name: &str) -> ExtensionConfig {
        ExtensionConfig::Builtin {
            name: name.to_string(),
            display_name: None,
            description: String::new(),
            timeout: None,
            bundled: None,
            available_tools: Vec::new(),
        }
    }

    #[test]
    fn test_recipe_extensions_take_priority() {
        let recipe_exts = vec![
            create_test_extension("recipe_ext_1"),
            create_test_extension("recipe_ext_2"),
        ];
        let override_exts = vec![create_test_extension("override_ext")];

        let result =
            resolve_extensions_for_new_session(Some(&recipe_exts), Some(override_exts));

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name(), "recipe_ext_1");
        assert_eq!(result[1].name(), "recipe_ext_2");
    }

    #[test]
    fn test_override_extensions_used_when_no_recipe() {
        let override_exts = vec![
            create_test_extension("override_ext_1"),
            create_test_extension("override_ext_2"),
        ];

        let result = resolve_extensions_for_new_session(None, Some(override_exts));

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name(), "override_ext_1");
        assert_eq!(result[1].name(), "override_ext_2");
    }

    #[test]
    fn test_falls_back_to_global_when_no_recipe_or_override() {
        let result = resolve_extensions_for_new_session(None, None);

        // Result will be from get_enabled_extensions() which depends on config
        // We just verify it doesn't panic and returns a Vec
        let _ = result;
    }
}
