use super::types::{Opinion, Vote, VoteResult};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use tracing::{debug, warn};

/// Validate that votes are properly formed and follow the rules
pub fn validate_votes(votes: &[Vote], opinions: &[Opinion]) -> Result<()> {
    // Check that we have votes
    if votes.is_empty() {
        return Err(anyhow!("No votes were cast"));
    }

    // Create a set of valid member IDs from opinions
    let valid_member_ids: Vec<&str> = opinions.iter().map(|o| o.member_id.as_str()).collect();

    // Validate each vote
    for vote in votes {
        // Check that voter exists
        if !valid_member_ids.contains(&vote.voter_id.as_str()) {
            return Err(anyhow!(
                "Invalid voter ID: {} (voter: {})",
                vote.voter_id,
                vote.voter_name
            ));
        }

        // Check that voted-for member exists
        if !valid_member_ids.contains(&vote.voted_for_id.as_str()) {
            return Err(anyhow!(
                "Invalid voted-for ID: {} (voted for: {})",
                vote.voted_for_id,
                vote.voted_for_name
            ));
        }

        // Check for self-voting
        if vote.voter_id == vote.voted_for_id {
            warn!(
                "Self-vote detected: {} voted for themselves",
                vote.voter_name
            );
            return Err(anyhow!(
                "Self-voting is not allowed: {} voted for themselves",
                vote.voter_name
            ));
        }
    }

    debug!("All {} votes validated successfully", votes.len());
    Ok(())
}

/// Aggregate votes and count them per member
pub fn aggregate_votes(votes: &[Vote]) -> HashMap<String, u32> {
    let mut vote_counts: HashMap<String, u32> = HashMap::new();

    for vote in votes {
        *vote_counts.entry(vote.voted_for_id.clone()).or_insert(0) += 1;
    }

    debug!("Vote counts: {:?}", vote_counts);
    vote_counts
}

/// Determine the winner from vote counts
pub fn determine_winner(
    vote_counts: &HashMap<String, u32>,
    opinions: &[Opinion],
) -> Result<Opinion> {
    if vote_counts.is_empty() {
        return Err(anyhow!("No votes to count"));
    }

    // Find the maximum vote count
    let max_votes = vote_counts.values().max().copied().unwrap_or(0);

    if max_votes == 0 {
        return Err(anyhow!("No member received any votes"));
    }

    // Find all members with the maximum vote count (to detect ties)
    let winners: Vec<&String> = vote_counts
        .iter()
        .filter(|(_, &count)| count == max_votes)
        .map(|(id, _)| id)
        .collect();

    if winners.is_empty() {
        return Err(anyhow!("Could not determine winner"));
    }

    // Handle tie
    if winners.len() > 1 {
        warn!(
            "Tie detected: {} members have {} votes each",
            winners.len(),
            max_votes
        );
        // For now, select the first one (in future, we could do a runoff)
        // This is deterministic based on HashMap iteration order
        let winner_id = winners[0];
        debug!("Breaking tie by selecting first: {}", winner_id);

        return opinions
            .iter()
            .find(|o| &o.member_id == winner_id)
            .cloned()
            .ok_or_else(|| anyhow!("Winner opinion not found for ID: {}", winner_id));
    }

    // Single winner
    let winner_id = winners[0];
    debug!("Winner: {} with {} votes", winner_id, max_votes);

    opinions
        .iter()
        .find(|o| &o.member_id == winner_id)
        .cloned()
        .ok_or_else(|| anyhow!("Winner opinion not found for ID: {}", winner_id))
}

