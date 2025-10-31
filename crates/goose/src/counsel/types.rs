use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Represents a single member of the counsel with their unique personality and expertise
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounselMember {
    /// Unique identifier for this counsel member
    pub id: String,
    /// Display name of the persona
    pub name: String,
    /// Detailed personality description
    pub personality: String,
    /// Core beliefs and values
    pub beliefs: Vec<String>,
    /// Areas of expertise
    pub expertise: Vec<String>,
    /// System prompt that defines how this persona thinks and responds
    pub system_prompt: String,
}

impl CounselMember {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        personality: impl Into<String>,
        beliefs: Vec<String>,
        expertise: Vec<String>,
        system_prompt: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            personality: personality.into(),
            beliefs,
            expertise,
            system_prompt: system_prompt.into(),
        }
    }
}

/// An opinion provided by a counsel member in response to the user's prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opinion {
    /// ID of the counsel member who provided this opinion
    pub member_id: String,
    /// Name of the counsel member
    pub member_name: String,
    /// The actual opinion content
    pub content: String,
    /// Reasoning behind the opinion
    pub reasoning: String,
}

impl Opinion {
    pub fn new(
        member_id: impl Into<String>,
        member_name: impl Into<String>,
        content: impl Into<String>,
        reasoning: impl Into<String>,
    ) -> Self {
        Self {
            member_id: member_id.into(),
            member_name: member_name.into(),
            content: content.into(),
            reasoning: reasoning.into(),
        }
    }
}

/// A vote cast by a counsel member for another member's opinion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    /// ID of the counsel member casting the vote
    pub voter_id: String,
    /// Name of the voter
    pub voter_name: String,
    /// ID of the counsel member being voted for
    pub voted_for_id: String,
    /// Name of the member being voted for
    pub voted_for_name: String,
    /// Optional reasoning for the vote
    pub reasoning: Option<String>,
}

impl Vote {
    pub fn new(
        voter_id: impl Into<String>,
        voter_name: impl Into<String>,
        voted_for_id: impl Into<String>,
        voted_for_name: impl Into<String>,
        reasoning: Option<String>,
    ) -> Self {
        Self {
            voter_id: voter_id.into(),
            voter_name: voter_name.into(),
            voted_for_id: voted_for_id.into(),
            voted_for_name: voted_for_name.into(),
            reasoning,
        }
    }
}

/// Result of the voting process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteResult {
    /// All votes cast
    pub votes: Vec<Vote>,
    /// The winning opinion
    pub winner: Opinion,
    /// Vote counts per member ID
    pub vote_counts: HashMap<String, u32>,
    /// Total number of votes cast
    pub total_votes: u32,
}

impl VoteResult {
    pub fn new(
        votes: Vec<Vote>,
        winner: Opinion,
        vote_counts: HashMap<String, u32>,
        total_votes: u32,
    ) -> Self {
        Self {
            votes,
            winner,
            vote_counts,
            total_votes,
        }
    }
}

/// A complete counsel session with all data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounselSession {
    /// Unique session ID
    pub id: String,
    /// The user's original prompt
    pub user_prompt: String,
    /// All opinions gathered
    pub opinions: Vec<Opinion>,
    /// All votes cast
    pub votes: Vec<Vote>,
    /// The final result (if voting completed)
    pub result: Option<VoteResult>,
    /// When this session was created
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl CounselSession {
    pub fn new(user_prompt: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_prompt: user_prompt.into(),
            opinions: Vec::new(),
            votes: Vec::new(),
            result: None,
            created_at: chrono::Utc::now(),
        }
    }

    pub fn with_opinions(mut self, opinions: Vec<Opinion>) -> Self {
        self.opinions = opinions;
        self
    }

    pub fn with_votes(mut self, votes: Vec<Vote>) -> Self {
        self.votes = votes;
        self
    }

    pub fn with_result(mut self, result: VoteResult) -> Self {
        self.result = Some(result);
        self
    }
}

