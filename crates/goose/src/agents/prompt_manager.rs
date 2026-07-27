#[cfg(test)]
use chrono::DateTime;
use chrono::Utc;
use indexmap::IndexMap;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

use crate::agents::{extension::ExtensionInfo, moim};
use crate::hints::{combine_hint_sources, HintSource, SubdirectoryHintTracker};
use crate::{
    config::{Config, GooseMode},
    prompt_template,
    utils::sanitize_unicode_tags,
};
use std::path::Path;

const MAX_EXTENSIONS: usize = 5;
const MAX_TOOLS: usize = 50;
const CONTEXT_REPORT_EXTENSION_MARKER_PREFIX: &str = "__GOOSE_CONTEXT_REPORT_EXTENSION_";
const CONTEXT_REPORT_MOIM_MARKER: &str = "__GOOSE_CONTEXT_REPORT_MOIM__";

pub struct PromptManager {
    system_prompt_override: Option<String>,
    system_prompt_extras: IndexMap<String, String>,
    current_date_timestamp: String,
    subdirectory_hint_tracker: SubdirectoryHintTracker,
}

impl Default for PromptManager {
    fn default() -> Self {
        PromptManager::new()
    }
}

#[derive(Serialize)]
struct SystemPromptContext {
    extensions: Vec<ExtensionInfo>,
    current_date_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    extension_tool_limits: Option<(usize, usize)>,
    goose_mode: GooseMode,
    is_autonomous: bool,
    enable_subagents: bool,
    max_extensions: usize,
    max_tools: usize,
    code_execution_mode: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    moim_system_prompt_block: Option<String>,
}

const CHAT_MODE_INSTRUCTION: &str =
    "Right now you are in the chat only mode, no access to any tool use and system.";

pub struct SystemPromptSegments {
    pub base_template_blanked: String,
    pub base_is_override: bool,
    pub moim_block: Option<String>,
    pub extension_instructions: Vec<(String, String)>,
    pub extras: IndexMap<String, String>,
}

pub struct SystemPromptBuilder<'a, M> {
    manager: &'a M,

    extensions_info: Vec<ExtensionInfo>,
    frontend_instructions: Option<String>,
    extension_tool_count: Option<(usize, usize)>,
    subagents_enabled: bool,
    hints: Option<String>,
    code_execution_mode: bool,
    goose_mode: Option<GooseMode>,
}

impl<'a> SystemPromptBuilder<'a, PromptManager> {
    pub fn with_extension(mut self, extension: ExtensionInfo) -> Self {
        self.extensions_info.push(extension);
        self
    }

    pub fn with_extensions(mut self, extensions: impl Iterator<Item = ExtensionInfo>) -> Self {
        for extension in extensions {
            self.extensions_info.push(extension);
        }
        self
    }

    pub fn with_frontend_instructions(mut self, frontend_instructions: Option<String>) -> Self {
        self.frontend_instructions = frontend_instructions;
        self
    }

    pub fn with_extension_and_tool_counts(
        mut self,
        extension_count: usize,
        tool_count: usize,
    ) -> Self {
        self.extension_tool_count = Some((extension_count, tool_count));
        self
    }

    pub fn with_code_execution_mode(mut self, enabled: bool) -> Self {
        self.code_execution_mode = enabled;
        self
    }

    pub fn with_hint_sources(mut self, sources: &[HintSource]) -> Self {
        let hints = combine_hint_sources(sources);
        if !hints.is_empty() {
            self.hints = Some(hints);
        }
        self
    }

    pub fn with_enable_subagents(mut self, subagents_enabled: bool) -> Self {
        self.subagents_enabled = subagents_enabled;
        self
    }

    pub fn with_goose_mode(mut self, mode: GooseMode) -> Self {
        self.goose_mode = Some(mode);
        self
    }

    fn prepared_extensions(&self) -> Vec<ExtensionInfo> {
        let mut extensions_info = self.extensions_info.clone();

        if let Some(frontend_instructions) = &self.frontend_instructions {
            extensions_info.push(ExtensionInfo::new("frontend", frontend_instructions, false));
        }
        extensions_info.sort_by(|a, b| a.name.cmp(&b.name));

        extensions_info
            .into_iter()
            .map(|mut ext_info| {
                ext_info.instructions = sanitize_unicode_tags(&ext_info.instructions);
                ext_info
            })
            .collect()
    }

