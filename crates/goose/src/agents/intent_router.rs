use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument, Span};

use crate::agents::developer_agent::DeveloperAgent;
use crate::agents::goose_agent::GooseAgent;
use crate::agents::pm_agent::PmAgent;
use crate::agents::qa_agent::QaAgent;
use crate::agents::research_agent::ResearchAgent;
use crate::agents::security_agent::SecurityAgent;
use crate::agents::semantic_router::SemanticRouter;
use crate::registry::manifest::AgentMode;

/// Represents a routing decision: which agent + mode should handle this message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub agent_name: String,
    pub mode_slug: String,
    pub confidence: f32,
    pub reasoning: String,
}

/// A slot in the registry representing one available agent with its modes.
#[derive(Debug, Clone)]
pub struct AgentSlot {
    pub name: String,
    pub description: String,
    pub modes: Vec<AgentMode>,
    pub default_mode: String,
    pub enabled: bool,
    pub bound_extensions: Vec<String>,
}

/// Routes user messages to the best agent/mode combination.
///
/// Uses a three-tier strategy:
/// 1. Fast-path keyword matching against mode `when_to_use` hints (<10ms)
/// 2. TF-IDF semantic similarity against route descriptions (~1ms)
/// 3. Fallback: default agent in default mode
pub struct IntentRouter {
    slots: Vec<AgentSlot>,
    semantic: SemanticRouter,
    project_default_agent: Option<String>,
    project_default_mode: Option<String>,
}

impl Default for IntentRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl IntentRouter {
    pub fn new() -> Self {
        let mut slots = Vec::new();

        // Register GooseAgent
        let goose = GooseAgent::new();
        let goose_modes = goose.to_agent_modes();
        slots.push(AgentSlot {
            name: "Goose Agent".into(),
            description:
                "General-purpose AI assistant for conversations, planning, writing, reviewing documents, creating data visualizations and charts, and building interactive apps"
                    .into(),
            modes: goose_modes,
            default_mode: goose.default_mode_slug().into(),
            enabled: true,
            bound_extensions: vec![],
        });

        // Register DeveloperAgent (universal modes — replaces legacy CodingAgent)
        let dev = DeveloperAgent::new();
        let dev_modes = dev.to_agent_modes();
        slots.push(AgentSlot {
            name: "Developer Agent".into(),
            description: "Software engineer for coding, debugging, code review, architecture design, CI/CD pipelines, deployment, DevOps, backend, frontend, API endpoints, server infrastructure".into(),
            modes: dev_modes,
            default_mode: dev.default_mode().into(),
            enabled: true,
            bound_extensions: vec![],
        });

        // Register QaAgent
        let qa = QaAgent::new();
        let qa_modes = qa.to_agent_modes();
        slots.push(AgentSlot {
            name: "QA Agent".into(),
            description: "Quality assurance engineer for test strategy, test writing, code quality review, bug investigation, and debugging test failures".into(),
            modes: qa_modes,
            default_mode: qa.default_mode().into(),
            enabled: true,
            bound_extensions: vec![],
        });

        // Register PmAgent
        let pm = PmAgent::new();
        let pm_modes = pm.to_agent_modes();
        slots.push(AgentSlot {
            name: "PM Agent".into(),
            description: "Product manager for requirements gathering, user stories, PRDs, product requirements documents, roadmaps, release planning, prioritization frameworks like RICE MoSCoW, feature scoping, stakeholder communication, acceptance criteria, sprint planning, phased rollout strategy, competitive analysis from product perspective"
                .into(),
            modes: pm_modes,
            default_mode: pm.default_mode().into(),
            enabled: true,
            bound_extensions: vec![],
        });

        // Register SecurityAgent
        let security = SecurityAgent::new();
        let security_modes = security.to_agent_modes();
        slots.push(AgentSlot {
            name: "Security Agent".into(),
            description:
                "Security engineer for threat modeling, vulnerability analysis, security review, compliance auditing, and penetration testing".into(),
            modes: security_modes,
            default_mode: security.default_mode().into(),
            enabled: true,
            bound_extensions: vec![],
        });

        // Register ResearchAgent
        let research = ResearchAgent::new();
        let research_modes = research.to_agent_modes();
        slots.push(AgentSlot {
            name: "Research Agent".into(),
            description:
                "Research analyst for investigating topics, literature review, comparing technologies, benchmarking frameworks, fact-checking claims, explaining concepts like borrow checker or WebSocket, summarizing RFCs and technical reports, competitive analysis, state-of-the-art surveys, and documentation synthesis"
                    .into(),
            modes: research_modes,
            default_mode: research.default_mode().into(),
            enabled: true,
            bound_extensions: vec![],
        });

        Self {
            semantic: Self::build_semantic(&slots),
            slots,
            project_default_agent: None,
            project_default_mode: None,
        }
    }

