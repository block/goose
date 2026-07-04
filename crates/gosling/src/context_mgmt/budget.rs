/// Fraction of the context window reserved for a future retrieved-memory slot.
/// The slot is always empty in this MVP; the reservation just keeps room for
/// it so enabling memory later doesn't immediately blow the budget.
pub const DEFAULT_RETRIEVED_MEMORY_RESERVE_FRACTION: f64 = 0.10;

/// Extra safety margin held back from the provider's stated context limit to
/// absorb estimation error between our token counter and the provider's.
pub const DEFAULT_SAFETY_MARGIN_FRACTION: f64 = 0.05;

/// Fallback reserved-response budget when the model config has no configured
/// `max_tokens`.
pub const DEFAULT_RESERVED_RESPONSE_TOKENS: usize = 4_096;

/// Token budget policy for assembling a [`crate::context_mgmt::packet::ContextPacket`].
#[derive(Debug, Clone, Copy)]
pub struct ContextBudgetPolicy {
    pub context_limit: usize,
    pub reserved_response_tokens: usize,
    pub retrieved_memory_reserve_fraction: f64,
    pub safety_margin_fraction: f64,
}

impl ContextBudgetPolicy {
    pub fn new(context_limit: usize, reserved_response_tokens: usize) -> Self {
        Self {
            context_limit,
            reserved_response_tokens,
            retrieved_memory_reserve_fraction: DEFAULT_RETRIEVED_MEMORY_RESERVE_FRACTION,
            safety_margin_fraction: DEFAULT_SAFETY_MARGIN_FRACTION,
        }
    }

    pub fn retrieved_memory_reserved_tokens(&self) -> usize {
        (self.context_limit as f64 * self.retrieved_memory_reserve_fraction).round() as usize
    }

    pub fn safety_margin_tokens(&self) -> usize {
        (self.context_limit as f64 * self.safety_margin_fraction).round() as usize
    }

    /// Tokens left over for conversation + tool-result slots once the fixed
    /// costs (system prompt, project instructions, reserved response,
    /// reserved memory slot, safety margin) are subtracted.
    pub fn available_for_conversation(&self, fixed_slot_tokens: usize) -> usize {
        self.context_limit
            .saturating_sub(self.reserved_response_tokens)
            .saturating_sub(self.retrieved_memory_reserved_tokens())
            .saturating_sub(self.safety_margin_tokens())
            .saturating_sub(fixed_slot_tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserves_are_fractions_of_context_limit() {
        let policy = ContextBudgetPolicy::new(128_000, 8_000);
        assert_eq!(policy.retrieved_memory_reserved_tokens(), 12_800);
        assert_eq!(policy.safety_margin_tokens(), 6_400);
    }

    #[test]
    fn available_for_conversation_subtracts_all_fixed_costs() {
        let policy = ContextBudgetPolicy::new(128_000, 8_000);
        let fixed = 3_900; // system + project instructions
        let available = policy.available_for_conversation(fixed);
        // 128000 - 8000 - 12800 - 6400 - 3900
        assert_eq!(available, 96_900);
    }

    #[test]
    fn available_for_conversation_saturates_at_zero() {
        let policy = ContextBudgetPolicy::new(1_000, 2_000);
        assert_eq!(policy.available_for_conversation(0), 0);
    }
}
