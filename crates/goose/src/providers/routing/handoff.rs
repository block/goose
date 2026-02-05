//! Handoff memo generation for seamless provider switching

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::SystemTime;

use super::{ProviderConfig, RoutingError, RoutingResult, SwitchReason};

/// Handoff memo for provider switches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffMemo {
    /// Schema version for compatibility
    pub schema_version: u32,
    /// When this memo was generated
    pub generated_at: SystemTime,
    /// Previous provider configuration
    pub from_provider: ProviderConfig,
    /// Target provider configuration
    pub to_provider: ProviderConfig,
    /// Reason for the switch
    pub switch_reason: SwitchReason,
    /// Current project state summary
    pub project_state: ProjectState,
    /// Execution context
    pub execution_context: ExecutionContext,
    /// Important constraints and requirements
    pub constraints: Vec<String>,
    /// Anti-pattern warnings (what not to do)
    pub anti_patterns: Vec<String>,
    /// Recommended next actions
    pub next_actions: Vec<String>,
}

/// Summary of current project state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectState {
    /// Project goal/objective
    pub goal: String,
    /// Current progress summary
    pub progress: String,
    /// Files created or modified
    pub files_touched: Vec<String>,
    /// Key decisions made
    pub decisions: Vec<String>,
    /// Open tasks and blockers
    pub open_tasks: Vec<String>,
    /// Last known good state (build/test status)
    pub last_good_state: Option<String>,
}

/// Current execution context
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionContext {
    /// Tools and commands run
    pub commands_run: Vec<String>,
    /// Current working directory
    pub working_directory: Option<String>,
    /// Environment variables set
    pub env_vars: Vec<String>,
    /// Active dependencies/requirements
    pub dependencies: Vec<String>,
    /// Error patterns encountered
    pub error_patterns: Vec<String>,
}

impl Default for ProjectState {
    fn default() -> Self {
        Self {
            goal: "Continue project development".to_string(),
            progress: "In progress".to_string(),
            files_touched: Vec::new(),
            decisions: Vec::new(),
            open_tasks: Vec::new(),
            last_good_state: None,
        }
    }
}

impl HandoffMemo {
    /// Generate a new handoff memo for a provider switch
    pub fn generate_for_switch(
        from_provider: &ProviderConfig,
        to_provider: &ProviderConfig,
        switch_reason: &SwitchReason,
        total_requests: u64,
    ) -> RoutingResult<Self> {
        let memo = Self {
            schema_version: 1,
            generated_at: SystemTime::now(),
            from_provider: from_provider.clone(),
            to_provider: to_provider.clone(),
            switch_reason: switch_reason.clone(),
            project_state: Self::generate_project_state(total_requests),
            execution_context: Self::generate_execution_context(),
            constraints: Self::generate_constraints(switch_reason),
            anti_patterns: Self::generate_anti_patterns(),
            next_actions: Self::generate_next_actions(switch_reason),
        };

        Ok(memo)
    }

    /// Generate project state summary
    fn generate_project_state(total_requests: u64) -> ProjectState {
        let mut state = ProjectState::default();

        if total_requests > 0 {
            state.progress = format!("Processed {} requests successfully", total_requests);
        }

        // Add common project constraints
        state
            .decisions
            .push("Using provider routing for resilience".to_string());
        state
            .open_tasks
            .push("Continue with current objectives".to_string());

        state
    }