    /// Build a SemanticRouter from current enabled slots and their non-internal modes.
    fn build_semantic(slots: &[AgentSlot]) -> SemanticRouter {
        let mut routes = Vec::new();
        for slot in slots.iter().filter(|s| s.enabled) {
            for mode in &slot.modes {
                if mode.is_internal {
                    continue;
                }
                // Combine description + when_to_use for a richer route corpus
                let mut corpus = format!("{} {}", slot.description, mode.description);
                if let Some(ref when) = mode.when_to_use {
                    corpus.push(' ');
                    corpus.push_str(when);
                }
                routes.push((slot.name.clone(), mode.slug.clone(), corpus));
            }
        }
        // Threshold 0.15: tuned to be above noise but below keyword threshold (0.2)
        SemanticRouter::new(routes, 0.15)
    }

    /// Rebuild the semantic router after slot changes.
    fn refresh_semantic(&mut self) {
        self.semantic = Self::build_semantic(&self.slots);
    }

    /// Apply per-project agent config overrides from `.goose/agents.yaml`.
    /// This modifies agent slots (enable/disable, descriptions, extensions, custom modes)
    /// and rebuilds the semantic router.
    pub fn apply_project_config(
        &mut self,
        config: &crate::agents::agent_config::ProjectAgentConfig,
    ) {
        crate::agents::agent_config::apply_project_config(config, &mut self.slots);

        // Override default agent/mode if specified
        if let Some(ref default_agent) = config.default_agent {
            // Store as a field we can check during routing fallback
            self.project_default_agent = Some(default_agent.clone());
        }
        if let Some(ref default_mode) = config.default_mode {
            self.project_default_mode = Some(default_mode.clone());
        }

        self.refresh_semantic();
    }

    pub fn set_enabled(&mut self, agent_name: &str, enabled: bool) {
        if let Some(slot) = self.slots.iter_mut().find(|s| s.name == agent_name) {
            slot.enabled = enabled;
        }
        self.refresh_semantic();
    }

    pub fn set_bound_extensions(&mut self, agent_name: &str, extensions: Vec<String>) {
        if let Some(slot) = self.slots.iter_mut().find(|s| s.name == agent_name) {
            slot.bound_extensions = extensions;
        }
    }

    pub fn add_slot(&mut self, slot: AgentSlot) {
        self.slots.push(slot);
        self.refresh_semantic();
    }

    pub fn remove_slot(&mut self, agent_name: &str) {
        self.slots.retain(|s| s.name != agent_name);
        self.refresh_semantic();
    }

    pub fn slots(&self) -> &[AgentSlot] {
        &self.slots
    }

