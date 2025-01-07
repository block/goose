use std::collections::HashMap;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use super::base::{Pricing, Usage};

lazy_static::lazy_static! {
    static ref MODEL_PRICING: HashMap<String, Pricing> = {
        let mut m = HashMap::new();
        // Anthropic
        m.insert("claude-3-5-sonnet-latest".to_string(), Pricing {
            input_token_price: dec!(3),
            output_token_price: dec!(15),
        });
        m.insert("claude-3-5-sonnet-20241022".to_string(), Pricing {
            input_token_price: dec!(3),
            output_token_price: dec!(15),
        });
        m.insert("anthropic.claude-3-5-sonnet-20241022-v2:0".to_string(), Pricing {
            input_token_price: dec!(3),
            output_token_price: dec!(15),
        });
        m.insert("claude-3-5-sonnet-20241022-v2:0".to_string(), Pricing {
            input_token_price: dec!(3),
            output_token_price: dec!(15),
        });
        m.insert("claude-3-5-sonnet-v2@20241022".to_string(), Pricing {
            input_token_price: dec!(3),
            output_token_price: dec!(15),
        });
        m.insert("claude-3-5-haiku-latest".to_string(), Pricing {
            input_token_price: dec!(0.8),
            output_token_price: dec!(4),
        });
        m.insert("claude-3-5-haiku-20241022".to_string(), Pricing {
            input_token_price: dec!(0.8),
            output_token_price: dec!(4),
        });
        m.insert("anthropic.claude-3-5-haiku-20241022-v1:0".to_string(), Pricing {
            input_token_price: dec!(0.8),
            output_token_price: dec!(4),
        });
        m.insert("claude-3-5-haiku@20241022".to_string(), Pricing {
            input_token_price: dec!(0.8),
            output_token_price: dec!(4),
        });
        m.insert("claude-3-opus-latest".to_string(), Pricing {
            input_token_price: dec!(15.00),
            output_token_price: dec!(75.00),
        });
        m.insert("claude-3-opus-20240229".to_string(), Pricing {
            input_token_price: dec!(15.00),
            output_token_price: dec!(75.00),
        });
        m.insert("anthropic.claude-3-opus-20240229-v1:0".to_string(), Pricing {
            input_token_price: dec!(15.00),
            output_token_price: dec!(75.00),
        });
        m.insert("claude-3-opus@20240229".to_string(), Pricing {
            input_token_price: dec!(15.00),
            output_token_price: dec!(75.00),
        });
        // OpenAI
        m.insert("gpt-4o".to_string(), Pricing {
            input_token_price: dec!(2.50),
            output_token_price: dec!(10.00),
        });
        m.insert("gpt-4o-2024-11-20".to_string(), Pricing {
            input_token_price: dec!(2.50),
            output_token_price: dec!(10.00),
        });
        m.insert("gpt-4o-2024-08-06".to_string(), Pricing {
            input_token_price: dec!(2.50),
            output_token_price: dec!(10.00),
        });
        m.insert("gpt-4o-2024-05-13".to_string(), Pricing {
            input_token_price: dec!(5.00),
            output_token_price: dec!(15.00),
        });
        m.insert("gpt-4o-mini".to_string(), Pricing {
            input_token_price: dec!(0.150),
            output_token_price: dec!(0.600),
        });
        m.insert("gpt-4o-mini-2024-07-18".to_string(), Pricing {
            input_token_price: dec!(0.150),
            output_token_price: dec!(0.600),
        });
        m.insert("o1-preview".to_string(), Pricing {
            input_token_price: dec!(15.00),
            output_token_price: dec!(60.00),
        });
        m.insert("o1-preview-2024-09-12".to_string(), Pricing {
            input_token_price: dec!(15.00),
            output_token_price: dec!(60.00),
        });
        m.insert("o1-mini".to_string(), Pricing {
            input_token_price: dec!(3.00),
            output_token_price: dec!(12.00),
        });
        m.insert("o1-mini-2024-09-12".to_string(), Pricing {
            input_token_price: dec!(3.00),
            output_token_price: dec!(12.00),
        });
        m
    };
}

pub fn model_pricing_for(model: &str) -> Option<Pricing> {
    MODEL_PRICING.get(model).cloned()
}

pub fn cost(usage: &Usage, model_pricing: &Option<Pricing>) -> Option<Decimal> {
    if let Some(model_pricing) = model_pricing {
        let input_price = Decimal::from(usage.input_tokens.unwrap_or(0))
            * model_pricing.input_token_price
            / Decimal::from(1_000_000);
        let output_price = Decimal::from(usage.output_tokens.unwrap_or(0))
            * model_pricing.output_token_price
            / Decimal::from(1_000_000);
        Some(input_price + output_price)
    } else {
        None
    }
}