    /// Generate execution context
    fn generate_execution_context() -> ExecutionContext {
        ExecutionContext {
            commands_run: vec!["Provider switch initiated".to_string()],
            working_directory: std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(String::from)),
            env_vars: Vec::new(), // Don't include actual env vars for security
            dependencies: vec!["Provider routing system".to_string()],
            error_patterns: Vec::new(),
        }
    }

    /// Generate constraints based on switch reason
    fn generate_constraints(switch_reason: &SwitchReason) -> Vec<String> {
        let mut constraints = Vec::new();

        constraints.push("Maintain conversation context and memory".to_string());
        constraints.push("Preserve project objectives and requirements".to_string());
        constraints.push("Continue from where previous provider left off".to_string());

        match switch_reason {
            SwitchReason::QuotaExhausted => {
                constraints.push("Previous provider hit quota limits".to_string());
                constraints.push("Monitor token usage on new provider".to_string());
            }
            SwitchReason::RateLimited => {
                constraints.push("Previous provider was rate limited".to_string());
                constraints.push("Use appropriate request pacing".to_string());
            }
            SwitchReason::EndpointUnreachable => {
                constraints.push("Previous provider had connectivity issues".to_string());
                constraints.push("Verify new provider endpoint is stable".to_string());
            }
            SwitchReason::UserInitiated => {
                constraints.push("User requested provider change".to_string());
                constraints.push("Respect user preferences for this session".to_string());
            }
            _ => {}
        }

        constraints
    }

    /// Generate anti-patterns to avoid
    fn generate_anti_patterns() -> Vec<String> {
        vec![
            "Do not start from scratch - continue existing work".to_string(),
            "Do not ignore previous context and decisions".to_string(),
            "Do not repeat work already completed".to_string(),
            "Do not change project goals without explicit instruction".to_string(),
            "Do not introduce breaking changes without justification".to_string(),
        ]
    }

    /// Generate recommended next actions
    fn generate_next_actions(switch_reason: &SwitchReason) -> Vec<String> {
        let mut actions = vec![
            "Review project state and continue from current point".to_string(),
            "Maintain consistency with previous work".to_string(),
            "Verify new provider capabilities match requirements".to_string(),
        ];

        match switch_reason {
            SwitchReason::QuotaExhausted | SwitchReason::RateLimited => {
                actions.push("Monitor usage to avoid similar issues".to_string());
            }
            SwitchReason::EndpointUnreachable | SwitchReason::Timeout => {
                actions.push("Verify connectivity and endpoint health".to_string());
            }
            SwitchReason::CapabilityMismatch => {
                actions.push("Verify new provider supports required capabilities".to_string());
            }
            _ => {}
        }

        actions
    }

    /// Compute SHA256 digest of the memo for tracking
    pub fn compute_digest(&self) -> String {
        let memo_json = serde_json::to_string(self).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(memo_json.as_bytes());
        format!("sha256:{:x}", hasher.finalize())
    }

    /// Convert to markdown format for human readability
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str("# Provider Switch Handoff Memo\n\n");

        md.push_str(&format!("**Generated:** {:?}\n", self.generated_at));
        md.push_str(&format!("**Switch Reason:** {:?}\n", self.switch_reason));
        md.push_str(&format!(
            "**From:** {} ({})\n",
            self.from_provider.provider, self.from_provider.model
        ));
        md.push_str(&format!(
            "**To:** {} ({})\n\n",
            self.to_provider.provider, self.to_provider.model
        ));

        md.push_str("## Project State\n\n");
        md.push_str(&format!("**Goal:** {}\n", self.project_state.goal));
        md.push_str(&format!(
            "**Progress:** {}\n\n",
            self.project_state.progress
        ));

        if !self.project_state.files_touched.is_empty() {
            md.push_str("**Files Touched:**\n");
            for file in &self.project_state.files_touched {
                md.push_str(&format!("- {}\n", file));
            }
            md.push('\n');
        }

        if !self.project_state.decisions.is_empty() {
            md.push_str("**Key Decisions:**\n");
            for decision in &self.project_state.decisions {
                md.push_str(&format!("- {}\n", decision));
            }
            md.push('\n');
        }

        if !self.project_state.open_tasks.is_empty() {
            md.push_str("**Open Tasks:**\n");
            for task in &self.project_state.open_tasks {
                md.push_str(&format!("- {}\n", task));
            }
            md.push('\n');
        }

        if !self.constraints.is_empty() {
            md.push_str("## Constraints\n\n");
            for constraint in &self.constraints {
                md.push_str(&format!("- {}\n", constraint));
            }
            md.push('\n');
        }

        if !self.anti_patterns.is_empty() {
            md.push_str("## Anti-Patterns (Do NOT Do)\n\n");
            for anti_pattern in &self.anti_patterns {
                md.push_str(&format!("- ❌ {}\n", anti_pattern));
            }
            md.push('\n');
        }

        if !self.next_actions.is_empty() {
            md.push_str("## Recommended Next Actions\n\n");
            for action in &self.next_actions {
                md.push_str(&format!("- ✅ {}\n", action));
            }
            md.push('\n');
        }

        md.push_str("---\n");
        md.push_str(&format!("**Memo Digest:** {}\n", self.compute_digest()));

        md
    }

    /// Save handoff memo to file
    pub async fn save_to_file(&self, path: &std::path::Path) -> RoutingResult<()> {
        let markdown = self.to_markdown();
        tokio::fs::write(path, markdown).await.map_err(|e| {
            RoutingError::HandoffError(format!("Failed to save handoff memo: {}", e))
        })?;
        Ok(())
    }

    /// Load handoff memo from file
    pub async fn load_from_file(path: &std::path::Path) -> RoutingResult<Self> {
        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            RoutingError::HandoffError(format!("Failed to read handoff memo: {}", e))
        })?;

        // Try to parse as JSON first
        if let Ok(memo) = serde_json::from_str::<Self>(&content) {
            return Ok(memo);
        }

        // If not JSON, this might be a markdown file - return error for now
        Err(RoutingError::HandoffError(
            "Handoff memo parsing from markdown not implemented".to_string(),
        ))
    }

    /// Update project state with new information
    pub fn update_project_state(&mut self, updates: ProjectStateUpdate) {
        if let Some(goal) = updates.goal {
            self.project_state.goal = goal;
        }
        if let Some(progress) = updates.progress {
            self.project_state.progress = progress;
        }
        self.project_state
            .files_touched
            .extend(updates.files_touched);
        self.project_state.decisions.extend(updates.decisions);
        self.project_state.open_tasks.extend(updates.open_tasks);
        if let Some(state) = updates.last_good_state {
            self.project_state.last_good_state = Some(state);
        }
    }

    /// Add execution context
    pub fn add_execution_context(&mut self, context: ExecutionContextUpdate) {
        self.execution_context
            .commands_run
            .extend(context.commands_run);
        self.execution_context.env_vars.extend(context.env_vars);
        self.execution_context
            .dependencies
            .extend(context.dependencies);
        self.execution_context
            .error_patterns
            .extend(context.error_patterns);

        if let Some(workdir) = context.working_directory {
            self.execution_context.working_directory = Some(workdir);
        }
    }
}