    /// Route a user message to the best agent/mode.
    #[instrument(
        name = "intent_router.route",
        skip(self),
        fields(
            router.agent,
            router.mode,
            router.confidence,
            router.strategy = "keyword",
        )
    )]
    pub fn route(&self, user_message: &str) -> RoutingDecision {
        let span = Span::current();
        let message_lower = user_message.to_lowercase();
        let message_preview: String = user_message.chars().take(120).collect();

        let enabled_slots: Vec<&AgentSlot> = self.slots.iter().filter(|s| s.enabled).collect();

        if enabled_slots.is_empty() {
            let decision = self.fallback_decision("No agents enabled");
            span.record("router.agent", decision.agent_name.as_str());
            span.record("router.mode", decision.mode_slug.as_str());
            span.record("router.confidence", decision.confidence as f64);
            info!(
                agent = decision.agent_name,
                mode = decision.mode_slug,
                confidence = decision.confidence,
                reasoning = decision.reasoning.as_str(),
                message_preview = message_preview.as_str(),
                "routing.decision"
            );
            return decision;
        }

        // Score each mode against the message, with agent-level description bonus.
        // With universal modes, all agents share the same mode keywords.
        // The agent slot description provides the persona-level differentiation.
        let mut best: Option<(f32, &AgentSlot, &AgentMode)> = None;

        for slot in &enabled_slots {
            // Compute agent-level bonus from the slot description
            let agent_bonus = {
                let desc_keywords = Self::extract_keywords(&slot.description);
                let matched = desc_keywords
                    .iter()
                    .filter(|kw| {
                        Self::extract_keywords(&message_lower)
                            .iter()
                            .any(|mw| Self::words_match(mw, kw))
                    })
                    .count();
                if desc_keywords.is_empty() {
                    0.0f32
                } else {
                    (matched as f32 / desc_keywords.len() as f32) * 0.3
                        + (matched as f32).min(4.0) * 0.05
                }
            };

            for mode in &slot.modes {
                // Skip internal modes — they're for orchestrator use, not user routing
                if mode.is_internal {
                    continue;
                }
                let mode_score = self.score_mode_match(&message_lower, mode);
                let score = mode_score + agent_bonus;
                if score > 0.0 {
                    debug!(
                        agent = slot.name.as_str(),
                        mode = mode.slug.as_str(),
                        score = score,
                        agent_bonus = agent_bonus,
                        "routing.score"
                    );
                    if best.is_none() || score > best.as_ref().unwrap().0 {
                        best = Some((score, slot, mode));
                    }
                }
            }
        }

        let decision = if let Some((score, slot, mode)) = best {
            if score >= 0.2 {
                span.record("router.strategy", "keyword");
                RoutingDecision {
                    agent_name: slot.name.clone(),
                    mode_slug: mode.slug.clone(),
                    confidence: score.min(1.0),
                    reasoning: format!(
                        "[keyword] Matched mode '{}' (score: {:.2})",
                        mode.name, score
                    ),
                }
            } else {
                // Layer 2: keyword score too low → try TF-IDF semantic similarity
                self.try_semantic_or_default(user_message, &span, &enabled_slots, score)
            }
        } else {
            // No keyword matches at all → try semantic routing
            self.try_semantic_or_default(user_message, &span, &enabled_slots, 0.0)
        };

        span.record("router.agent", decision.agent_name.as_str());
        span.record("router.mode", decision.mode_slug.as_str());
        span.record("router.confidence", decision.confidence as f64);

        info!(
            agent = decision.agent_name.as_str(),
            mode = decision.mode_slug.as_str(),
            confidence = decision.confidence,
            reasoning = decision.reasoning.as_str(),
            message_preview = message_preview.as_str(),
            "routing.decision"
        );

        decision
    }

    /// Layer 2: Try TF-IDF semantic similarity when keyword score is below threshold.
    /// Falls back to default agent if semantic score is also too low.
    fn try_semantic_or_default(
        &self,
        user_message: &str,
        span: &Span,
        enabled_slots: &[&AgentSlot],
        keyword_score: f32,
    ) -> RoutingDecision {
        if let Some(hit) = self.semantic.route(user_message) {
            // Verify the matched agent is still enabled
            let agent_enabled = enabled_slots.iter().any(|s| s.name == hit.agent_name);
            if agent_enabled {
                span.record("router.strategy", "semantic");
                debug!(
                    agent = hit.agent_name.as_str(),
                    mode = hit.mode_slug.as_str(),
                    semantic_score = hit.similarity,
                    keyword_score = keyword_score,
                    top_terms = format!("{:?}", hit.top_terms).as_str(),
                    "routing.semantic_match"
                );
                return RoutingDecision {
                    agent_name: hit.agent_name,
                    mode_slug: hit.mode_slug,
                    confidence: hit.similarity.min(1.0),
                    reasoning: format!(
                        "[semantic] TF-IDF match (score: {:.3}, terms: {:?})",
                        hit.similarity, hit.top_terms
                    ),
                };
            }
        }

        // Layer 3: Fall back to project default or first enabled agent
        span.record("router.strategy", "default");

        // Use project-configured default agent if set and enabled
        if let Some(ref project_agent) = self.project_default_agent {
            if let Some(slot) = enabled_slots.iter().find(|s| &s.name == project_agent) {
                let mode = self
                    .project_default_mode
                    .as_deref()
                    .unwrap_or(&slot.default_mode);
                return RoutingDecision {
                    agent_name: slot.name.clone(),
                    mode_slug: mode.to_string(),
                    confidence: 0.5,
                    reasoning: format!(
                        "[project-default] keyword={:.2}, semantic=none; using project default",
                        keyword_score
                    ),
                };
            }
        }

        let default_slot = enabled_slots.first().unwrap();
        RoutingDecision {
            agent_name: default_slot.name.clone(),
            mode_slug: default_slot.default_mode.clone(),
            confidence: 0.5,
            reasoning: format!(
                "[default] keyword={:.2}, semantic=none; using default agent",
                keyword_score
            ),
        }
    }

    /// Score a mode against a message, returning the score and matched keywords.
    pub fn score_mode_detail(&self, message: &str, mode: &AgentMode) -> (f32, Vec<String>) {
        let message_lower = message.to_lowercase();
        let message_words = Self::extract_keywords(&message_lower);
        let mut matched = Vec::new();

        let mut score: f32 = 0.0;

        if let Some(ref when) = mode.when_to_use {
            let keywords = Self::extract_keywords(when);
            for kw in &keywords {
                if message_words.iter().any(|mw| Self::words_match(mw, kw)) {
                    matched.push(kw.clone());
                }
            }
            if !keywords.is_empty() {
                score += (matched.len() as f32 / keywords.len() as f32) * 0.6;
            }
        }

        let desc_keywords = Self::extract_keywords(&mode.description);
        let desc_matched: Vec<_> = desc_keywords
            .iter()
            .filter(|kw| message_words.iter().any(|mw| Self::words_match(mw, kw)))
            .cloned()
            .collect();
        if !desc_keywords.is_empty() {
            score += (desc_matched.len() as f32 / desc_keywords.len() as f32) * 0.3;
        }
        matched.extend(desc_matched);

        let name_clean = mode
            .name
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric() && c != ' ', "");
        let name_trimmed = name_clean.trim();
        if !name_trimmed.is_empty() && message_lower.contains(name_trimmed) {
            score += 0.1;
            matched.push(name_trimmed.to_string());
        }

        matched.sort();
        matched.dedup();
        (score, matched)
    }

    fn score_mode_match(&self, message_lower: &str, mode: &AgentMode) -> f32 {
        let mut score: f32 = 0.0;
        let message_words = Self::extract_keywords(message_lower);
        let mut total_matched: usize = 0;

        if let Some(ref when) = mode.when_to_use {
            let keywords = Self::extract_keywords(when);
            let matched = keywords
                .iter()
                .filter(|kw| message_words.iter().any(|mw| Self::words_match(mw, kw)))
                .count();
            total_matched += matched;
            if !keywords.is_empty() {
                score += (matched as f32 / keywords.len() as f32) * 0.5;
            }
        }

        let desc_keywords = Self::extract_keywords(&mode.description);
        let desc_matched = desc_keywords
            .iter()
            .filter(|kw| message_words.iter().any(|mw| Self::words_match(mw, kw)))
            .count();
        total_matched += desc_matched;
        if !desc_keywords.is_empty() {
            score += (desc_matched as f32 / desc_keywords.len() as f32) * 0.2;
        }

        let name_clean = mode
            .name
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric() && c != ' ', "");
        let name_trimmed = name_clean.trim();
        if !name_trimmed.is_empty() && message_lower.contains(name_trimmed) {
            score += 0.1;
            total_matched += 1;
        }

        // Absolute match bonus: more keyword hits → higher score
        // This prevents modes with few keywords from winning over modes with many matches
        score += (total_matched as f32).min(5.0) * 0.04;

        score
    }

    fn extract_keywords(text: &str) -> Vec<String> {
        let stop_words: std::collections::HashSet<&str> = [
            "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has",
            "had", "do", "does", "did", "will", "would", "could", "should", "may", "might",
            "shall", "can", "need", "to", "of", "in", "for", "on", "with", "at", "by", "from",
            "as", "into", "through", "during", "before", "after", "when", "where", "why", "how",
            "all", "each", "both", "few", "more", "most", "other", "some", "no", "not", "only",
            "own", "same", "so", "than", "too", "very", "just", "and", "or", "if", "but", "about",
            "up", "that", "this", "it",
        ]
        .into_iter()
        .collect();

        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 2 && !stop_words.contains(w))
            .map(String::from)
            .collect()
    }

    fn words_match(a: &str, b: &str) -> bool {
        if a == b {
            return true;
        }
        let shorter = a.len().min(b.len());
        let shared = a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count();
        // If the shorter word is a complete prefix of the longer, match
        if shared == shorter && shorter >= 3 {
            return true;
        }
        // Otherwise require a shared prefix of at least 4 covering most of the shorter word
        shared >= 4 && shared >= shorter.saturating_sub(2)
    }

    fn fallback_decision(&self, reason: &str) -> RoutingDecision {
        RoutingDecision {
            agent_name: "Goose Agent".into(),
            mode_slug: "ask".into(),
            confidence: 0.1,
            reasoning: reason.into(),
        }
    }
}