    fn resolved_goose_mode(&self) -> GooseMode {
        self.goose_mode
            .unwrap_or_else(|| Config::global().get_goose_mode().unwrap_or_default())
    }

    fn extension_tool_limits(&self) -> Option<(usize, usize)> {
        self.extension_tool_count
            .filter(|(extensions, tools)| *extensions > MAX_EXTENSIONS || *tools > MAX_TOOLS)
    }

    fn render_base(
        &self,
        extensions: Vec<ExtensionInfo>,
        goose_mode: GooseMode,
        moim_system_prompt_block: Option<String>,
    ) -> String {
        let context = SystemPromptContext {
            extensions,
            current_date_time: self.manager.current_date_timestamp.clone(),
            extension_tool_limits: self.extension_tool_limits(),
            goose_mode,
            is_autonomous: goose_mode == GooseMode::Auto,
            enable_subagents: self.subagents_enabled,
            max_extensions: MAX_EXTENSIONS,
            max_tools: MAX_TOOLS,
            code_execution_mode: self.code_execution_mode,
            moim_system_prompt_block,
        };

        if let Some(override_prompt) = &self.manager.system_prompt_override {
            let sanitized_override_prompt = sanitize_unicode_tags(override_prompt);
            prompt_template::render_string(&sanitized_override_prompt, &context)
        } else {
            prompt_template::render_template("system.md", &context)
        }
        .unwrap_or_else(|_| {
            "You are a general-purpose AI agent called goose, created by Block".to_string()
        })
    }

    fn finalize_extras(&self, goose_mode: GooseMode) -> IndexMap<String, String> {
        let mut extras = self.manager.system_prompt_extras.clone();

        if let Some(hints) = &self.hints {
            extras.insert("hints".to_string(), hints.clone());
        }

        if goose_mode == GooseMode::Chat {
            extras.insert("chat_mode".to_string(), CHAT_MODE_INSTRUCTION.to_string());
        }

        extras
            .into_iter()
            .map(|(key, value)| (key, sanitize_unicode_tags(&value)))
            .collect()
    }

    pub fn build(self) -> String {
        let goose_mode = self.resolved_goose_mode();
        let base_prompt = self.render_base(
            self.prepared_extensions(),
            goose_mode,
            moim::system_prompt_block(),
        );

        let extras = self.finalize_extras(goose_mode);

        if extras.is_empty() {
            base_prompt
        } else {
            format!(
                "{}\n\n# Additional Instructions:\n\n{}",
                base_prompt,
                extras.into_values().collect::<Vec<_>>().join("\n\n")
            )
        }
    }

    /// Decomposes the prompt `build` would produce into the pieces the context
    /// report attributes separately: the template itself, each extension's
    /// instructions, the MOIM block and the extras.
    ///
    /// The base is rendered with each variable-supplied body replaced by a marker
    /// which is then stripped, rather than rendered empty: an empty value makes
    /// the template's `{% if %}` guards drop literal scaffolding (the
    /// `### Instructions` heading) that the real prompt does contain.
    pub fn build_segments(self) -> SystemPromptSegments {
        let goose_mode = self.resolved_goose_mode();
        let prepared = self.prepared_extensions();

        let markers: Vec<Option<String>> = prepared
            .iter()
            .enumerate()
            .map(|(index, extension)| {
                (!extension.instructions.is_empty())
                    .then(|| format!("{CONTEXT_REPORT_EXTENSION_MARKER_PREFIX}{index}__"))
            })
            .collect();
        let marked_extensions: Vec<ExtensionInfo> = prepared
            .iter()
            .zip(&markers)
            .map(|(extension, marker)| {
                let mut extension = extension.clone();
                extension.instructions = marker.clone().unwrap_or_default();
                extension
            })
            .collect();

        let moim_block = moim::system_prompt_block();
        let mut base = self.render_base(
            marked_extensions,
            goose_mode,
            moim_block
                .as_ref()
                .map(|_| CONTEXT_REPORT_MOIM_MARKER.to_string()),
        );

        let extension_instructions = prepared
            .iter()
            .zip(&markers)
            .filter(|(_, marker)| marker.as_deref().is_some_and(|m| base.contains(m)))
            .map(|(extension, _)| (extension.name.clone(), extension.instructions.clone()))
            .collect();
        let moim_block = moim_block.filter(|_| base.contains(CONTEXT_REPORT_MOIM_MARKER));

        for marker in markers.iter().flatten() {
            base = base.replace(marker.as_str(), "");
        }
        base = base.replace(CONTEXT_REPORT_MOIM_MARKER, "");

        let extras = self.finalize_extras(goose_mode);

        SystemPromptSegments {
            base_template_blanked: base,
            base_is_override: self.manager.system_prompt_override.is_some(),
            moim_block,
            extension_instructions,
            extras,
        }
    }
}

