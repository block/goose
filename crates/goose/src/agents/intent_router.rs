use serde::{Deserialize, Serialize};

use crate::agents::coding_agent::CodingAgent;
use crate::agents::goose_agent::GooseAgent;
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
/// Uses a two-tier strategy:
/// 1. Fast-path keyword matching against mode `when_to_use` hints
/// 2. Fallback: default agent in default mode
pub struct IntentRouter {
    slots: Vec<AgentSlot>,
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
                "General-purpose AI assistant for conversations, planning, and task execution"
                    .into(),
            modes: goose_modes,
            default_mode: goose.default_mode_slug().into(),
            enabled: true,
            bound_extensions: vec![],
        });

        // Register CodingAgent
        let coding = CodingAgent::new();
        let coding_modes = coding.to_agent_modes();
        slots.push(AgentSlot {
            name: "Coding Agent".into(),
            description: "Software development agent with SDLC-specialized modes".into(),
            modes: coding_modes,
            default_mode: coding.default_mode_slug().into(),
            enabled: true,
            bound_extensions: vec![],
        });

        Self { slots }
    }

    pub fn set_enabled(&mut self, agent_name: &str, enabled: bool) {
        if let Some(slot) = self.slots.iter_mut().find(|s| s.name == agent_name) {
            slot.enabled = enabled;
        }
    }

    pub fn set_bound_extensions(&mut self, agent_name: &str, extensions: Vec<String>) {
        if let Some(slot) = self.slots.iter_mut().find(|s| s.name == agent_name) {
            slot.bound_extensions = extensions;
        }
    }

    pub fn add_slot(&mut self, slot: AgentSlot) {
        self.slots.push(slot);
    }

    pub fn remove_slot(&mut self, agent_name: &str) {
        self.slots.retain(|s| s.name != agent_name);
    }

    pub fn slots(&self) -> &[AgentSlot] {
        &self.slots
    }

    /// Route a user message to the best agent/mode.
    pub fn route(&self, user_message: &str) -> RoutingDecision {
        let message_lower = user_message.to_lowercase();

        let enabled_slots: Vec<&AgentSlot> = self.slots.iter().filter(|s| s.enabled).collect();

        if enabled_slots.is_empty() {
            return self.fallback_decision("No agents enabled");
        }

        // Score each mode against the message
        let mut best: Option<(f32, &AgentSlot, &AgentMode)> = None;

        for slot in &enabled_slots {
            for mode in &slot.modes {
                let score = self.score_mode_match(&message_lower, mode);
                if score > 0.0 && (best.is_none() || score > best.as_ref().unwrap().0) {
                    best = Some((score, slot, mode));
                }
            }
        }

        if let Some((score, slot, mode)) = best {
            if score >= 0.2 {
                return RoutingDecision {
                    agent_name: slot.name.clone(),
                    mode_slug: mode.slug.clone(),
                    confidence: score.min(1.0),
                    reasoning: format!("Matched mode '{}' (score: {:.2})", mode.name, score),
                };
            }
        }

        let default_slot = enabled_slots.first().unwrap();
        RoutingDecision {
            agent_name: default_slot.name.clone(),
            mode_slug: default_slot.default_mode.clone(),
            confidence: 0.5,
            reasoning: "No strong mode match; using default agent".into(),
        }
    }

    fn score_mode_match(&self, message_lower: &str, mode: &AgentMode) -> f32 {
        let mut score: f32 = 0.0;
        let message_words = Self::extract_keywords(message_lower);

        if let Some(ref when) = mode.when_to_use {
            let keywords = Self::extract_keywords(when);
            let matched = keywords
                .iter()
                .filter(|kw| message_words.iter().any(|mw| Self::words_match(mw, kw)))
                .count();
            if !keywords.is_empty() {
                score += (matched as f32 / keywords.len() as f32) * 0.6;
            }
        }

        let desc_keywords = Self::extract_keywords(&mode.description);
        let desc_matched = desc_keywords
            .iter()
            .filter(|kw| message_words.iter().any(|mw| Self::words_match(mw, kw)))
            .count();
        if !desc_keywords.is_empty() {
            score += (desc_matched as f32 / desc_keywords.len() as f32) * 0.3;
        }

        let name_clean = mode
            .name
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric() && c != ' ', "");
        let name_trimmed = name_clean.trim();
        if !name_trimmed.is_empty() && message_lower.contains(name_trimmed) {
            score += 0.1;
        }

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
            mode_slug: "assistant".into(),
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
        let decision = router.route("implement a REST API endpoint for user authentication");
        assert_eq!(decision.agent_name, "Coding Agent");
    }

    #[test]
    fn test_route_security() {
        let router = IntentRouter::new();
        let decision =
            router.route("review this code for security vulnerabilities and threat modeling");
        assert_eq!(decision.agent_name, "Coding Agent");
        assert_eq!(decision.mode_slug, "security");
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
        router.set_enabled("Coding Agent", false);
        let decision = router.route("implement a REST API endpoint");
        assert_eq!(decision.agent_name, "Goose Agent");
    }

    #[test]
    fn test_route_architecture() {
        let router = IntentRouter::new();
        let decision = router.route("design the system architecture and create an ADR");
        assert_eq!(decision.agent_name, "Coding Agent");
        assert_eq!(decision.mode_slug, "architect");
    }

    #[test]
    fn test_route_qa_testing() {
        let router = IntentRouter::new();
        let decision = router.route("write tests and investigate bugs in the auth module");
        assert_eq!(decision.agent_name, "Coding Agent");
        assert_eq!(decision.mode_slug, "qa");
    }
}
