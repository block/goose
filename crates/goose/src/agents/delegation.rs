use crate::registry::manifest::AgentDistribution;

/// Determines how a task should be routed to another agent.
///
/// Three strategies:
///
/// 1. **InProcessSpecialist** — Spawn a temporary in-process agent with custom
///    instructions, extensions, and model. This is the default for agents that
///    are purely configuration-based (no separate binary).
///
/// 2. **ExternalAcpAgent** — Delegate to a separate process via ACP (Agent
///    Communication Protocol) JSON-RPC over stdio. Used for agents distributed
///    as standalone binaries.
///
/// 3. **RemoteA2AAgent** — Delegate to a remote agent over HTTP via A2A
///    (Agent-to-Agent) protocol. Used for agents hosted at a URL endpoint.
///
/// Future variants:
///   - `TemporaryModeActivation` — Activate a mode on the current agent session.
#[derive(Debug, Clone)]
pub enum DelegationStrategy {
    InProcessSpecialist {
        has_custom_extensions: bool,
        has_model_override: bool,
        has_modes: bool,
    },
    ExternalAcpAgent {
        distribution: Box<AgentDistribution>,
    },
    RemoteA2AAgent {
        url: String,
    },
}

impl DelegationStrategy {
    pub fn choose(
        distribution: Option<&AgentDistribution>,
        a2a_url: Option<&str>,
        has_custom_extensions: bool,
        has_model_override: bool,
        has_modes: bool,
    ) -> Self {
        // A2A remote takes priority if a URL is provided
        if let Some(url) = a2a_url {
            return DelegationStrategy::RemoteA2AAgent {
                url: url.to_string(),
            };
        }

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

    pub fn is_a2a(&self) -> bool {
        matches!(self, DelegationStrategy::RemoteA2AAgent { .. })
    }

    pub fn is_remote(&self) -> bool {
        self.is_external() || self.is_a2a()
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
            DelegationStrategy::RemoteA2AAgent { url } => {
                write!(f, "RemoteA2AAgent({})", url)
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
        let strategy = DelegationStrategy::choose(Some(&dist), None, false, false, false);
        assert!(strategy.is_external());
        assert!(strategy.is_remote());
    }

    #[test]
    fn choose_in_process_when_no_distribution() {
        let strategy = DelegationStrategy::choose(None, None, false, false, false);
        assert!(strategy.is_in_process());
        assert!(!strategy.is_remote());
    }

    #[test]
    fn choose_a2a_when_url_present() {
        let strategy = DelegationStrategy::choose(
            None,
            Some("https://agent.example.com"),
            false,
            false,
            false,
        );
        assert!(strategy.is_a2a());
        assert!(strategy.is_remote());
        assert!(!strategy.is_external());
        assert!(!strategy.is_in_process());
    }

    #[test]
    fn a2a_takes_priority_over_distribution() {
        let dist = AgentDistribution::default();
        let strategy = DelegationStrategy::choose(
            Some(&dist),
            Some("https://agent.example.com"),
            false,
            false,
            false,
        );
        assert!(strategy.is_a2a());
        assert!(!strategy.is_external());
    }

    #[test]
    fn choose_in_process_with_custom_extensions() {
        let strategy = DelegationStrategy::choose(None, None, true, true, true);
        assert!(strategy.is_in_process());
        assert_eq!(
            strategy.to_string(),
            "InProcessSpecialist(custom_extensions, model_override, multi_mode)"
        );
    }

    #[test]
    fn simple_specialist_display() {
        let strategy = DelegationStrategy::choose(None, None, false, false, false);
        assert_eq!(strategy.to_string(), "InProcessSpecialist(simple)");
    }

    #[test]
    fn external_display() {
        let dist = AgentDistribution::default();
        let strategy = DelegationStrategy::choose(Some(&dist), None, false, false, false);
        assert_eq!(strategy.to_string(), "ExternalAcpAgent");
    }

    #[test]
    fn a2a_display() {
        let strategy = DelegationStrategy::choose(
            None,
            Some("https://agent.example.com/a2a"),
            false,
            false,
            false,
        );
        assert_eq!(
            strategy.to_string(),
            "RemoteA2AAgent(https://agent.example.com/a2a)"
        );
    }
}