/// Build a routing prompt for future LLM-based classification.
pub fn build_routing_prompt(slots: &[AgentSlot], user_message: &str) -> String {
    let mut prompt = String::from(
        "You are a routing classifier. Given the user's message, decide which agent and mode should handle it.\n\n",
    );
    prompt.push_str("Available agents and modes:\n");
    for slot in slots {
        if !slot.enabled {
            continue;
        }
        prompt.push_str(&format!("\n## {} - {}\n", slot.name, slot.description));
        for mode in &slot.modes {
            prompt.push_str(&format!(
                "  - {} (slug: {}): {}",
                mode.name, mode.slug, mode.description
            ));
            if let Some(ref when) = mode.when_to_use {
                prompt.push_str(&format!(" [use when: {}]", when));
            }
            prompt.push('\n');
        }
    }
    prompt.push_str(&format!(
        "\nUser message: {}\n\nRespond with JSON: {{\"agent_name\": \"...\", \"mode_slug\": \"...\", \"confidence\": 0.0-1.0, \"reasoning\": \"...\"}}",
        user_message
    ));
    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_backend_coding() {
        let router = IntentRouter::new();
        let decision = router.route("implement a new backend API endpoint and write server code");
        // Should route to Developer Agent (write mode) for implementation tasks
        assert_eq!(decision.agent_name, "Developer Agent");
    }

    #[test]
    fn test_route_security() {
        let router = IntentRouter::new();
        let decision = router.route(
            "analyze security vulnerabilities and create a threat model for the auth system",
        );
        // Routes to dedicated Security Agent
        assert_eq!(decision.agent_name, "Security Agent");
    }

    #[test]
    fn test_route_general_conversation() {
        let router = IntentRouter::new();
        let decision = router.route("hello, how are you today?");
        assert_eq!(decision.agent_name, "Goose Agent");
    }

    #[test]
    fn test_disabled_agent_fallback() {
        let mut router = IntentRouter::new();
        router.set_enabled("Developer Agent", false);
        let decision = router.route("implement a REST API endpoint");
        // Falls back to Goose Agent when Developer Agent is disabled
        assert_ne!(decision.agent_name, "Developer Agent");
    }

    #[test]
    fn test_route_architecture() {
        let router = IntentRouter::new();
        let decision = router.route("design the system architecture and create an ADR");
        // Routes to Developer Agent (plan mode for architecture)
        assert_eq!(decision.agent_name, "Developer Agent");
    }

    #[test]
    fn test_route_qa_testing() {
        let router = IntentRouter::new();
        let decision =
            router.route("analyze test coverage gaps and review code quality in the auth module");
        // Routes to dedicated QA Agent
        assert_eq!(decision.agent_name, "QA Agent");
    }

    #[test]
    fn test_route_debugging() {
        let router = IntentRouter::new();
        let decision = router.route("debug this error, the server is crashing on startup");
        // Routes to Developer Agent (debug mode)
        assert_eq!(decision.agent_name, "Developer Agent");
    }

    #[test]
    fn test_route_devops() {
        let router = IntentRouter::new();
        let decision = router.route("set up the CI/CD pipeline and Dockerfile for deployment");
        // Routes to Developer Agent (write mode for devops)
        assert_eq!(decision.agent_name, "Developer Agent");
    }

    #[test]
    fn test_route_visual_dashboard_to_genui() {
        let router = IntentRouter::new();
        let decision = router.route("show a dashboard with charts summarizing session token usage");
        assert_eq!(decision.agent_name, "Goose Agent");
        assert_eq!(decision.mode_slug, "genui");
    }

    #[test]
    fn test_route_generate_chart_to_genui() {
        let router = IntentRouter::new();
        let decision = router.route("generate a chart visualization from random data");
        assert_eq!(decision.agent_name, "Goose Agent");
        assert_eq!(decision.mode_slug, "genui");
    }

    #[test]
    fn test_internal_modes_not_routable() {
        let router = IntentRouter::new();
        // "generating a recipe" should NOT route to recipe_maker (internal)
        let decision = router.route("generating a recipe yaml from this conversation");
        assert_ne!(
            decision.mode_slug, "recipe_maker",
            "Internal mode recipe_maker should not be routable"
        );
    }

    #[test]
    fn test_route_qa_dashboard_prefers_qa_agent() {
        let router = IntentRouter::new();
        let decision = router
            .route("show a dashboard of test coverage gaps and flaky tests in the auth module");
        assert_eq!(decision.agent_name, "QA Agent");
    }

    #[test]
    fn test_route_security_dashboard_prefers_security_agent() {
        let router = IntentRouter::new();
        let decision = router
            .route("create a dashboard of CVEs and security vulnerabilities for our dependencies");
        assert_eq!(decision.agent_name, "Security Agent");
    }

    #[test]
    fn test_semantic_layer_routes_research() {
        let router = IntentRouter::new();
        // This phrasing avoids direct keyword matches but has strong semantic overlap
        // with "investigating topics, comparing technologies, fact-checking"
        let decision =
            router.route("investigate and compare different database technologies for our project");
        assert_eq!(
            decision.agent_name, "Research Agent",
            "Semantic layer should catch research-oriented queries even without exact keyword hits"
        );
    }

    #[test]
    fn test_semantic_layer_falls_back_for_greetings() {
        let router = IntentRouter::new();
        let decision = router.route("hey there, what's up?");
        assert_eq!(decision.agent_name, "Goose Agent");
        // Should fall back to default — no keyword or semantic match for greetings
        assert!(
            decision.reasoning.contains("[default]") || decision.reasoning.contains("[keyword]"),
            "Greetings should not match via semantic layer: {}",
            decision.reasoning
        );
    }

    #[test]
    fn test_apply_project_config_disables_agent() {
        use crate::agents::agent_config::ProjectAgentConfig;
        let mut router = IntentRouter::new();
        let config: ProjectAgentConfig = serde_yaml::from_str(
            r#"
agents:
  "Developer Agent":
    enabled: false
"#,
        )
        .unwrap();
        router.apply_project_config(&config);
        let decision = router.route("implement a REST API endpoint");
        assert_ne!(
            decision.agent_name, "Developer Agent",
            "Disabled agent should not be routed to"
        );
    }

    #[test]
    fn test_apply_project_config_default_agent() {
        use crate::agents::agent_config::ProjectAgentConfig;
        let mut router = IntentRouter::new();
        let config: ProjectAgentConfig = serde_yaml::from_str(
            r#"
default_agent: "Developer Agent"
default_mode: "write"
"#,
        )
        .unwrap();
        router.apply_project_config(&config);
        // Generic greeting should fall back to project default (Developer Agent/write)
        let decision = router.route("hey there");
        assert_eq!(decision.agent_name, "Developer Agent");
        assert_eq!(decision.mode_slug, "write");
        assert!(decision.reasoning.contains("[project-default]"));
    }

    #[test]
    fn test_apply_project_config_custom_mode() {
        use crate::agents::agent_config::ProjectAgentConfig;
        let mut router = IntentRouter::new();
        let config: ProjectAgentConfig = serde_yaml::from_str(
            r#"
custom_modes:
  - slug: "data-pipeline"
    name: "Data Pipeline"
    description: "Build ETL data pipelines and transformations"
    when_to_use: "ETL data pipeline transformation orchestration airflow"
    agents: ["Developer Agent"]
    extensions: ["developer"]
    tool_groups: ["read", "edit"]
"#,
        )
        .unwrap();
        router.apply_project_config(&config);
        // Verify custom mode was added
        let dev_slot = router
            .slots()
            .iter()
            .find(|s| s.name == "Developer Agent")
            .unwrap();
        assert!(
            dev_slot.modes.iter().any(|m| m.slug == "data-pipeline"),
            "Custom mode should be added to Developer Agent"
        );
    }
}