impl PromptManager {
    pub fn new() -> Self {
        PromptManager {
            system_prompt_override: None,
            system_prompt_extras: IndexMap::new(),
            // Use the fixed current date time so that prompt cache can be used.
            // Filtering to an hour to balance user time accuracy and multi session prompt cache hits.
            current_date_timestamp: Utc::now().format("%Y-%m-%d %H:00 %:z").to_string(),
            subdirectory_hint_tracker: SubdirectoryHintTracker::new(),
        }
    }

    #[cfg(test)]
    pub fn with_timestamp(dt: DateTime<Utc>) -> Self {
        PromptManager {
            system_prompt_override: None,
            system_prompt_extras: IndexMap::new(),
            current_date_timestamp: dt.format("%Y-%m-%d %H:%M:%S %:z").to_string(),
            subdirectory_hint_tracker: SubdirectoryHintTracker::new(),
        }
    }

    /// Add an additional instruction to the system prompt with a key
    /// Using the same key will replace the previous instruction
    pub fn add_system_prompt_extra(&mut self, key: String, instruction: String) {
        self.system_prompt_extras.insert(key, instruction);
    }

    pub fn remove_system_prompt_extra(&mut self, key: &str) {
        self.system_prompt_extras.shift_remove(key);
    }

    pub fn record_tool_arguments(
        &mut self,
        arguments: &Option<serde_json::Map<String, serde_json::Value>>,
        working_dir: &Path,
    ) {
        self.subdirectory_hint_tracker
            .record_tool_arguments(arguments, working_dir);
    }

    pub fn load_subdirectory_hints(&mut self, working_dir: &Path) -> bool {
        let new_hints = self.subdirectory_hint_tracker.load_new_hints(working_dir);
        let has_new = !new_hints.is_empty();
        for (key, content) in new_hints {
            self.system_prompt_extras.insert(key, content);
        }
        has_new
    }

    /// Override the system prompt with custom text
    pub fn set_system_prompt_override(&mut self, template: String) {
        self.system_prompt_override = Some(template);
    }

    pub fn clear_system_prompt_override(&mut self) {
        self.system_prompt_override = None;
    }