/// Updates for project state
#[derive(Debug, Clone, Default)]
pub struct ProjectStateUpdate {
    pub goal: Option<String>,
    pub progress: Option<String>,
    pub files_touched: Vec<String>,
    pub decisions: Vec<String>,
    pub open_tasks: Vec<String>,
    pub last_good_state: Option<String>,
}

/// Updates for execution context
#[derive(Debug, Clone, Default)]
pub struct ExecutionContextUpdate {
    pub commands_run: Vec<String>,
    pub working_directory: Option<String>,
    pub env_vars: Vec<String>,
    pub dependencies: Vec<String>,
    pub error_patterns: Vec<String>,
}

/// Handoff memo generator utility
pub struct HandoffGenerator {
    /// Template for common constraints
    common_constraints: Vec<String>,
    /// Template for common anti-patterns
    common_anti_patterns: Vec<String>,
}

impl HandoffGenerator {
    /// Create a new handoff generator
    pub fn new() -> Self {
        Self {
            common_constraints: vec![
                "Maintain conversation context and memory".to_string(),
                "Preserve project objectives and requirements".to_string(),
                "Continue from where previous provider left off".to_string(),
                "Respect established coding patterns and architecture".to_string(),
            ],
            common_anti_patterns: vec![
                "Do not start from scratch - continue existing work".to_string(),
                "Do not ignore previous context and decisions".to_string(),
                "Do not repeat work already completed".to_string(),
                "Do not change project goals without explicit instruction".to_string(),
            ],
        }
    }

