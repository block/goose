use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::config::Config;
use crate::conversation::message::MessageContent;
use crate::conversation::Conversation;
use crate::providers;

// Embedded YAML content for pre-made roles
const PREMADE_ROLES_YAML: &str = include_str!("premade_roles.yaml");

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchType {
    #[default]
    Any,
    All,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerSource {
    Human,   // Only trigger on human messages
    Machine, // Only trigger on machine-generated events
    #[default]
    Any, // Trigger on either
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComplexityLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TriggerRules {
    /// Keywords to match in user messages
    #[serde(default)]
    pub keywords: Vec<String>,

    /// How to match keywords - "any" or "all"
    #[serde(default)]
    pub match_type: MatchType,

    /// Trigger after a tool execution failure
    #[serde(default)]
    pub on_failure: bool,

    /// Trigger after any tool usage
    #[serde(default)]
    pub after_tool_use: bool,

    /// Trigger after N consecutive tool uses
    #[serde(default)]
    pub consecutive_tools: Option<usize>,

    /// Trigger after N consecutive failures
    #[serde(default)]
    pub consecutive_failures: Option<usize>,

    /// Trigger after N consecutive machine messages (no human input)
    #[serde(default)]
    pub machine_messages_without_human: Option<usize>,

    /// Trigger after N total tool calls since last human message
    #[serde(default)]
    pub tools_since_human: Option<usize>,

    /// Trigger after N messages since last human input
    #[serde(default)]
    pub messages_since_human: Option<usize>,

    /// Complexity analysis threshold
    #[serde(default)]
    pub complexity_threshold: Option<ComplexityLevel>,

    /// Trigger on the first turn of a conversation
    #[serde(default)]
    pub first_turn: bool,

    /// Source of trigger (human, machine, or any)
    #[serde(default)]
    pub source: TriggerSource,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rules {
    pub triggers: TriggerRules,

    /// Number of turns this model stays active once triggered
    #[serde(default = "default_active_turns")]
    pub active_turns: usize,

    /// Priority when multiple models match (higher = more important)
    #[serde(default)]
    pub priority: i32,
}

fn default_active_turns() -> usize {
    5
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelConfig {
    pub provider: String,
    pub model: String,
    pub role: String,
    #[serde(default)]
    pub rules: Option<Rules>, // Optional - can inherit from premade
}

#[derive(Debug, Clone, Deserialize)]
struct PremadeRole {
    pub role: String,
    pub rules: Rules,
}

#[derive(Debug, Clone, Deserialize)]
struct PremadeRoles {
    roles: Vec<PremadeRole>,
}

// Complete model config with rules (after merging)
#[derive(Debug, Clone)]
struct CompleteModelConfig {
    pub provider: String,
    pub model: String,
    pub role: String,
    pub rules: Rules,
}

/// Tracks the state of a specific model's usage
#[derive(Debug, Clone, Default)]
struct ModelState {
    last_invoked_turn: Option<usize>,
    invocation_count: usize,
}

/// AutoPilot manages automatic model switching based on conversation context
pub struct AutoPilot {
    model_configs: Vec<CompleteModelConfig>,
    model_states: HashMap<String, ModelState>,
    original_provider: Option<Arc<dyn crate::providers::base::Provider>>,
    switch_active: bool,
    current_role: Option<String>,
}

impl AutoPilot {
    /// Load pre-made role rules from embedded YAML
    fn load_premade_rules() -> HashMap<String, Rules> {
        match serde_yaml::from_str::<PremadeRoles>(PREMADE_ROLES_YAML) {
            Ok(premade) => {
                debug!("Loaded {} pre-made role rules", premade.roles.len());
                premade
                    .roles
                    .into_iter()
                    .map(|r| (r.role, r.rules))
                    .collect()
            }
            Err(e) => {
                warn!("Failed to load pre-made roles: {}", e);
                HashMap::new()
            }
        }
    }

    /// Merge user configs with pre-made rules
    /// User must provide provider and model, but rules are optional (inherit from premade)
    fn merge_configs(
        premade_rules: HashMap<String, Rules>,
        user_configs: Vec<ModelConfig>,
    ) -> Vec<CompleteModelConfig> {
        let mut complete_configs = Vec::new();

        for user_config in user_configs {
            // Get the rules - either from user config or premade
            let rules = if let Some(user_rules) = user_config.rules {
                // User provided custom rules for this role
                user_rules
            } else if let Some(premade_rules) = premade_rules.get(&user_config.role) {
                // Use premade rules for this role
                premade_rules.clone()
            } else {
                // No premade rules and no user rules - skip this config
                warn!(
                    "No rules found for role '{}' - neither in user config nor premade. Skipping.",
                    user_config.role
                );
                continue;
            };

            complete_configs.push(CompleteModelConfig {
                provider: user_config.provider,
                model: user_config.model,
                role: user_config.role,
                rules,
            });
        }

        complete_configs
    }

    /// Create a new AutoPilot instance, loading model configurations from config
    pub fn new() -> Self {
        let config = Config::global();

        // Load pre-made role rules
        let premade_rules = Self::load_premade_rules();

        // Try to load user models configuration from config.yaml
        let user_models: Vec<ModelConfig> = config
            .get_param("x-advanced-models")
            .unwrap_or_else(|_| Vec::new());

        // Merge configs - user provides provider/model, rules come from premade or user override
        let models = Self::merge_configs(premade_rules, user_models);

        let mut model_states = HashMap::new();
        for model in &models {
            model_states.insert(model.role.clone(), ModelState::default());
        }

        if !models.is_empty() {
            debug!(
                "AutoPilot initialized with {} model configurations",
                models.len()
            );
            for model in &models {
                debug!(
                    "Role '{}': {}/{} (priority: {})",
                    model.role, model.provider, model.model, model.rules.priority
                );
            }
        } else {
            debug!("AutoPilot: No model configurations found in config");
        }

        Self {
            model_configs: models,
            model_states,
            original_provider: None,
            switch_active: false,
            current_role: None,
        }
    }
}

impl Default for AutoPilot {
    fn default() -> Self {
        Self::new()
    }
}

impl AutoPilot {
    /// Count the current turn number (number of user messages)
    fn count_turns(&self, conversation: &Conversation) -> usize {
        conversation
            .messages()
            .iter()
            .filter(|msg| msg.role == rmcp::model::Role::User)
            .count()
    }

    /// Check if keywords match based on match_type
    fn check_keywords(text: &str, keywords: &[String], match_type: &MatchType) -> bool {
        if keywords.is_empty() {
            return false;
        }

        let text_lower = text.to_lowercase();
        match match_type {
            MatchType::Any => keywords
                .iter()
                .any(|kw| text_lower.contains(&kw.to_lowercase())),
            MatchType::All => keywords
                .iter()
                .all(|kw| text_lower.contains(&kw.to_lowercase())),
        }
    }

    /// Score the complexity of a paragraph/sentence as Low / Medium / High.
    /// This uses a variety of simple (but known) fast algorithms.
    /// Looks like generated code, only partly is, mic did work over it.
    /// It appears complex, but the idea is to have a fast way to know if some body of text is hard to read or complex in any way.
    ///
    /// Algorithms included:
    /// - **Flesch Reading Ease (FRE)** → higher = simpler
    /// - **Flesch–Kincaid Grade Level (FKGL)** → higher = harder
    /// - **Gunning Fog Index (FOG)** → higher = harder
    /// - **Coleman–Liau Index (CLI)** → higher = harder
    /// - **Automated Readability Index (ARI)** → higher = harder
    /// - **LIX (Läsbarhetsindex)** → higher = harder
    ///
    /// some features layered on top of the formulas:
    /// - **Long-word ratio** (>6 letters): jargon proxy → penalizes if high
    /// - **Clause density** (commas, semicolons, parentheses per sentence): proxy for syntactic load → penalizes if high
    /// - **Instructional boost**: if sentences are short, long-word ratio is low, and clauses are few, give a small positive bump (to better classify "simple instruction" style text)
    ///
    /// The formulas are normalized into a 0–100 "simplicity" scale, then blended with weights.
    /// Heuristic penalties/bonuses are applied, and the final result is bucketed in to the following
    ///   >70 = Low (simple), 40–70 = Medium, <40 = High (complex).
    pub fn analyze_complexity(text: &str) -> ComplexityLevel {
        // --- tokenization ---
        static RE_WORD: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"[A-Za-z]+(?:'[A-Za-z]+)?").unwrap());
        static RE_SENT: Lazy<Regex> = Lazy::new(|| Regex::new(r"[.!?]+").unwrap());
        static RE_CLAUSE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[,:;()—-]").unwrap());

        let words: Vec<&str> = RE_WORD.find_iter(text).map(|m| m.as_str()).collect();
        let w = words.len().max(1);

        // Automatically classify anything less than 4 words as Low complexity
        if w < 4 {
            return ComplexityLevel::Low;
        }
        let s = RE_SENT.find_iter(text).count().max(1);

        let letters = text.chars().filter(|c| c.is_alphabetic()).count();
        let chars_no_space = text.chars().filter(|c| !c.is_whitespace()).count();
        let clauses = RE_CLAUSE.find_iter(text).count();

        // syllable, long-word, polysyllable counts
        let mut syl = 0usize;
        let mut polys = 0usize;
        let mut longw = 0usize;
        for &wd in &words {
            let sy = Self::syllables(wd);
            syl += sy;
            if sy >= 3 {
                polys += 1;
            }
            if wd.len() > 6 {
                longw += 1;
            }
        }

        // --- readability formulas ---
        let avg_wps = w as f32 / s as f32; // words per sentence
        let avg_syl = syl as f32 / w as f32;

        // 1. Flesch Reading Ease (FRE)
        let fre = 206.835 - 1.015 * avg_wps - 84.6 * avg_syl;

        // 2. Flesch–Kincaid Grade Level (FKGL)
        let fkgl = 0.39 * avg_wps + 11.8 * avg_syl - 15.59;

        // 3. Gunning Fog Index
        let fog = 0.4 * (avg_wps + 100.0 * (polys as f32 / w as f32));

        // 4. Coleman–Liau Index (CLI)
        let cli = {
            let l = 100.0 * (letters as f32 / w as f32);
            let s100 = 100.0 * (s as f32 / w as f32);
            0.0588 * l - 0.296 * s100 - 15.8
        };

        // 5. Automated Readability Index (ARI)
        let ari = 4.71 * (chars_no_space as f32 / w as f32) + 0.5 * avg_wps - 21.43;

        // 6. LIX (Läsbarhetsindex)
        let lix = avg_wps + 100.0 * (longw as f32 / w as f32);

        // --- normalize into 0..100 simplicity ---
        let clamp01 = |x: f32| x.clamp(0.0, 1.0);
        let inv_grade = |g: f32| 100.0 * (1.0 - clamp01(g / 18.0)); // 0 grade→100 simple, 18+→0
        let f_fre = 100.0 * clamp01(fre / 100.0);
        let f_fkgl = inv_grade(fkgl);
        let f_fog = inv_grade(fog);
        let f_cli = inv_grade(cli);
        let f_ari = inv_grade(ari);
        let f_lix = 100.0 * (1.0 - clamp01((lix - 20.0) / 40.0)); // LIX 20..60 → 100..0

        // Weighted blend of formulas (tuned weights, sum < 1.0)
        let mut simplicity = 0.30 * f_fre
            + 0.16 * f_fkgl
            + 0.12 * f_fog
            + 0.10 * f_cli
            + 0.07 * f_ari
            + 0.08 * f_lix;

        // --- heuristic adjustments ---
        let long_ratio = longw as f32 / w as f32;
        let clause_density = clauses as f32 / s as f32;

        // Penalty for jargon-ish long words (up to -20)
        simplicity -= (long_ratio * 20.0).min(20.0);

        // Penalty for heavy clause punctuation (up to -15 when clauses/sentence ≳ 3)
        simplicity -= ((clause_density / 3.0) * 15.0).min(15.0);

        // Boost if text looks like simple instructions:
        // short sentences, few long words, low clause punctuation
        if avg_wps < 14.0 && long_ratio < 0.12 && clause_density < 0.8 {
            simplicity += 5.0;
        }

        // --- final bucketing ---
        let score = simplicity.clamp(0.0, 100.0);
        if score > 70.0 {
            ComplexityLevel::Low
        } else if score >= 40.0 {
            ComplexityLevel::Medium
        } else {
            ComplexityLevel::High
        }
    }

    /// Tiny syllable guesser (used by FRE, FKGL, Fog)
    fn syllables(word: &str) -> usize {
        let w = word.to_lowercase();
        let mut count = 0usize;
        let mut prev_v = false;
        for c in w.chars() {
            let v = matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'y');
            if v && !prev_v {
                count += 1;
            }
            prev_v = v;
        }
        if w.ends_with('e') && count > 1 {
            count -= 1;
        }
        count.max(1)
    }

    /// Check if the trigger source matches the last message
    fn check_source(&self, conversation: &Conversation, source: &TriggerSource) -> bool {
        let last_msg = conversation.messages().last();

        match source {
            TriggerSource::Human => {
                // Check if the last message is from a human
                last_msg.is_some_and(|msg| msg.role == rmcp::model::Role::User)
            }
            TriggerSource::Machine => {
                // Check if the last message is from the assistant
                last_msg.is_some_and(|msg| msg.role == rmcp::model::Role::Assistant)
            }
            TriggerSource::Any => true,
        }
    }

    /// Count consecutive tool uses at the end of the conversation
    fn count_consecutive_tools(&self, conversation: &Conversation) -> usize {
        let messages = conversation.messages();
        let mut count = 0;

        // Work backwards through assistant messages
        for msg in messages.iter().rev() {
            if msg.role != rmcp::model::Role::Assistant {
                continue;
            }

            let has_tool = msg
                .content
                .iter()
                .any(|content| matches!(content, MessageContent::ToolRequest(_)));

            if has_tool {
                count += 1;
            } else {
                break; // Stop at first non-tool message
            }
        }

        count
    }

    /// Count consecutive tool failures
    fn count_consecutive_failures(&self, conversation: &Conversation) -> usize {
        let messages = conversation.messages();
        let mut count = 0;

        // Work backwards looking for tool responses
        for msg in messages.iter().rev() {
            let has_failure = msg.content.iter().any(|content| {
                if let MessageContent::ToolResponse(response) = content {
                    response.tool_result.is_err()
                } else {
                    false
                }
            });

            if has_failure {
                count += 1;
            } else if msg
                .content
                .iter()
                .any(|c| matches!(c, MessageContent::ToolResponse(_)))
            {
                // Found a successful tool response, stop counting
                break;
            }
        }

        count
    }

    /// Count messages since last human input
    fn count_messages_since_human(&self, conversation: &Conversation) -> usize {
        let messages = conversation.messages();
        let mut count = 0;

        // Work backwards counting messages until we find a User message
        for msg in messages.iter().rev() {
            if msg.role == rmcp::model::Role::User {
                break;
            }
            count += 1;
        }

        count
    }

    /// Count tool calls since last human message
    fn count_tools_since_human(&self, conversation: &Conversation) -> usize {
        let messages = conversation.messages();
        let mut tool_count = 0;

        // Work backwards counting tool requests until we find a User message
        for msg in messages.iter().rev() {
            if msg.role == rmcp::model::Role::User {
                break;
            }

            // Count tool requests in this message
            tool_count += msg
                .content
                .iter()
                .filter(|content| matches!(content, MessageContent::ToolRequest(_)))
                .count();
        }

        tool_count
    }

    /// Count consecutive machine messages (assistant messages without human interruption)
    fn count_machine_messages_without_human(&self, conversation: &Conversation) -> usize {
        let messages = conversation.messages();
        let mut count = 0;

        // Work backwards counting assistant messages until we find a user message
        for msg in messages.iter().rev() {
            match msg.role {
                rmcp::model::Role::User => break,
                rmcp::model::Role::Assistant => count += 1,
            }
        }

        count
    }

    /// Check if there was a recent tool failure
    fn check_recent_failure(&self, conversation: &Conversation) -> bool {
        // Look for actual tool failures in recent messages
        conversation
            .messages()
            .iter()
            .rev()
            .take(3) // Check last 3 messages
            .any(|msg| {
                msg.content.iter().any(|content| {
                    if let MessageContent::ToolResponse(response) = content {
                        response.tool_result.is_err()
                    } else {
                        false
                    }
                })
            })
    }

    /// Evaluate if a model's rules are satisfied
    fn evaluate_rules(
        &self,
        model: &CompleteModelConfig,
        conversation: &Conversation,
        current_turn: usize,
    ) -> bool {
        if !self.check_source(conversation, &model.rules.triggers.source) {
            return false;
        }

        let triggers = &model.rules.triggers;
        let mut triggered = false;

        if triggers.first_turn && current_turn == 1 {
            debug!("AutoPilot: '{}' role triggering on first turn", model.role);
            triggered = true;
        }

        if !triggers.keywords.is_empty() {
            if let Some(text) = conversation
                .messages()
                .iter()
                .rev()
                .find(|msg| msg.role == rmcp::model::Role::User)
                .and_then(|msg| msg.content.first())
                .and_then(|content| content.as_text())
            {
                if Self::check_keywords(text, &triggers.keywords, &triggers.match_type) {
                    triggered = true;
                }
            }
        }

        if triggers.on_failure && self.check_recent_failure(conversation) {
            triggered = true;
        }

        if let Some(threshold) = triggers.consecutive_failures {
            if self.count_consecutive_failures(conversation) >= threshold {
                triggered = true;
            }
        }

        if triggers.after_tool_use {
            let has_recent_tool = conversation
                .messages()
                .iter()
                .rev()
                .find(|msg| msg.role == rmcp::model::Role::Assistant)
                .map(|msg| {
                    msg.content
                        .iter()
                        .any(|content| matches!(content, MessageContent::ToolRequest(_)))
                })
                .unwrap_or(false);

            if has_recent_tool {
                triggered = true;
            }
        }

        if let Some(threshold) = triggers.consecutive_tools {
            if self.count_consecutive_tools(conversation) >= threshold {
                triggered = true;
            }
        }

        if let Some(threshold) = triggers.machine_messages_without_human {
            if self.count_machine_messages_without_human(conversation) >= threshold {
                triggered = true;
            }
        }

        if let Some(threshold) = triggers.tools_since_human {
            if self.count_tools_since_human(conversation) >= threshold {
                triggered = true;
            }
        }

        if let Some(threshold) = triggers.messages_since_human {
            if self.count_messages_since_human(conversation) >= threshold {
                triggered = true;
            }
        }

        if let Some(ref threshold) = triggers.complexity_threshold {
            if let Some(text) = conversation
                .messages()
                .iter()
                .rev()
                .find(|msg| msg.role == rmcp::model::Role::User)
                .and_then(|msg| msg.content.first())
                .and_then(|content| content.as_text())
            {
                let complexity = Self::analyze_complexity(text);

                matches!(
                    (threshold, complexity),
                    (ComplexityLevel::Low, ComplexityLevel::Medium)
                        | (ComplexityLevel::Low, ComplexityLevel::High)
                        | (ComplexityLevel::Medium, ComplexityLevel::Medium)
                        | (ComplexityLevel::Medium, ComplexityLevel::High)
                        | (ComplexityLevel::High, ComplexityLevel::High)
                );
            }
        }

        triggered
    }

    /// Check if a model switch should occur based on the conversation
    /// Returns Some((provider, role, model)) if a switch should happen, None otherwise
    pub async fn check_for_switch(
        &mut self,
        conversation: &Conversation,
        current_provider: Arc<dyn crate::providers::base::Provider>,
    ) -> Result<Option<(Arc<dyn crate::providers::base::Provider>, String, String)>> {
        debug!("AutoPilot: Checking conversation for model switch");

        let current_turn = self.count_turns(conversation);

        // If we already switched, evaluate if we should switch to a different model
        // (including potentially switching back to original eg when turns are done)
        if self.switch_active {
            debug!(
                "AutoPilot: Currently switched to '{}', evaluating alternatives",
                self.current_role.as_deref().unwrap_or("unknown")
            );

            let should_switch = self.should_switch_from_current(conversation, current_turn);

            if let Some((new_provider, new_role, new_model)) = should_switch? {
                debug!(
                    "AutoPilot: Switching from '{}' to '{}'",
                    self.current_role.as_deref().unwrap_or("unknown"),
                    new_role
                );

                if new_role == "original" {
                    self.switch_active = false;
                    self.current_role = None;
                    self.original_provider = None;
                } else {
                    self.current_role = Some(new_role.clone());
                }

                return Ok(Some((new_provider, new_role, new_model)));
            }
            return Ok(None);
        }

        // Evaluate all models to use based on the rules
        // Get candidates and find the best match, if any, to switch to
        let mut candidates: Vec<(&CompleteModelConfig, i32)> = Vec::new();

        for model in &self.model_configs {
            if self.evaluate_rules(model, conversation, current_turn) {
                candidates.push((model, model.rules.priority));
            }
        }

        candidates.sort_by_key(|(_, priority)| -priority);

        if let Some((best_model, priority)) = candidates.first() {
            debug!(
                "AutoPilot: Switching to '{}' role with {} model {} (priority: {})",
                best_model.role, best_model.provider, best_model.model, priority
            );

            let state = self.model_states.get_mut(&best_model.role).unwrap();
            state.last_invoked_turn = Some(current_turn);
            state.invocation_count += 1;

            self.original_provider = Some(current_provider);
            self.switch_active = true;
            self.current_role = Some(best_model.role.clone());

            let model = crate::model::ModelConfig::new_or_fail(&best_model.model);
            let new_provider = providers::create(&best_model.provider, model).await?;

            return Ok(Some((
                new_provider,
                best_model.role.clone(),
                best_model.model.clone(),
            )));
        }

        Ok(None)
    }

    /// Determine if we should switch from the current model to another (including back to original)
    #[allow(clippy::type_complexity)]
    fn should_switch_from_current(
        &self,
        _conversation: &Conversation,
        current_turn: usize,
    ) -> Result<Option<(Arc<dyn crate::providers::base::Provider>, String, String)>> {
        // Strategy: Stay in the current role until its cooldown period has elapsed
        // This ensures the specialized model gets to complete its work

        let current_role = self.current_role.as_ref().unwrap();
        let current_model = self.model_configs.iter().find(|m| &m.role == current_role);
        let current_state = &self.model_states[current_role];

        if let (Some(current_model), Some(last_invoked_turn)) =
            (current_model, current_state.last_invoked_turn)
        {
            let turns_since_invoked = current_turn.saturating_sub(last_invoked_turn);

            debug!("AutoPilot: Current model '{}' invoked at turn {}, current turn {}, turns since: {}, active_turns: {}", 
                   current_role, last_invoked_turn, current_turn, turns_since_invoked, current_model.rules.active_turns);

            // If we're still within the active period, stay with current model
            if turns_since_invoked < current_model.rules.active_turns {
                debug!(
                    "AutoPilot: Still within active period for '{}', staying",
                    current_role
                );
                return Ok(None);
            }

            // Active period has elapsed, switch back to original
            debug!(
                "AutoPilot: Active period elapsed for '{}', switching back to original",
                current_role
            );
            if let Some(original) = &self.original_provider {
                let original_model = original.get_active_model_name();
                return Ok(Some((
                    Arc::clone(original),
                    "original".to_string(),
                    original_model,
                )));
            }
        }

        // Fallback: if we can't determine the state, switch back to original
        debug!("AutoPilot: Unable to determine current model state, switching back to original");
        if let Some(original) = &self.original_provider {
            let original_model = original.get_active_model_name();
            return Ok(Some((
                Arc::clone(original),
                "original".to_string(),
                original_model,
            )));
        }

        Ok(None)
    }

    /// Check if autopilot is currently in a switched state
    #[allow(dead_code)]
    pub fn is_switched(&self) -> bool {
        self.switch_active
    }

    /// Get the current role if switched
    #[allow(dead_code)]
    pub fn current_role(&self) -> Option<&str> {
        self.current_role.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;
    use rmcp::model::{Content, ErrorCode};
    use rmcp::ErrorData;
    use std::borrow::Cow;

    fn create_test_configs() -> Vec<CompleteModelConfig> {
        vec![
            CompleteModelConfig {
                provider: "openai".to_string(),
                model: "o1-preview".to_string(),
                role: "thinker".to_string(),
                rules: Rules {
                    triggers: TriggerRules {
                        keywords: vec!["think".to_string(), "analyze".to_string()],
                        match_type: MatchType::Any,
                        on_failure: false,
                        after_tool_use: false,
                        consecutive_tools: None,
                        consecutive_failures: None,
                        complexity_threshold: None,
                        source: TriggerSource::Human,
                        machine_messages_without_human: None,
                        tools_since_human: None,
                        messages_since_human: None,
                        first_turn: false,
                    },
                    active_turns: 0,
                    priority: 10,
                },
            },
            CompleteModelConfig {
                provider: "anthropic".to_string(),
                model: "claude-sonnet-4-20250514".to_string(),
                role: "helper".to_string(),
                rules: Rules {
                    triggers: TriggerRules {
                        keywords: vec!["help".to_string()],
                        match_type: MatchType::Any,
                        on_failure: true,
                        after_tool_use: false,
                        consecutive_tools: None,
                        consecutive_failures: None,
                        complexity_threshold: None,
                        source: TriggerSource::Any,
                        machine_messages_without_human: None,
                        tools_since_human: None,
                        messages_since_human: None,
                        first_turn: false,
                    },
                    active_turns: 5,
                    priority: 5,
                },
            },
            CompleteModelConfig {
                provider: "openai".to_string(),
                model: "gpt-4o".to_string(),
                role: "recovery".to_string(),
                rules: Rules {
                    triggers: TriggerRules {
                        keywords: vec![],
                        match_type: MatchType::Any,
                        on_failure: false,
                        after_tool_use: false,
                        consecutive_tools: None,
                        consecutive_failures: Some(2),
                        complexity_threshold: None,
                        source: TriggerSource::Machine,
                        machine_messages_without_human: None,
                        tools_since_human: None,
                        messages_since_human: None,
                        first_turn: false,
                    },
                    active_turns: 10,
                    priority: 20,
                },
            },
        ]
    }

    #[test]
    fn test_keyword_matching_any() {
        let keywords = vec!["think".to_string(), "analyze".to_string()];
        assert!(AutoPilot::check_keywords(
            "I need to think about this",
            &keywords,
            &MatchType::Any
        ));
        assert!(AutoPilot::check_keywords(
            "Please analyze the data",
            &keywords,
            &MatchType::Any
        ));
        assert!(!AutoPilot::check_keywords(
            "Just do it",
            &keywords,
            &MatchType::Any
        ));
    }

    #[test]
    fn test_complexity() {
        // Test <4 words rule
        assert!(matches!(
            AutoPilot::analyze_complexity("Hello"),
            ComplexityLevel::Low
        ));

        // Test complex text
        let complex_text = "I need help understanding this extremely complex distributed system architecture. \
                          How does the authentication and authorization flow work across multiple microservices? \
                          What are the security implications of our current design? Can you explain the database schema in detail? \
                          Also, I'm seeing various errors in the production logs and need to debug the API endpoints systematically. \
                          The performance seems significantly degraded and I'm wondering if we need to optimize the database queries. \
                          Additionally, there are concerns about scalability and high availability. \
                          Can you review the caching strategy and suggest improvements? \
                          We also need to consider the disaster recovery plan and backup procedures. \
                          What monitoring and alerting mechanisms should we implement? \
                          How can we ensure data consistency across services? \
                          Please provide detailed recommendations for each area.";

        assert!(matches!(
            AutoPilot::analyze_complexity(complex_text),
            ComplexityLevel::High
        ));
    }

    #[test]
    fn test_keyword_matching_all() {
        let keywords = vec!["think".to_string(), "analyze".to_string()];
        assert!(AutoPilot::check_keywords(
            "Think about and analyze this problem",
            &keywords,
            &MatchType::All
        ));
        assert!(!AutoPilot::check_keywords(
            "Just think about it",
            &keywords,
            &MatchType::All
        ));
    }

    #[test]
    fn test_complexity_analysis() {
        assert!(matches!(
            AutoPilot::analyze_complexity("Hello"),
            ComplexityLevel::Low
        ));
        assert!(matches!(
            AutoPilot::analyze_complexity("Yes please"),
            ComplexityLevel::Low
        ));
        assert!(matches!(
            AutoPilot::analyze_complexity("No thank you"),
            ComplexityLevel::Low
        ));

        // Medium complexity - 50+ words with questions
        let medium_text = "Can you help me understand how this complex system works? \
                          I need detailed information about the implementation. \
                          There are several components that interact with each other. \
                          What are the main design patterns used? \
                          How does the data flow through the system? \
                          Can you also explain the error handling approach?";
        assert!(matches!(
            AutoPilot::analyze_complexity(medium_text),
            ComplexityLevel::Medium
        ));

        // High complexity - Very long text with multiple questions
        let complex_text = "I need help understanding this extremely complex distributed system architecture. \
                          How does the authentication and authorization flow work across multiple microservices? \
                          What are the security implications of our current design? Can you explain the database schema in detail? \
                          Also, I'm seeing various errors in the production logs and need to debug the API endpoints systematically. \
                          The performance seems significantly degraded and I'm wondering if we need to optimize the database queries. \
                          Additionally, there are concerns about scalability and high availability. \
                          Can you review the caching strategy and suggest improvements? \
                          We also need to consider the disaster recovery plan and backup procedures. \
                          What monitoring and alerting mechanisms should we implement? \
                          How can we ensure data consistency across services? \
                          Please provide detailed recommendations for each area.";
        // This should definitely be high complexity with 100+ words and many questions
        let complexity = AutoPilot::analyze_complexity(complex_text);
        assert!(matches!(
            complexity,
            ComplexityLevel::High | ComplexityLevel::Medium
        ));
    }

    #[test]
    fn test_source_filtering() {
        let mut autopilot = AutoPilot {
            model_configs: create_test_configs(),
            model_states: HashMap::new(),
            original_provider: None,
            switch_active: false,
            current_role: None,
        };

        // Initialize states
        for model in &autopilot.model_configs {
            autopilot
                .model_states
                .insert(model.role.clone(), ModelState::default());
        }

        // Test human source - should trigger "thinker"
        let user_msg = Message::user().with_text("I need to think about this");
        let conversation = Conversation::new(vec![user_msg]).unwrap();

        let thinker_model = &autopilot.model_configs[0];
        assert!(autopilot.evaluate_rules(thinker_model, &conversation, 1));

        // Test machine source filtering
        // Human message as last - should NOT match Machine source filter
        let human_conversation =
            Conversation::new(vec![Message::user().with_text("test")]).unwrap();
        assert!(!autopilot.check_source(&human_conversation, &TriggerSource::Machine));

        // Assistant message as last - should match Machine source filter
        // Use new_unvalidated since a conversation ending with assistant is technically invalid
        let machine_conversation = Conversation::new_unvalidated(vec![
            Message::user().with_text("test"),
            Message::assistant().with_text("response"),
        ]);
        assert!(autopilot.check_source(&machine_conversation, &TriggerSource::Machine));
    }

    #[test]
    fn test_active_turns_mechanism() {
        let mut autopilot = AutoPilot {
            model_configs: create_test_configs(),
            model_states: HashMap::new(),
            original_provider: None,
            switch_active: false,
            current_role: None,
        };

        // Initialize states
        for model in &autopilot.model_configs {
            autopilot
                .model_states
                .insert(model.role.clone(), ModelState::default());
        }

        // Create a conversation with "help" keyword
        let message = Message::user().with_text("I need help");
        let conversation = Conversation::new(vec![message]).unwrap();

        // The helper model should trigger based on keyword matching
        let model = &autopilot.model_configs[1]; // helper model
        assert!(autopilot.evaluate_rules(model, &conversation, 6));

        // Test the active turns logic directly in should_switch_from_current
        autopilot.switch_active = true;
        autopilot.current_role = Some("helper".to_string());
        autopilot
            .model_states
            .get_mut("helper")
            .unwrap()
            .last_invoked_turn = Some(5);

        // At turn 6 (within active period of 5 turns), should stay
        // Since we don't have an original provider, it should return None (stay)
        let result = autopilot.should_switch_from_current(&conversation, 6);
        assert!(result.unwrap().is_none()); // Should stay with current model

        // At turn 11 (active period elapsed), should try to switch back but fail without provider
        let result = autopilot.should_switch_from_current(&conversation, 11);
        assert!(result.unwrap().is_none()); // No original provider, so can't switch back
    }

    #[test]
    fn test_consecutive_failures_trigger() {
        let autopilot = AutoPilot {
            model_configs: create_test_configs(),
            model_states: HashMap::new(),
            original_provider: None,
            switch_active: false,
            current_role: None,
        };

        // Create messages with consecutive failures
        // Simulate a pattern where we have tool responses that failed
        // The count_consecutive_failures function looks at tool responses in messages

        // Mock data - can't actually test this properly without real tool responses in the conversation
        // Since tool responses are part of the message content, not separate messages
        // This test would need a different approach or mock conversation

        // For now, just test the counting logic works with empty conversation
        let messages = vec![
            Message::user().with_text("do something"),
            Message::assistant().with_text("I'll try"),
        ];

        let conversation = Conversation::new_unvalidated(messages);

        // Should detect 0 failures in this simple conversation
        assert_eq!(autopilot.count_consecutive_failures(&conversation), 0);
    }

    #[test]
    fn test_premade_rules_loading() {
        // This tests that pre-made role rules can be loaded
        let premade = AutoPilot::load_premade_rules();
        assert!(!premade.is_empty());

        // Check that specific roles exist
        assert!(premade.contains_key("deep-thinker"));
        assert!(premade.contains_key("debugger"));
        assert!(premade.contains_key("coder"));
        assert!(premade.contains_key("second-opinion"));
    }

    #[test]
    fn test_config_merging() {
        let mut premade_rules = HashMap::new();
        premade_rules.insert(
            "helper".to_string(),
            Rules {
                triggers: TriggerRules::default(),
                active_turns: 5,
                priority: 5,
            },
        );

        // User config with custom rules
        let user_with_rules = vec![ModelConfig {
            provider: "anthropic".to_string(),
            model: "claude".to_string(),
            role: "helper".to_string(),
            rules: Some(Rules {
                triggers: TriggerRules::default(),
                active_turns: 3,
                priority: 10,
            }),
        }];

        let merged = AutoPilot::merge_configs(premade_rules.clone(), user_with_rules);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].provider, "anthropic");
        assert_eq!(merged[0].rules.priority, 10); // User rules override

        // User config without rules (inherit from premade)
        let user_without_rules = vec![ModelConfig {
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            role: "helper".to_string(),
            rules: None, // No rules, should inherit from premade
        }];

        let merged = AutoPilot::merge_configs(premade_rules, user_without_rules);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].provider, "openai");
        assert_eq!(merged[0].rules.priority, 5); // Inherited from premade
    }

    #[test]
    fn test_first_turn_trigger() {
        let mut autopilot = AutoPilot {
            model_configs: vec![
                CompleteModelConfig {
                    provider: "openai".to_string(),
                    model: "o1-preview".to_string(),
                    role: "lead".to_string(),
                    rules: Rules {
                        triggers: TriggerRules {
                            keywords: vec![],
                            match_type: MatchType::Any,
                            on_failure: false,
                            after_tool_use: false,
                            consecutive_tools: None,
                            consecutive_failures: Some(2),
                            complexity_threshold: None,
                            first_turn: true, // This should trigger on first turn
                            source: TriggerSource::Any,
                            machine_messages_without_human: None,
                            tools_since_human: None,
                            messages_since_human: None,
                        },
                        active_turns: 3,
                        priority: 30,
                    },
                },
                CompleteModelConfig {
                    provider: "anthropic".to_string(),
                    model: "claude-sonnet-4-20250514".to_string(),
                    role: "helper".to_string(),
                    rules: Rules {
                        triggers: TriggerRules {
                            keywords: vec!["help".to_string()],
                            match_type: MatchType::Any,
                            on_failure: false,
                            after_tool_use: false,
                            consecutive_tools: None,
                            consecutive_failures: None,
                            complexity_threshold: None,
                            first_turn: false, // This should NOT trigger on first turn
                            source: TriggerSource::Any,
                            machine_messages_without_human: None,
                            tools_since_human: None,
                            messages_since_human: None,
                        },
                        active_turns: 5,
                        priority: 5,
                    },
                },
            ],
            model_states: HashMap::new(),
            original_provider: None,
            switch_active: false,
            current_role: None,
        };

        // Initialize states
        for model in &autopilot.model_configs {
            autopilot
                .model_states
                .insert(model.role.clone(), ModelState::default());
        }

        // Test first turn - only "lead" role should trigger
        let first_message = Message::user().with_text("Hello, this is the first message");
        let conversation = Conversation::new(vec![first_message]).unwrap();

        let lead_model = &autopilot.model_configs[0]; // lead model
        let helper_model = &autopilot.model_configs[1]; // helper model

        // Lead model should trigger on first turn (current_turn = 1)
        assert!(autopilot.evaluate_rules(lead_model, &conversation, 1));

        // Helper model should NOT trigger on first turn (no first_turn: true and no "help" keyword)
        assert!(!autopilot.evaluate_rules(helper_model, &conversation, 1));

        // Test second turn - lead should NOT trigger on first_turn anymore
        let second_message = Message::user().with_text("This is the second message");
        let conversation_turn2 = Conversation::new(vec![
            Message::user().with_text("Hello, this is the first message"),
            Message::assistant().with_text("Hello! How can I help you?"),
            second_message,
        ])
        .unwrap();

        // Lead model should NOT trigger on second turn (current_turn = 2, first_turn only works on turn 1)
        assert!(!autopilot.evaluate_rules(lead_model, &conversation_turn2, 2));

        // Test that helper model can still trigger on keyword even on first turn
        let help_message = Message::user().with_text("I need help with something");
        let help_conversation = Conversation::new(vec![help_message]).unwrap();

        // Helper model should trigger on "help" keyword, even on first turn
        assert!(autopilot.evaluate_rules(helper_model, &help_conversation, 1));
    }

    #[test]
    fn test_tool_failure_detection() {
        let autopilot = AutoPilot {
            model_configs: create_test_configs(),
            model_states: HashMap::new(),
            original_provider: None,
            switch_active: false,
            current_role: None,
        };

        // Create a conversation with a tool failure
        let messages = vec![
            Message::user().with_text("test"),
            Message::user().with_tool_response(
                "test_tool",
                Err(ErrorData {
                    code: ErrorCode(-32000),
                    message: Cow::Borrowed("Tool execution failed"),
                    data: None,
                }),
            ),
            Message::assistant().with_text("The tool failed"),
        ];

        let conversation = Conversation::new_unvalidated(messages);
        assert!(autopilot.check_recent_failure(&conversation));

        // Test with successful tool response
        let success_messages = vec![
            Message::user().with_text("test"),
            Message::user().with_tool_response("test_tool", Ok(vec![Content::text("Success!")])),
            Message::assistant().with_text("The tool succeeded"),
        ];

        let success_conversation = Conversation::new_unvalidated(success_messages);
        assert!(!autopilot.check_recent_failure(&success_conversation));

        // Create a conversation without tool failures
        let messages = vec![
            Message::user().with_text("test"),
            Message::assistant().with_text("Let me help"),
        ];

        let conversation = Conversation::new_unvalidated(messages);
        // Should not detect any failures
        assert!(!autopilot.check_recent_failure(&conversation));
    }

    impl TriggerRules {
        fn default() -> Self {
            Self {
                keywords: vec![],
                match_type: MatchType::Any,
                on_failure: false,
                after_tool_use: false,
                consecutive_tools: None,
                consecutive_failures: None,
                machine_messages_without_human: None,
                tools_since_human: None,
                messages_since_human: None,
                complexity_threshold: None,
                first_turn: false,
                source: TriggerSource::Any,
            }
        }
    }
}
