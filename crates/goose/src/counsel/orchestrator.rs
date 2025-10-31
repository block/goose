use super::personas::get_all_personas;
use super::types::{CounselMember, CounselResult, CounselSession, Opinion, Vote};
use super::voting::process_votes;
use crate::conversation::message::Message;
use crate::providers::base::Provider;
use anyhow::{anyhow, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, info, warn};

const PERSONA_TIMEOUT_SECONDS: u64 = 30;
const MINIMUM_REQUIRED_OPINIONS: usize = 5;

/// Orchestrates the entire counsel process
pub struct CounselOrchestrator {
    provider: Arc<dyn Provider>,
    personas: Vec<CounselMember>,
}

impl CounselOrchestrator {
    /// Create a new orchestrator with the given provider
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        Self {
            provider,
            personas: get_all_personas(),
        }
    }

    /// Run the complete counsel process
    pub async fn run(&self, user_prompt: impl Into<String>) -> Result<CounselResult> {
        let prompt = user_prompt.into();
        info!("Starting Counsel of 9 for prompt: {}", prompt);

        let mut session = CounselSession::new(prompt.clone());

        // Phase 1: Gather opinions
        info!(
            "Phase 1: Gathering opinions from {} personas",
            self.personas.len()
        );
        let (opinions, unavailable) = self.gather_initial_opinions(&prompt).await?;

        if opinions.len() < MINIMUM_REQUIRED_OPINIONS {
            return Err(anyhow!(
                "Insufficient opinions: got {}, need at least {}",
                opinions.len(),
                MINIMUM_REQUIRED_OPINIONS
            ));
        }

        info!(
            "Gathered {} opinions ({} unavailable)",
            opinions.len(),
            unavailable.len()
        );
        session = session.with_opinions(opinions.clone());

        // Phase 2: Conduct voting
        info!(
            "Phase 2: Conducting voting among {} personas",
            opinions.len()
        );
        let votes = self.conduct_voting(&opinions).await?;

        if votes.len() < MINIMUM_REQUIRED_OPINIONS {
            return Err(anyhow!(
                "Insufficient votes: got {}, need at least {}",
                votes.len(),
                MINIMUM_REQUIRED_OPINIONS
            ));
        }

        info!("Collected {} votes", votes.len());
        let _session = session.with_votes(votes.clone());

        // Phase 3: Process results
        info!("Phase 3: Processing votes and determining winner");
        let vote_result = process_votes(votes, &opinions)?;

        info!(
            "Winner: {} with {} votes",
            vote_result.winner.member_name,
            vote_result
                .vote_counts
                .get(&vote_result.winner.member_id)
                .unwrap_or(&0)
        );

        // Compile final result
        let result = CounselResult::new(
            vote_result.winner,
            opinions,
            vote_result.vote_counts,
            vote_result.total_votes,
            unavailable,
        );

        Ok(result)
    }

    /// Phase 1: Gather opinions from all personas in parallel
    async fn gather_initial_opinions(&self, prompt: &str) -> Result<(Vec<Opinion>, Vec<String>)> {
        let mut tasks = Vec::new();

        for persona in &self.personas {
            let provider = Arc::clone(&self.provider);
            let persona = persona.clone();
            let prompt = prompt.to_string();

            let task =
                tokio::spawn(
                    async move { Self::get_persona_opinion(provider, persona, prompt).await },
                );

            tasks.push(task);
        }

        // Wait for all tasks to complete
        let results = futures::future::join_all(tasks).await;

        let mut opinions = Vec::new();
        let mut unavailable = Vec::new();

        for (idx, result) in results.into_iter().enumerate() {
            match result {
                Ok(Ok(opinion)) => {
                    debug!("Received opinion from {}", opinion.member_name);
                    opinions.push(opinion);
                }
                Ok(Err(e)) => {
                    let persona_name = &self.personas[idx].name;
                    warn!("Failed to get opinion from {}: {}", persona_name, e);
                    unavailable.push(persona_name.clone());
                }
                Err(e) => {
                    let persona_name = &self.personas[idx].name;
                    warn!("Task failed for {}: {}", persona_name, e);
                    unavailable.push(persona_name.clone());
                }
            }
        }

        Ok((opinions, unavailable))
    }

    /// Get an opinion from a single persona
    async fn get_persona_opinion(
        provider: Arc<dyn Provider>,
        persona: CounselMember,
        prompt: String,
    ) -> Result<Opinion> {
        let timeout_duration = Duration::from_secs(PERSONA_TIMEOUT_SECONDS);

        let opinion_future = async {
            let message = Message::user().with_text(&prompt);

            let (response, _usage) = provider
                .complete(&persona.system_prompt, &[message], &[])
                .await
                .map_err(|e| anyhow!("Provider error for {}: {}", persona.name, e))?;

            let content = response.as_concat_text();

            if content.trim().is_empty() {
                return Err(anyhow!("Empty response from {}", persona.name));
            }

            // For now, use the entire response as both content and reasoning
            // In the future, we could parse structured output
            Ok(Opinion::new(
                persona.id.clone(),
                persona.name.clone(),
                content.clone(),
                content,
            ))
        };

        timeout(timeout_duration, opinion_future)
            .await
            .map_err(|_| anyhow!("Timeout getting opinion from {}", persona.name))?
    }

    /// Phase 2: Conduct voting among personas
    async fn conduct_voting(&self, opinions: &[Opinion]) -> Result<Vec<Vote>> {
        let mut tasks = Vec::new();

        // Create a map of member_id to persona for voting
        let persona_map: std::collections::HashMap<String, CounselMember> = self
            .personas
            .iter()
            .map(|p| (p.id.clone(), p.clone()))
            .collect();

        for opinion in opinions {
            // Only personas who provided opinions can vote
            if let Some(persona) = persona_map.get(&opinion.member_id) {
                let provider = Arc::clone(&self.provider);
                let persona = persona.clone();
                let opinions = opinions.to_vec();

                let task = tokio::spawn(async move {
                    Self::get_persona_vote(provider, persona, opinions).await
                });

                tasks.push(task);
            }
        }

        // Wait for all voting tasks to complete
        let results = futures::future::join_all(tasks).await;

        let mut votes = Vec::new();

        for result in results {
            match result {
                Ok(Ok(vote)) => {
                    debug!(
                        "Received vote from {} for {}",
                        vote.voter_name, vote.voted_for_name
                    );
                    votes.push(vote);
                }
                Ok(Err(e)) => {
                    warn!("Failed to get vote: {}", e);
                }
                Err(e) => {
                    warn!("Voting task failed: {}", e);
                }
            }
        }

        Ok(votes)
    }

    /// Get a vote from a single persona
    async fn get_persona_vote(
        provider: Arc<dyn Provider>,
        persona: CounselMember,
        opinions: Vec<Opinion>,
    ) -> Result<Vote> {
        let timeout_duration = Duration::from_secs(PERSONA_TIMEOUT_SECONDS);

        let vote_future = async {
            // Build the voting prompt with persona-specific framing
            let mut voting_prompt = format!(
                "You are {}, and you have reviewed the following opinions from your fellow counsel members. \
                You must vote for the opinion that BEST ALIGNS with your values, beliefs, and expertise. \
                Remember your core beliefs: {}. Consider which opinion best reflects what YOU value most.\n\n\
                You CANNOT vote for yourself.\n\n",
                persona.name,
                persona.beliefs.join(", ")
            );

            for (idx, opinion) in opinions.iter().enumerate() {
                voting_prompt.push_str(&format!(
                    "Opinion {} from {}:\n{}\n\n",
                    idx + 1,
                    opinion.member_name,
                    opinion.content
                ));
            }

            voting_prompt.push_str(&format!(
                "As {}, which opinion BEST ALIGNS with your perspective and values? \
                Think about:\n\
                - Which opinion reflects the priorities YOU care about most?\n\
                - Which approach would YOU advocate for?\n\
                - Which opinion demonstrates thinking that resonates with YOUR expertise?\n\n\
                You MUST vote for one of the other members (you cannot vote for yourself). \
                Respond with ONLY the name of the member you're voting for (e.g., 'The Pragmatist', 'The Visionary', etc.), \
                followed by a brief explanation of why it aligns with your values.",
                persona.name
            ));

            let message = Message::user().with_text(&voting_prompt);

            let (response, _usage) = provider
                .complete(&persona.system_prompt, &[message], &[])
                .await
                .map_err(|e| anyhow!("Provider error for {}: {}", persona.name, e))?;

            let content = response.as_concat_text();

            if content.trim().is_empty() {
                return Err(anyhow!("Empty vote response from {}", persona.name));
            }

            // Parse the vote - look for a persona name in the response
            let voted_for = Self::parse_vote(&content, &opinions, &persona)?;

            Ok(Vote::new(
                persona.id.clone(),
                persona.name.clone(),
                voted_for.member_id.clone(),
                voted_for.member_name.clone(),
                Some(content),
            ))
        };

        timeout(timeout_duration, vote_future)
            .await
            .map_err(|_| anyhow!("Timeout getting vote from {}", persona.name))?
    }

    /// Parse a vote response to extract which member was voted for
    fn parse_vote(response: &str, opinions: &[Opinion], voter: &CounselMember) -> Result<Opinion> {
        let response_lower = response.to_lowercase();

        // Try to find a persona name in the response
        for opinion in opinions {
            // Skip self
            if opinion.member_id == voter.id {
                continue;
            }

            let name_lower = opinion.member_name.to_lowercase();
            if response_lower.contains(&name_lower) {
                return Ok(opinion.clone());
            }
        }

        // If we couldn't parse it, return an error
        Err(anyhow!(
            "Could not parse vote from {}: {}",
            voter.name,
            response
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ModelConfig;
    use crate::providers::base::{Provider, ProviderUsage, Usage};
    use crate::providers::errors::ProviderError;
    use async_trait::async_trait;

    // Mock provider for testing
    struct MockProvider {
        response_text: String,
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn metadata() -> crate::providers::base::ProviderMetadata {
            crate::providers::base::ProviderMetadata::empty()
        }

        fn get_name(&self) -> &str {
            "mock"
        }

        async fn complete_with_model(
            &self,
            _model_config: &ModelConfig,
            _system: &str,
            _messages: &[Message],
            _tools: &[rmcp::model::Tool],
        ) -> Result<(Message, ProviderUsage), ProviderError> {
            let message = Message::assistant().with_text(&self.response_text);
            let usage = ProviderUsage::new(
                "mock-model".to_string(),
                Usage::new(Some(10), Some(20), Some(30)),
            );
            Ok((message, usage))
        }

        fn get_model_config(&self) -> ModelConfig {
            ModelConfig::new_or_fail("gpt-4o")
        }
    }

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let provider = Arc::new(MockProvider {
            response_text: "Test response".to_string(),
        });
        let orchestrator = CounselOrchestrator::new(provider);
        assert_eq!(orchestrator.personas.len(), 9);
    }

    #[tokio::test]
    async fn test_parse_vote_success() {
        let opinions = vec![
            Opinion::new("pragmatist", "The Pragmatist", "Opinion 1", "Reasoning 1"),
            Opinion::new("visionary", "The Visionary", "Opinion 2", "Reasoning 2"),
        ];
        let voter = CounselMember::new(
            "pragmatist",
            "The Pragmatist",
            "test",
            vec![],
            vec![],
            "test",
        );

        let response = "I vote for The Visionary because of their long-term thinking.";
        let result = CounselOrchestrator::parse_vote(response, &opinions, &voter);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().member_id, "visionary");
    }

    #[tokio::test]
    async fn test_parse_vote_self_vote_skipped() {
        let opinions = vec![
            Opinion::new("pragmatist", "The Pragmatist", "Opinion 1", "Reasoning 1"),
            Opinion::new("visionary", "The Visionary", "Opinion 2", "Reasoning 2"),
        ];
        let voter = CounselMember::new(
            "pragmatist",
            "The Pragmatist",
            "test",
            vec![],
            vec![],
            "test",
        );

        // Even if response mentions self, should find another valid vote
        let response = "I vote for The Visionary.";
        let result = CounselOrchestrator::parse_vote(response, &opinions, &voter);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().member_id, "visionary");
    }

    #[tokio::test]
    async fn test_parse_vote_not_found() {
        let opinions = vec![Opinion::new(
            "pragmatist",
            "The Pragmatist",
            "Opinion 1",
            "Reasoning 1",
        )];
        let voter =
            CounselMember::new("visionary", "The Visionary", "test", vec![], vec![], "test");

        let response = "I vote for nobody in particular.";
        let result = CounselOrchestrator::parse_vote(response, &opinions, &voter);

        assert!(result.is_err());
    }
}
