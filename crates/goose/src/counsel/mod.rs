//! Counsel of 9 - A deliberative decision-making system
//!
//! This module implements the "Counsel of 9" feature, where 9 unique AI personas
//! with different personalities and expertise analyze a user's prompt, form opinions,
//! vote on the best response, and return the winning opinion.
//!
//! # Architecture
//!
//! The counsel process consists of three phases:
//!
//! 1. **Opinion Gathering**: Each of the 9 personas independently analyzes the user's
//!    prompt and provides their opinion based on their unique perspective.
//!
//! 2. **Voting**: Each persona reviews all opinions and votes for the best one
//!    (excluding their own).
//!
//! 3. **Result Compilation**: Votes are aggregated, a winner is determined, and
//!    the final result is compiled with all opinions and vote counts.
//!
//! # Example
//!
//! ```no_run
//! use goose::counsel::CounselOrchestrator;
//! use std::sync::Arc;
//!
//! # async fn example(provider: Arc<dyn goose::providers::base::Provider>) -> anyhow::Result<()> {
//! let orchestrator = CounselOrchestrator::new(provider);
//! let result = orchestrator.run("Should I use microservices or a monolith?").await?;
//!
//! println!("Winner: {}", result.winner.member_name);
//! println!("Opinion: {}", result.winner.content);
//! # Ok(())
//! # }
//! ```

pub mod orchestrator;
pub mod personas;
pub mod types;
pub mod voting;

// Re-export main types for convenience
pub use orchestrator::CounselOrchestrator;
pub use personas::get_all_personas;
pub use types::{CounselMember, CounselResult, CounselSession, Opinion, Vote, VoteResult};