/// The final result returned to the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounselResult {
    /// The winning opinion
    pub winner: Opinion,
    /// All opinions from all counsel members
    pub all_opinions: Vec<Opinion>,
    /// Vote counts per member ID
    pub vote_counts: HashMap<String, u32>,
    /// Total number of votes cast
    pub total_votes: u32,
    /// List of any members that failed to respond
    pub unavailable_members: Vec<String>,
}

impl CounselResult {
    pub fn new(
        winner: Opinion,
        all_opinions: Vec<Opinion>,
        vote_counts: HashMap<String, u32>,
        total_votes: u32,
        unavailable_members: Vec<String>,
    ) -> Self {
        Self {
            winner,
            all_opinions,
            vote_counts,
            total_votes,
            unavailable_members,
        }
    }

    /// Check if this is a valid result (minimum threshold met)
    pub fn is_valid(&self) -> bool {
        // Need at least 5 out of 9 members to have provided opinions
        self.all_opinions.len() >= 5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counsel_member_creation() {
        let member = CounselMember::new(
            "pragmatist",
            "The Pragmatist",
            "Focuses on practical solutions",
            vec!["Action over theory".to_string()],
            vec!["Implementation".to_string()],
            "You are a pragmatic thinker",
        );

        assert_eq!(member.id, "pragmatist");
        assert_eq!(member.name, "The Pragmatist");
        assert_eq!(member.beliefs.len(), 1);
    }

    #[test]
    fn test_opinion_creation() {
        let opinion = Opinion::new(
            "pragmatist",
            "The Pragmatist",
            "Start small and iterate",
            "This approach minimizes risk",
        );

        assert_eq!(opinion.member_id, "pragmatist");
        assert_eq!(opinion.member_name, "The Pragmatist");
    }

    #[test]
    fn test_vote_creation() {
        let vote = Vote::new(
            "pragmatist",
            "The Pragmatist",
            "visionary",
            "The Visionary",
            Some("Strong long-term thinking".to_string()),
        );

        assert_eq!(vote.voter_id, "pragmatist");
        assert_eq!(vote.voted_for_id, "visionary");
        assert!(vote.reasoning.is_some());
    }

    #[test]
    fn test_counsel_session_creation() {
        let session = CounselSession::new("Should I use Rust?");

        assert!(!session.id.is_empty());
        assert_eq!(session.user_prompt, "Should I use Rust?");
        assert!(session.opinions.is_empty());
        assert!(session.result.is_none());
    }

    #[test]
    fn test_counsel_session_builder() {
        let opinion = Opinion::new("pragmatist", "The Pragmatist", "Yes", "It's practical");
        let session = CounselSession::new("Test prompt").with_opinions(vec![opinion.clone()]);

        assert_eq!(session.opinions.len(), 1);
        assert_eq!(session.opinions[0].member_id, "pragmatist");
    }

    #[test]
    fn test_counsel_result_validity() {
        let opinions: Vec<Opinion> = (0..5)
            .map(|i| {
                Opinion::new(
                    format!("member_{}", i),
                    format!("Member {}", i),
                    "Opinion",
                    "Reasoning",
                )
            })
            .collect();

        let result = CounselResult::new(
            opinions[0].clone(),
            opinions.clone(),
            HashMap::new(),
            5,
            vec![],
        );

        assert!(result.is_valid());
    }

    #[test]
    fn test_counsel_result_invalid_threshold() {
        let opinions: Vec<Opinion> = (0..4)
            .map(|i| {
                Opinion::new(
                    format!("member_{}", i),
                    format!("Member {}", i),
                    "Opinion",
                    "Reasoning",
                )
            })
            .collect();

        let result = CounselResult::new(
            opinions[0].clone(),
            opinions.clone(),
            HashMap::new(),
            4,
            vec![],
        );

        assert!(!result.is_valid());
    }
}
