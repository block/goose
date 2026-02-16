use std::collections::{HashMap, HashSet};

use anyhow::{bail, Result};
use tracing::debug;

use crate::agents::extension::PLATFORM_EXTENSIONS;
use crate::agents::ExtensionConfig;
use crate::registry::manifest::{AgentDependency, AgentDetail, RegistryEntryKind};

/// Resolution status for a single dependency.
#[derive(Debug, Clone)]
pub struct ResolvedDependency {
    pub name: String,
    pub dep_type: RegistryEntryKind,
    pub source: DependencySource,
    pub required: bool,
}

/// Where a dependency was resolved from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencySource {
    Platform,
    Builtin,
    AlreadyLoaded,
    SessionConfig,
    Unresolved,
}

/// Result of resolving all dependencies for an agent.
#[derive(Debug, Clone)]
pub struct ResolutionResult {
    pub resolved: Vec<ResolvedDependency>,
    pub extensions_to_load: Vec<String>,
    pub missing_required: Vec<String>,
    pub missing_optional: Vec<String>,
}

impl ResolutionResult {
    pub fn is_satisfied(&self) -> bool {
        self.missing_required.is_empty()
    }
}

/// ServiceBroker resolves agent manifest dependencies to concrete MCP extensions.
///
/// Given an agent's `AgentDetail.dependencies`, the broker determines which
/// MCP extensions need to be loaded and returns a resolution plan. The actual
/// loading is left to the caller (ExtensionManager or session setup).
///
/// Resolution order:
/// 1. Already loaded in session → skip
/// 2. Platform extension → load from PLATFORM_EXTENSIONS
/// 3. Builtin extension → load from BUILTIN_REGISTRY
/// 4. Session config → load from user's extension config
/// 5. Unresolved → report as missing
pub struct ServiceBroker {
    loaded_extensions: HashSet<String>,
    session_extensions: HashMap<String, ExtensionConfig>,
}

impl Default for ServiceBroker {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceBroker {
    pub fn new() -> Self {
        Self {
            loaded_extensions: HashSet::new(),
            session_extensions: HashMap::new(),
        }
    }

    pub fn with_loaded(loaded: HashSet<String>) -> Self {
        Self {
            loaded_extensions: loaded,
            session_extensions: HashMap::new(),
        }
    }

    pub fn add_session_extension(&mut self, name: String, config: ExtensionConfig) {
        self.session_extensions.insert(name, config);
    }

    pub fn set_loaded_extensions(&mut self, loaded: HashSet<String>) {
        self.loaded_extensions = loaded;
    }

    /// Resolve all dependencies from an agent's detail.
    pub fn resolve_dependencies(&self, detail: &AgentDetail) -> ResolutionResult {
        let mut resolved = Vec::new();
        let mut extensions_to_load = Vec::new();
        let mut missing_required = Vec::new();
        let mut missing_optional = Vec::new();

        for dep in &detail.dependencies {
            let resolution = self.resolve_single(dep);

            match resolution.source {
                DependencySource::Unresolved => {
                    if dep.required {
                        missing_required.push(dep.name.clone());
                    } else {
                        missing_optional.push(dep.name.clone());
                    }
                }
                DependencySource::AlreadyLoaded => {
                    // nothing to do
                }
                _ => {
                    extensions_to_load.push(dep.name.clone());
                }
            }

            resolved.push(resolution);
        }

        ResolutionResult {
            resolved,
            extensions_to_load,
            missing_required,
            missing_optional,
        }
    }