    pub fn builder<'a>(&'a self) -> SystemPromptBuilder<'a, Self> {
        SystemPromptBuilder {
            manager: self,

            extensions_info: vec![],
            frontend_instructions: None,
            extension_tool_count: None,
            subagents_enabled: false,
            hints: None,
            code_execution_mode: false,
            goose_mode: None,
        }
    }

    pub async fn get_recipe_prompt(&self) -> String {
        let context: HashMap<&str, Value> = HashMap::new();
        prompt_template::render_template("recipe.md", &context)
            .unwrap_or_else(|_| "The recipe prompt is busted. Tell the user.".to_string())
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use super::*;

    #[test]
    fn test_build_system_prompt_sanitizes_override() {
        let mut manager = PromptManager::new();
        let malicious_override = "System prompt\u{E0041}\u{E0042}\u{E0043}with hidden text";
        manager.set_system_prompt_override(malicious_override.to_string());

        let result = manager.builder().build();

        assert!(!result.contains('\u{E0041}'));
        assert!(!result.contains('\u{E0042}'));
        assert!(!result.contains('\u{E0043}'));
        assert!(result.contains("System prompt"));
        assert!(result.contains("with hidden text"));
    }

    #[test]
    fn test_current_date_time_includes_timezone() {
        let mut manager =
            PromptManager::with_timestamp(DateTime::<Utc>::from_timestamp(0, 0).unwrap());
        manager.set_system_prompt_override("It is currently {{current_date_time}}".to_string());

        let result = manager.builder().build();

        assert_eq!(result, "It is currently 1970-01-01 00:00:00 +00:00");
    }

    #[test]
    fn test_build_system_prompt_sanitizes_extras() {
        let mut manager = PromptManager::new();
        let malicious_extra = "Extra instruction\u{E0041}\u{E0042}\u{E0043}hidden";
        manager.add_system_prompt_extra("test".to_string(), malicious_extra.to_string());

        let result = manager.builder().build();

        assert!(!result.contains('\u{E0041}'));
        assert!(!result.contains('\u{E0042}'));
        assert!(!result.contains('\u{E0043}'));
        assert!(result.contains("Extra instruction"));
        assert!(result.contains("hidden"));
    }

    #[test]
    fn test_build_system_prompt_sanitizes_multiple_extras() {
        let mut manager = PromptManager::new();
        manager
            .add_system_prompt_extra("test1".to_string(), "First\u{E0041}instruction".to_string());
        manager.add_system_prompt_extra(
            "test2".to_string(),
            "Second\u{E0042}instruction".to_string(),
        );
        manager
            .add_system_prompt_extra("test3".to_string(), "Third\u{E0043}instruction".to_string());

        let result = manager.builder().build();

        assert!(!result.contains('\u{E0041}'));
        assert!(!result.contains('\u{E0042}'));
        assert!(!result.contains('\u{E0043}'));
        assert!(result.contains("Firstinstruction"));
        assert!(result.contains("Secondinstruction"));
        assert!(result.contains("Thirdinstruction"));
    }

    #[test]
    fn test_remove_system_prompt_extra() {
        let mut manager = PromptManager::new();
        manager.add_system_prompt_extra("agent".to_string(), "Agent instruction".to_string());
        manager.add_system_prompt_extra("project".to_string(), "Project instruction".to_string());

        manager.remove_system_prompt_extra("agent");
        let result = manager.builder().build();

        assert!(!result.contains("Agent instruction"));
        assert!(result.contains("Project instruction"));
    }

    #[test]
    fn test_clear_system_prompt_override() {
        let mut manager = PromptManager::new();
        manager.set_system_prompt_override("Replacement prompt".to_string());
        assert!(manager.builder().build().contains("Replacement prompt"));

        manager.clear_system_prompt_override();
        assert!(!manager.builder().build().contains("Replacement prompt"));
    }

    #[test]
    fn test_build_segments_respects_override_without_extension_or_moim_variables() {
        let mut manager = PromptManager::new();
        manager.set_system_prompt_override("Replacement prompt".to_string());

        let segments = manager
            .builder()
            .with_extension(ExtensionInfo::new("test", "extension instructions", true))
            .build_segments();

        assert!(segments.base_is_override);
        assert!(segments.extension_instructions.is_empty());
        assert!(segments.moim_block.is_none());
        assert_eq!(segments.base_template_blanked, "Replacement prompt");
    }

    #[test]
    fn test_build_segments_ignores_literal_variable_names_in_override() {
        let mut manager = PromptManager::new();
        manager.set_system_prompt_override(
            "Do not mention extensions or moim_system_prompt_block in responses.".to_string(),
        );

        let segments = manager
            .builder()
            .with_extension(ExtensionInfo::new("test", "extension instructions", true))
            .build_segments();

        assert!(segments.base_is_override);
        assert!(segments.extension_instructions.is_empty());
        assert!(segments.moim_block.is_none());
    }

    #[test]
    fn test_build_segments_tracks_variables_rendered_by_override() {
        let mut manager = PromptManager::new();
        manager.set_system_prompt_override(
            "{% for extension in extensions %}{{ extension.instructions }}{% endfor %}\n{{ moim_system_prompt_block }}"
                .to_string(),
        );

        let segments = manager
            .builder()
            .with_extension(ExtensionInfo::new("test", "extension instructions", true))
            .build_segments();

        assert!(segments.base_is_override);
        assert_eq!(
            segments.extension_instructions,
            vec![("test".to_string(), "extension instructions".to_string())]
        );
        assert!(segments.moim_block.is_some());
    }

    #[test]
    fn test_build_segments_without_override_reports_template_base() {
        let manager = PromptManager::new();

        let segments = manager
            .builder()
            .with_goose_mode(GooseMode::Auto)
            .build_segments();

        assert!(!segments.base_is_override);
        assert!(!segments.base_template_blanked.is_empty());
    }

    #[test]
    fn test_hint_sources_combine_to_loaded_hints() {
        use crate::hints::{build_gitignore, load_hint_files, load_hint_files_with_sources};

        let home_dir = tempfile::tempdir().unwrap();
        let home = home_dir.path().display().to_string();
        let _guard = env_lock::lock_env([
            ("HOME", Some(home.as_str())),
            ("GOOSE_PATH_ROOT", Some(home.as_str())),
        ]);

        let config_dir = crate::config::paths::Paths::config_dir();
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("AGENTS.md"), "global hint content").unwrap();

        let working_dir = tempfile::tempdir().unwrap();
        std::fs::write(working_dir.path().join("AGENTS.md"), "project hint content").unwrap();

        let filenames = vec!["AGENTS.md".to_string()];
        let ignore = build_gitignore(working_dir.path());
        let sources = load_hint_files_with_sources(working_dir.path(), &filenames, &ignore);
        let combined = combine_hint_sources(&sources);

        assert_eq!(
            combined,
            load_hint_files(working_dir.path(), &filenames, &ignore)
        );
        assert!(combined.contains("global hint content"));
        assert!(combined.contains("project hint content"));

        let manager = PromptManager::new();
        let prompt = manager
            .builder()
            .with_goose_mode(GooseMode::Auto)
            .with_hint_sources(&sources)
            .build();
        assert!(prompt.contains("global hint content"));
        assert!(prompt.contains("project hint content"));
    }

    /// Everything `build` emits must be attributed to exactly one segment. The
    /// only text `build_segments` does not own is the literal
    /// `\n\n# Additional Instructions:\n\n` separator `build` inserts before the
    /// extras, which the report sweeps into its residual bucket.
    #[test]
    fn test_build_segments_reconstruct_the_built_prompt_byte_for_byte() {
        use crate::hints::HintScope;
        use std::path::PathBuf;

        let mut manager = PromptManager::new();
        manager.add_system_prompt_extra("project".to_string(), "Project extra".to_string());

        let hint_sources = vec![
            HintSource {
                path: PathBuf::from("/config/AGENTS.md"),
                scope: HintScope::Global,
                content: "Global hint body.".to_string(),
            },
            HintSource {
                path: PathBuf::from("/repo/AGENTS.md"),
                scope: HintScope::Project,
                content: "Project hint body.".to_string(),
            },
        ];
        let builder = || {
            manager
                .builder()
                .with_extension(ExtensionInfo::new("extension_A", "<how to use A>", true))
                .with_extension(ExtensionInfo::new("extension_B", "<how to use B>", false))
                .with_extension(ExtensionInfo::new("extension_C", "", false))
                .with_hint_sources(&hint_sources)
                .with_goose_mode(GooseMode::Auto)
        };

        let prompt = builder().build();
        let segments = builder().build_segments();

        assert_eq!(
            segments
                .base_template_blanked
                .matches("### Instructions")
                .count(),
            2,
            "literal template scaffolding must survive in the base segment"
        );

        let (base_part, extras_part) = prompt
            .split_once("\n\n# Additional Instructions:\n\n")
            .expect("extras are present");
        assert_eq!(
            extras_part,
            segments
                .extras
                .values()
                .cloned()
                .collect::<Vec<_>>()
                .join("\n\n")
        );

        let mut remaining = base_part.to_string();
        if let Some(moim_block) = &segments.moim_block {
            remaining = remaining.replacen(moim_block, "", 1);
        }
        for (name, instructions) in &segments.extension_instructions {
            assert!(
                remaining.contains(instructions),
                "{name} instructions are not in the built prompt"
            );
            remaining = remaining.replacen(instructions, "", 1);
        }

        assert_eq!(remaining, segments.base_template_blanked);
    }

    #[test]
    fn test_build_system_prompt_preserves_legitimate_unicode_in_extras() {
        let mut manager = PromptManager::new();
        let legitimate_unicode = "Instruction with 世界 and 🌍 emojis";
        manager.add_system_prompt_extra("test".to_string(), legitimate_unicode.to_string());

        let result = manager.builder().build();

        assert!(result.contains("世界"));
        assert!(result.contains("🌍"));
        assert!(result.contains("Instruction with"));
        assert!(result.contains("emojis"));
    }

    #[test]
    fn test_build_system_prompt_sanitizes_extension_instructions() {
        let manager = PromptManager::new();
        let malicious_extension_info = ExtensionInfo::new(
            "test_extension",
            "Extension help\u{E0041}\u{E0042}\u{E0043}hidden instructions",
            false,
        );

        let result = manager
            .builder()
            .with_extension(malicious_extension_info)
            .build();

        assert!(!result.contains('\u{E0041}'));
        assert!(!result.contains('\u{E0042}'));
        assert!(!result.contains('\u{E0043}'));
        assert!(result.contains("Extension help"));
        assert!(result.contains("hidden instructions"));
    }

    #[test]
    fn test_basic() {
        let manager = PromptManager::with_timestamp(DateTime::<Utc>::from_timestamp(0, 0).unwrap());

        let system_prompt = manager.builder().build();

        assert_snapshot!(system_prompt)
    }

    #[test]
    fn test_one_extension() {
        let manager = PromptManager::with_timestamp(DateTime::<Utc>::from_timestamp(0, 0).unwrap());

        let system_prompt = manager
            .builder()
            .with_extension(ExtensionInfo::new(
                "test",
                "how to use this extension",
                true,
            ))
            .build();

        assert_snapshot!(system_prompt)
    }

    #[test]
    fn test_typical_setup() {
        let manager = PromptManager::with_timestamp(DateTime::<Utc>::from_timestamp(0, 0).unwrap());

        let system_prompt = manager
            .builder()
            .with_extension(ExtensionInfo::new(
                "extension_A",
                "<instructions on how to use extension A>",
                true,
            ))
            .with_extension(ExtensionInfo::new(
                "extension_B",
                "<instructions on how to use extension B (no resources)>",
                false,
            ))
            .with_extension_and_tool_counts(MAX_EXTENSIONS + 1, MAX_TOOLS + 1)
            .build();

        assert_snapshot!(system_prompt)
    }

    #[tokio::test]
    async fn test_all_platform_extensions() {
        use crate::agents::platform_extensions::{PlatformExtensionContext, PLATFORM_EXTENSIONS};
        use crate::config::GooseMode;
        use crate::session::SessionManager;
        use std::sync::Arc;

        let tmp_dir = tempfile::tempdir().unwrap();
        let temp_root = tmp_dir.path().display().to_string();
        let _guard = env_lock::lock_env([
            ("HOME", Some(temp_root.as_str())),
            ("GOOSE_PATH_ROOT", Some(temp_root.as_str())),
        ]);
        let session_manager = Arc::new(SessionManager::new(tmp_dir.path().to_path_buf()));
        let session = session_manager
            .create_session(
                tmp_dir.path().to_path_buf(),
                "test session".to_owned(),
                crate::session::SessionType::Hidden,
                GooseMode::default(),
            )
            .await
            .unwrap();
        let context = PlatformExtensionContext {
            extension_manager: None,
            session_manager,
            session: Some(Arc::new(session)),
            use_login_shell_path: false,
        };

        let mut extensions: Vec<ExtensionInfo> = PLATFORM_EXTENSIONS
            .values()
            .map(|def| {
                let client = (def.client_factory)(context.clone());
                let instructions = client.get_instructions().unwrap_or_default();
                let has_resources = client
                    .get_info()
                    .and_then(|i| i.capabilities.resources.as_ref())
                    .is_some();
                ExtensionInfo::new(def.name, &instructions, has_resources)
            })
            .collect();

        extensions.sort_by(|a, b| a.name.cmp(&b.name));

        let manager = PromptManager::with_timestamp(DateTime::<Utc>::from_timestamp(0, 0).unwrap());
        let system_prompt = manager
            .builder()
            .with_extensions(extensions.into_iter())
            .build();

        assert_snapshot!(system_prompt);
    }
}