    /// Generate a comprehensive handoff memo
    pub fn generate_comprehensive(
        &self,
        from_provider: &ProviderConfig,
        to_provider: &ProviderConfig,
        switch_reason: &SwitchReason,
        project_update: Option<ProjectStateUpdate>,
        context_update: Option<ExecutionContextUpdate>,
    ) -> RoutingResult<HandoffMemo> {
        let mut memo = HandoffMemo::generate_for_switch(
            from_provider,
            to_provider,
            switch_reason,
            0, // Will be updated with actual request count
        )?;

        // Add common constraints and anti-patterns
        memo.constraints.extend(self.common_constraints.clone());
        memo.anti_patterns.extend(self.common_anti_patterns.clone());

        // Apply updates if provided
        if let Some(project_update) = project_update {
            memo.update_project_state(project_update);
        }

        if let Some(context_update) = context_update {
            memo.add_execution_context(context_update);
        }

        Ok(memo)
    }
}

impl Default for HandoffGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handoff_memo_generation() {
        let from_config =
            ProviderConfig::new("anthropic", "anthropic_primary", "claude-sonnet-3.5");
        let to_config = ProviderConfig::new("openai", "openai_primary", "gpt-4o");
        let reason = SwitchReason::QuotaExhausted;

        let memo = HandoffMemo::generate_for_switch(&from_config, &to_config, &reason, 10).unwrap();

        assert_eq!(memo.from_provider.provider, "anthropic");
        assert_eq!(memo.to_provider.provider, "openai");
        assert_eq!(memo.switch_reason, SwitchReason::QuotaExhausted);
        assert!(!memo.constraints.is_empty());
        assert!(!memo.anti_patterns.is_empty());
    }

    #[test]
    fn test_digest_computation() {
        let from_config =
            ProviderConfig::new("anthropic", "anthropic_primary", "claude-sonnet-3.5");
        let to_config = ProviderConfig::new("openai", "openai_primary", "gpt-4o");
        let reason = SwitchReason::UserInitiated;

        let memo1 = HandoffMemo::generate_for_switch(&from_config, &to_config, &reason, 5).unwrap();
        let memo2 = HandoffMemo::generate_for_switch(&from_config, &to_config, &reason, 5).unwrap();

        // Digests should be different due to timestamp differences
        assert_ne!(memo1.compute_digest(), memo2.compute_digest());
    }

    #[test]
    fn test_markdown_generation() {
        let from_config =
            ProviderConfig::new("anthropic", "anthropic_primary", "claude-sonnet-3.5");
        let to_config = ProviderConfig::new("openai", "openai_primary", "gpt-4o");
        let reason = SwitchReason::RateLimited;

        let memo = HandoffMemo::generate_for_switch(&from_config, &to_config, &reason, 15).unwrap();
        let markdown = memo.to_markdown();

        assert!(markdown.contains("# Provider Switch Handoff Memo"));
        assert!(markdown.contains("**From:** anthropic"));
        assert!(markdown.contains("**To:** openai"));
        assert!(markdown.contains("RateLimited"));
    }

    #[test]
    fn test_handoff_generator() {
        let generator = HandoffGenerator::new();
        let from_config =
            ProviderConfig::new("anthropic", "anthropic_primary", "claude-sonnet-3.5");
        let to_config = ProviderConfig::new("openai", "openai_primary", "gpt-4o");
        let reason = SwitchReason::CapabilityMismatch;

        let project_update = ProjectStateUpdate {
            goal: Some("Build a web application".to_string()),
            progress: Some("Frontend complete, backend in progress".to_string()),
            files_touched: vec!["src/main.rs".to_string(), "Cargo.toml".to_string()],
            ..Default::default()
        };

        let memo = generator
            .generate_comprehensive(
                &from_config,
                &to_config,
                &reason,
                Some(project_update),
                None,
            )
            .unwrap();

        assert_eq!(memo.project_state.goal, "Build a web application");
        assert!(memo
            .project_state
            .files_touched
            .contains(&"src/main.rs".to_string()));
        assert!(memo.constraints.len() > 4); // Should have common + specific constraints
    }
}
