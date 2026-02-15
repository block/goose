//! Delegation strategy for routing tasks to the appropriate execution backend.
//!
//! # Conceptual Model
//!
//! In Goose's architecture, there are three ways to delegate work:
//!
//! 1. **InProcessSpecialist** — Creates a temporary in-process Agent with custom
//!    instructions and extensions. This is conceptually equivalent to an "ephemeral
//!    mode" — the agent file's frontmatter defines instructions (system prompt),
//!    tool groups (extensions), and optionally a model override. The specialist runs
//!    in the same process, sharing the parent's provider (unless overridden).
//!
//! 2. **ExternalAcpAgent** — Spawns a separate process and connects via the
//!    Agent Communication Protocol (ACP) over stdio. The external agent has its
//!    own process, model, extensions, and lifecycle. Communication is via JSON-RPC.
//!
//! 3. **TemporaryModeActivation** (future) — For simple agent sources that only
//!    define instructions (no custom extensions, no model override), the parent
//!    agent could temporarily inject the agent's instructions into its own system
//!    prompt. This avoids creating a new Agent and session but requires the
//!    delegation layer to have access to the parent Agent (not currently possible
//!    from within an MCP extension).
//!
//! # Relationship to ACP SessionMode
//!
//! The ACP protocol supports `SessionMode` — behavioral modes with IDs, names,
//! and descriptions. When connecting to an external ACP agent, modes are advertised
//! during session creation and can be switched via `set_session_mode`.
//!
//! For in-process agents, the frontmatter `modes:` section maps to the same concept:
//! each mode has a slug (id), name, instructions, and tool_groups. The
//! `build_recipe_from_agent` function resolves the requested mode and builds a
//! Recipe with the mode's instructions.
//!
//! # Migration Path
//!
//! To fully unify specialists with modes:
//! 1. Move delegation out of the MCP extension layer into the Agent core
//! 2. Add `delegate_with_mode(mode_id, instructions)` to Agent
//! 3. For simple delegations: temporarily activate mode on parent agent
//! 4. For complex delegations: spawn specialist (current behavior)
//! 5. For external agents: use AgentClientManager (current behavior)

use crate::registry::manifest::AgentDistribution;

/// Strategy for executing a delegated task
#[derive(Debug, Clone)]
pub enum DelegationStrategy {
    /// Create a temporary in-process Agent with custom instructions and extensions.
    /// Conceptually equivalent to an "ephemeral mode" generated on the fly.
    InProcessSpecialist {
        /// Whether the agent defines custom extensions beyond the parent's
        has_custom_extensions: bool,
        /// Whether the agent overrides the model
        has_model_override: bool,
        /// Whether the agent defines multiple modes
        has_modes: bool,
    },

    /// Spawn an external process and communicate via ACP protocol.
    ExternalAcpAgent {
        /// Distribution information for spawning
        distribution: Box<AgentDistribution>,
    },
}

impl DelegationStrategy {
    /// Choose the appropriate strategy based on source characteristics
    pub fn choose(
        distribution: Option<&AgentDistribution>,
        has_custom_extensions: bool,
        has_model_override: bool,
        has_modes: bool,
    ) -> Self {
        if let Some(dist) = distribution {
            DelegationStrategy::ExternalAcpAgent {
                distribution: Box::new(dist.clone()),
            }
        } else {
            DelegationStrategy::InProcessSpecialist {
                has_custom_extensions,
                has_model_override,
                has_modes,
            }
        }
    }

    pub fn is_external(&self) -> bool {
        matches!(self, DelegationStrategy::ExternalAcpAgent { .. })
    }

    pub fn is_in_process(&self) -> bool {
        matches!(self, DelegationStrategy::InProcessSpecialist { .. })
    }
}

impl std::fmt::Display for DelegationStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DelegationStrategy::InProcessSpecialist {
                has_custom_extensions,
                has_model_override,
                has_modes,
            } => {
                write!(f, "InProcessSpecialist(")?;
                let mut parts = Vec::new();
                if *has_custom_extensions {
                    parts.push("custom_extensions");
                }
                if *has_model_override {
                    parts.push("model_override");
                }
                if *has_modes {
                    parts.push("multi_mode");
                }
                if parts.is_empty() {
                    write!(f, "simple")?;
                } else {
                    write!(f, "{}", parts.join(", "))?;
                }
                write!(f, ")")
            }
            DelegationStrategy::ExternalAcpAgent { .. } => {
                write!(f, "ExternalAcpAgent")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choose_external_when_distribution_present() {
        let dist = AgentDistribution::default();
        let strategy = DelegationStrategy::choose(Some(&dist), false, false, false);
        assert!(strategy.is_external());
    }

    #[test]
    fn choose_in_process_when_no_distribution() {
        let strategy = DelegationStrategy::choose(None, false, false, false);
        assert!(strategy.is_in_process());
    }

    #[test]
    fn choose_in_process_with_custom_extensions() {
        let strategy = DelegationStrategy::choose(None, true, true, true);
        assert!(strategy.is_in_process());
        assert_eq!(
            strategy.to_string(),
            "InProcessSpecialist(custom_extensions, model_override, multi_mode)"
        );
    }

    #[test]
    fn simple_specialist_display() {
        let strategy = DelegationStrategy::choose(None, false, false, false);
        assert_eq!(strategy.to_string(), "InProcessSpecialist(simple)");
    }

    #[test]
    fn external_display() {
        let dist = AgentDistribution::default();
        let strategy = DelegationStrategy::choose(Some(&dist), false, false, false);
        assert_eq!(strategy.to_string(), "ExternalAcpAgent");
    }
}
