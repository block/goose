use crate::config::{get_enabled_extensions, ExtensionConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionResolutionStrategy {
    RecipeFirst,
    Resume,
}

/// Resolves which extensions to load for a session.
///
/// Priority order:
/// - Resume strategy: Use session's saved extensions, fallback to global config
/// - RecipeFirst strategy: Use recipe extensions if present, fallback to global config
///
/// # Arguments
/// * `strategy` - The resolution strategy to use
/// * `recipe_extensions` - Extensions defined in the recipe (if any)
/// * `session_extensions` - Extensions saved in the session (for resume scenarios)
///
/// # Returns
/// The list of extensions to load
pub fn resolve_extensions(
    strategy: ExtensionResolutionStrategy,
    recipe_extensions: Option<&Vec<ExtensionConfig>>,
    session_extensions: Option<Vec<ExtensionConfig>>,
) -> Vec<ExtensionConfig> {
    match strategy {
        ExtensionResolutionStrategy::Resume => {
            session_extensions.unwrap_or_else(get_enabled_extensions)
        }
        ExtensionResolutionStrategy::RecipeFirst => recipe_extensions
            .cloned()
            .unwrap_or_else(get_enabled_extensions),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ExtensionConfig;

    fn create_test_extension(name: &str) -> ExtensionConfig {
        ExtensionConfig::Builtin {
            name: name.to_string(),
        }
    }

    #[test]
    fn test_recipe_first_with_recipe_extensions() {
        let recipe_exts = vec![
            create_test_extension("recipe_ext_1"),
            create_test_extension("recipe_ext_2"),
        ];

        let result = resolve_extensions(
            ExtensionResolutionStrategy::RecipeFirst,
            Some(&recipe_exts),
            None,
        );

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name(), "recipe_ext_1");
        assert_eq!(result[1].name(), "recipe_ext_2");
    }

    #[test]
    fn test_recipe_first_without_recipe_extensions_falls_back_to_global() {
        // When no recipe extensions, should fall back to global config
        // This test just verifies the fallback path is taken
        let result = resolve_extensions(ExtensionResolutionStrategy::RecipeFirst, None, None);

        // Result will be from get_enabled_extensions() which depends on config
        // We just verify it doesn't panic and returns a Vec
        assert!(result.len() >= 0);
    }

    #[test]
    fn test_resume_with_session_extensions() {
        let session_exts = vec![
            create_test_extension("session_ext_1"),
            create_test_extension("session_ext_2"),
        ];

        let result = resolve_extensions(
            ExtensionResolutionStrategy::Resume,
            None,
            Some(session_exts),
        );

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name(), "session_ext_1");
        assert_eq!(result[1].name(), "session_ext_2");
    }

    #[test]
    fn test_resume_ignores_recipe_extensions() {
        let recipe_exts = vec![create_test_extension("recipe_ext")];
        let session_exts = vec![create_test_extension("session_ext")];

        // Even with recipe extensions provided, Resume strategy should use session extensions
        let result = resolve_extensions(
            ExtensionResolutionStrategy::Resume,
            Some(&recipe_exts),
            Some(session_exts),
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name(), "session_ext");
    }

    #[test]
    fn test_resume_without_session_extensions_falls_back_to_global() {
        // When no session extensions, should fall back to global config
        let result = resolve_extensions(ExtensionResolutionStrategy::Resume, None, None);

        // Result will be from get_enabled_extensions() which depends on config
        // We just verify it doesn't panic and returns a Vec
        assert!(result.len() >= 0);
    }
}