/// Process votes and create a VoteResult
pub fn process_votes(votes: Vec<Vote>, opinions: &[Opinion]) -> Result<VoteResult> {
    // Validate votes
    validate_votes(&votes, opinions)?;

    // Aggregate votes
    let vote_counts = aggregate_votes(&votes);

    // Determine winner
    let winner = determine_winner(&vote_counts, opinions)?;

    let total_votes = votes.len() as u32;

    Ok(VoteResult::new(votes, winner, vote_counts, total_votes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_opinions() -> Vec<Opinion> {
        vec![
            Opinion::new("member1", "Member 1", "Opinion 1", "Reasoning 1"),
            Opinion::new("member2", "Member 2", "Opinion 2", "Reasoning 2"),
            Opinion::new("member3", "Member 3", "Opinion 3", "Reasoning 3"),
        ]
    }

    #[test]
    fn test_validate_votes_success() {
        let opinions = create_test_opinions();
        let votes = vec![
            Vote::new("member1", "Member 1", "member2", "Member 2", None),
            Vote::new("member2", "Member 2", "member3", "Member 3", None),
            Vote::new("member3", "Member 3", "member1", "Member 1", None),
        ];

        assert!(validate_votes(&votes, &opinions).is_ok());
    }

    #[test]
    fn test_validate_votes_self_vote() {
        let opinions = create_test_opinions();
        let votes = vec![
            Vote::new("member1", "Member 1", "member1", "Member 1", None), // Self-vote
        ];

        let result = validate_votes(&votes, &opinions);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Self-voting"));
    }

    #[test]
    fn test_validate_votes_invalid_voter() {
        let opinions = create_test_opinions();
        let votes = vec![Vote::new("invalid", "Invalid", "member1", "Member 1", None)];

        let result = validate_votes(&votes, &opinions);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid voter ID"));
    }

    #[test]
    fn test_validate_votes_invalid_voted_for() {
        let opinions = create_test_opinions();
        let votes = vec![Vote::new("member1", "Member 1", "invalid", "Invalid", None)];

        let result = validate_votes(&votes, &opinions);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid voted-for ID"));
    }

    #[test]
    fn test_aggregate_votes() {
        let votes = vec![
            Vote::new("member1", "Member 1", "member2", "Member 2", None),
            Vote::new("member2", "Member 2", "member2", "Member 2", None),
            Vote::new("member3", "Member 3", "member1", "Member 1", None),
        ];

        let counts = aggregate_votes(&votes);

        assert_eq!(counts.get("member1"), Some(&1));
        assert_eq!(counts.get("member2"), Some(&2));
        assert_eq!(counts.get("member3"), None);
    }

    #[test]
    fn test_determine_winner_clear() {
        let opinions = create_test_opinions();
        let mut vote_counts = HashMap::new();
        vote_counts.insert("member1".to_string(), 1);
        vote_counts.insert("member2".to_string(), 3);
        vote_counts.insert("member3".to_string(), 2);

        let winner = determine_winner(&vote_counts, &opinions).unwrap();
        assert_eq!(winner.member_id, "member2");
    }

    #[test]
    fn test_determine_winner_tie() {
        let opinions = create_test_opinions();
        let mut vote_counts = HashMap::new();
        vote_counts.insert("member1".to_string(), 2);
        vote_counts.insert("member2".to_string(), 2);
        vote_counts.insert("member3".to_string(), 1);

        // Should handle tie (currently picks first, but should succeed)
        let result = determine_winner(&vote_counts, &opinions);
        assert!(result.is_ok());
        let winner = result.unwrap();
        // Winner should be either member1 or member2
        assert!(winner.member_id == "member1" || winner.member_id == "member2");
    }

    #[test]
    fn test_process_votes_success() {
        let opinions = create_test_opinions();
        let votes = vec![
            Vote::new("member1", "Member 1", "member2", "Member 2", None),
            Vote::new("member2", "Member 2", "member3", "Member 3", None),
            Vote::new("member3", "Member 3", "member2", "Member 2", None),
        ];

        let result = process_votes(votes, &opinions).unwrap();

        assert_eq!(result.winner.member_id, "member2");
        assert_eq!(result.total_votes, 3);
        assert_eq!(result.vote_counts.get("member2"), Some(&2));
    }

    #[test]
    fn test_process_votes_with_self_vote_fails() {
        let opinions = create_test_opinions();
        let votes = vec![
            Vote::new("member1", "Member 1", "member1", "Member 1", None), // Self-vote
            Vote::new("member2", "Member 2", "member1", "Member 1", None),
        ];

        let result = process_votes(votes, &opinions);
        assert!(result.is_err());
    }
}
