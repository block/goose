use std::ops::{Add, AddAssign};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderUsage {
    pub model: String,
    pub usage: Usage,
}

impl ProviderUsage {
    pub fn new(model: String, usage: Usage) -> Self {
        Self { model, usage }
    }

    /// Combine this ProviderUsage with another, adding their token counts
    /// Uses the model from this ProviderUsage
    pub fn combine_with(&self, other: &ProviderUsage) -> ProviderUsage {
        ProviderUsage {
            model: self.model.clone(),
            usage: self.usage + other.usage,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Copy)]
pub struct Usage {
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub cache_read_input_tokens: Option<i32>,
    pub cache_write_input_tokens: Option<i32>,
    /// Provider-reported cost for this usage, in USD. Populated only when the
    /// provider/gateway includes a `cost` field in its `usage` object (e.g.
    /// OpenRouter, or an Anthropic-compatible gateway). `None` => the cost is
    /// computed from the local pricing catalog instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<f64>,
}

fn sum_optionals<T>(a: Option<T>, b: Option<T>) -> Option<T>
where
    T: Add<Output = T> + Default,
{
    match (a, b) {
        (Some(x), Some(y)) => Some(x + y),
        (Some(x), None) => Some(x + T::default()),
        (None, Some(y)) => Some(T::default() + y),
        (None, None) => None,
    }
}

impl Add for Usage {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(
            sum_optionals(self.input_tokens, other.input_tokens),
            sum_optionals(self.output_tokens, other.output_tokens),
            sum_optionals(self.total_tokens, other.total_tokens),
        )
        .with_cache_tokens(
            sum_optionals(self.cache_read_input_tokens, other.cache_read_input_tokens),
            sum_optionals(
                self.cache_write_input_tokens,
                other.cache_write_input_tokens,
            ),
        )
        // Cost is a total for the usage that carries it (e.g. the final
        // message_delta), not an additive per-field count — take the latest
        // non-None value rather than summing, so merging message_start (no cost)
        // with message_delta (total cost) yields the total, not a doubled value.
        .with_cost(other.cost.or(self.cost))
    }
}

impl AddAssign for Usage {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Usage {
    pub fn new(
        input_tokens: Option<i32>,
        output_tokens: Option<i32>,
        total_tokens: Option<i32>,
    ) -> Self {
        let calculated_total = if total_tokens.is_none() {
            match (input_tokens, output_tokens) {
                (Some(input), Some(output)) => Some(input + output),
                (Some(input), None) => Some(input),
                (None, Some(output)) => Some(output),
                (None, None) => None,
            }
        } else {
            total_tokens
        };

        Self {
            input_tokens,
            output_tokens,
            total_tokens: calculated_total,
            cache_read_input_tokens: None,
            cache_write_input_tokens: None,
            cost: None,
        }
    }

    pub fn with_cache_tokens(
        mut self,
        cache_read_input_tokens: Option<i32>,
        cache_write_input_tokens: Option<i32>,
    ) -> Self {
        self.cache_read_input_tokens = cache_read_input_tokens;
        self.cache_write_input_tokens = cache_write_input_tokens;
        self
    }

    /// Attach a provider-reported cost (USD) for this usage. `None` leaves the
    /// cost to be computed from the local pricing catalog downstream.
    pub fn with_cost(mut self, cost: Option<f64>) -> Self {
        self.cost = cost;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use serde_json::json;

    #[test]
    fn test_usage_serialization() -> Result<()> {
        let usage = Usage::new(Some(10), Some(20), Some(30));
        let serialized = serde_json::to_string(&usage)?;
        let deserialized: Usage = serde_json::from_str(&serialized)?;

        assert_eq!(usage.input_tokens, deserialized.input_tokens);
        assert_eq!(usage.output_tokens, deserialized.output_tokens);
        assert_eq!(usage.total_tokens, deserialized.total_tokens);

        // Test JSON structure
        let json_value: serde_json::Value = serde_json::from_str(&serialized)?;
        assert_eq!(json_value["input_tokens"], json!(10));
        assert_eq!(json_value["output_tokens"], json!(20));
        assert_eq!(json_value["total_tokens"], json!(30));

        Ok(())
    }

    #[test]
    fn test_usage_addition_includes_cached_tokens() {
        let usage_a =
            Usage::new(Some(100), Some(20), Some(120)).with_cache_tokens(Some(10), Some(5));
        let usage_b = Usage::new(Some(50), Some(8), Some(58)).with_cache_tokens(Some(4), Some(1));

        let combined = usage_a + usage_b;

        assert_eq!(combined.input_tokens, Some(150));
        assert_eq!(combined.output_tokens, Some(28));
        assert_eq!(combined.total_tokens, Some(178));
        assert_eq!(combined.cache_read_input_tokens, Some(14));
        assert_eq!(combined.cache_write_input_tokens, Some(6));
        // No provider cost on either side -> None.
        assert_eq!(combined.cost, None);
    }

    #[test]
    fn test_provider_cost_field() {
        // Defaults to None and is omitted from JSON when absent.
        let bare = Usage::new(Some(10), Some(20), Some(30));
        assert_eq!(bare.cost, None);
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&bare).unwrap()).unwrap();
        assert!(json.get("cost").is_none());

        // Deserializes from a provider `usage` that includes `cost`.
        let with_cost: Usage = serde_json::from_value(
            json!({"input_tokens": 10, "output_tokens": 20, "cost": 0.0123}),
        )
        .unwrap();
        assert_eq!(with_cost.cost, Some(0.0123));
    }

    #[test]
    fn test_cost_merge_takes_latest_non_none_not_sum() {
        // message_start has no cost; the final message_delta carries the total.
        let start = Usage::new(Some(100), Some(0), Some(100));
        let delta = Usage::new(Some(0), Some(50), Some(50)).with_cost(Some(0.42));
        let merged = start + delta;
        // Total cost wins (not summed, not lost).
        assert_eq!(merged.cost, Some(0.42));

        // And when the accumulator already had a cost, the newer one replaces it.
        let next = Usage::new(Some(0), Some(10), Some(10)).with_cost(Some(0.10));
        assert_eq!((merged + next).cost, Some(0.10));
    }
}