    /// Resolve a list of dependency names (convenience for DeveloperAgent tool_groups).
    pub fn resolve_names(&self, names: &[String]) -> ResolutionResult {
        let deps: Vec<AgentDependency> = names
            .iter()
            .map(|name| AgentDependency {
                dep_type: RegistryEntryKind::Tool,
                name: name.clone(),
                version: None,
                required: false,
            })
            .collect();

        let detail = AgentDetail {
            instructions: String::new(),
            model: None,
            recommended_models: Vec::new(),
            capabilities: Vec::new(),
            domains: Vec::new(),
            input_content_types: Vec::new(),
            output_content_types: Vec::new(),
            required_extensions: Vec::new(),
            dependencies: deps,
            default_mode: None,
            modes: Vec::new(),
            skills: Vec::new(),
            distribution: None,
            security: Vec::new(),
            status: None,
            framework: None,
            programming_language: None,
            natural_languages: Vec::new(),
        };

        self.resolve_dependencies(&detail)
    }

    fn resolve_single(&self, dep: &AgentDependency) -> ResolvedDependency {
        let name = &dep.name;

        // 1. Already loaded
        if self.loaded_extensions.contains(name) {
            debug!(name, "ServiceBroker: dependency already loaded");
            return ResolvedDependency {
                name: name.clone(),
                dep_type: dep.dep_type,
                source: DependencySource::AlreadyLoaded,
                required: dep.required,
            };
        }

        // 2. Platform extension
        if PLATFORM_EXTENSIONS.contains_key(name.as_str()) {
            debug!(name, "ServiceBroker: resolved as platform extension");
            return ResolvedDependency {
                name: name.clone(),
                dep_type: dep.dep_type,
                source: DependencySource::Platform,
                required: dep.required,
            };
        }

        // 3. Builtin extension
        if crate::builtin_extension::get_builtin_extension(name).is_some() {
            debug!(name, "ServiceBroker: resolved as builtin extension");
            return ResolvedDependency {
                name: name.clone(),
                dep_type: dep.dep_type,
                source: DependencySource::Builtin,
                required: dep.required,
            };
        }

        // 4. Session config
        if self.session_extensions.contains_key(name) {
            debug!(name, "ServiceBroker: resolved from session config");
            return ResolvedDependency {
                name: name.clone(),
                dep_type: dep.dep_type,
                source: DependencySource::SessionConfig,
                required: dep.required,
            };
        }

        // 5. Unresolved
        debug!(name, dep.required, "ServiceBroker: dependency unresolved");
        ResolvedDependency {
            name: name.clone(),
            dep_type: dep.dep_type,
            source: DependencySource::Unresolved,
            required: dep.required,
        }
    }
}

/// Validate that an agent's required dependencies can be satisfied.
pub fn validate_agent_dependencies(
    detail: &AgentDetail,
    loaded: &HashSet<String>,
) -> Result<ResolutionResult> {
    let broker = ServiceBroker::with_loaded(loaded.clone());
    let result = broker.resolve_dependencies(detail);

    if !result.is_satisfied() {
        bail!(
            "Agent has unsatisfied required dependencies: {}",
            result.missing_required.join(", ")
        );
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_dep(name: &str, required: bool) -> AgentDependency {
        AgentDependency {
            dep_type: RegistryEntryKind::Tool,
            name: name.to_string(),
            version: None,
            required,
        }
    }

    fn make_detail(deps: Vec<AgentDependency>) -> AgentDetail {
        AgentDetail {
            instructions: String::new(),
            model: None,
            recommended_models: Vec::new(),
            capabilities: Vec::new(),
            domains: Vec::new(),
            input_content_types: Vec::new(),
            output_content_types: Vec::new(),
            required_extensions: Vec::new(),
            dependencies: deps,
            default_mode: None,
            modes: Vec::new(),
            skills: Vec::new(),
            distribution: None,
            security: Vec::new(),
            status: None,
            framework: None,
            programming_language: None,
            natural_languages: Vec::new(),
        }
    }

    #[test]
    fn test_resolve_already_loaded() {
        let mut loaded = HashSet::new();
        loaded.insert("developer".to_string());
        let broker = ServiceBroker::with_loaded(loaded);

        let detail = make_detail(vec![make_dep("developer", true)]);
        let result = broker.resolve_dependencies(&detail);

        assert!(result.is_satisfied());
        assert!(result.extensions_to_load.is_empty());
        assert_eq!(result.resolved[0].source, DependencySource::AlreadyLoaded);
    }

    #[test]
    fn test_resolve_platform_extension() {
        let broker = ServiceBroker::new();

        // "todo" is a known platform extension
        let detail = make_detail(vec![make_dep("todo", true)]);
        let result = broker.resolve_dependencies(&detail);

        assert!(result.is_satisfied());
        assert_eq!(result.extensions_to_load, vec!["todo"]);
        assert_eq!(result.resolved[0].source, DependencySource::Platform);
    }

    #[test]
    fn test_resolve_missing_required() {
        let broker = ServiceBroker::new();

        let detail = make_detail(vec![make_dep("nonexistent_tool_xyz", true)]);
        let result = broker.resolve_dependencies(&detail);

        assert!(!result.is_satisfied());
        assert_eq!(result.missing_required, vec!["nonexistent_tool_xyz"]);
    }

    #[test]
    fn test_resolve_missing_optional() {
        let broker = ServiceBroker::new();

        let detail = make_detail(vec![make_dep("nonexistent_tool_xyz", false)]);
        let result = broker.resolve_dependencies(&detail);

        assert!(result.is_satisfied());
        assert_eq!(result.missing_optional, vec!["nonexistent_tool_xyz"]);
        assert!(result.extensions_to_load.is_empty());
    }

    #[test]
    fn test_resolve_mixed_dependencies() {
        let mut loaded = HashSet::new();
        loaded.insert("developer".to_string());
        let broker = ServiceBroker::with_loaded(loaded);

        let detail = make_detail(vec![
            make_dep("developer", true),    // already loaded
            make_dep("todo", true),         // platform
            make_dep("missing_req", true),  // missing required
            make_dep("missing_opt", false), // missing optional
        ]);

        let result = broker.resolve_dependencies(&detail);

        assert!(!result.is_satisfied());
        assert_eq!(result.extensions_to_load, vec!["todo"]);
        assert_eq!(result.missing_required, vec!["missing_req"]);
        assert_eq!(result.missing_optional, vec!["missing_opt"]);
    }

    #[test]
    fn test_resolve_session_config() {
        let mut broker = ServiceBroker::new();
        broker.add_session_extension(
            "custom_ext".to_string(),
            ExtensionConfig::Stdio {
                name: "custom_ext".to_string(),
                description: String::new(),
                cmd: "test".to_string(),
                args: vec![],
                envs: Default::default(),
                env_keys: vec![],
                timeout: None,
                bundled: None,
                available_tools: vec![],
            },
        );

        let detail = make_detail(vec![make_dep("custom_ext", true)]);
        let result = broker.resolve_dependencies(&detail);

        assert!(result.is_satisfied());
        assert_eq!(result.extensions_to_load, vec!["custom_ext"]);
        assert_eq!(result.resolved[0].source, DependencySource::SessionConfig);
    }

    #[test]
    fn test_validate_satisfied() {
        let mut loaded = HashSet::new();
        loaded.insert("developer".to_string());

        let detail = make_detail(vec![make_dep("developer", true)]);
        let result = validate_agent_dependencies(&detail, &loaded);

        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_unsatisfied() {
        let loaded = HashSet::new();

        let detail = make_detail(vec![make_dep("nonexistent_xyz", true)]);
        let result = validate_agent_dependencies(&detail, &loaded);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("nonexistent_xyz"));
    }

    #[test]
    fn test_resolve_names_convenience() {
        let mut loaded = HashSet::new();
        loaded.insert("developer".to_string());
        let broker = ServiceBroker::with_loaded(loaded);

        let names = vec!["developer".to_string(), "todo".to_string()];
        let result = broker.resolve_names(&names);

        assert!(result.is_satisfied());
        assert_eq!(result.resolved.len(), 2);
    }
}
