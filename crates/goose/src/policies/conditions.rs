//! Condition Types and Evaluation
//!
//! Implements 18+ condition types for rule evaluation:
//! - String conditions (contains, matches, equals)
//! - Numeric conditions (greater_than, less_than, between)
//! - Collection conditions (in_list, not_in_list, has_key)
//! - Temporal conditions (before, after, within_last)
//! - Logical conditions (and, or, not)

use super::errors::PolicyError;
use super::rule_engine::Event;
use chrono::{DateTime, Duration, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Condition types for rule evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    // =========================================================================
    // String Conditions
    // =========================================================================
    /// Check if field contains a substring
    Contains {
        field: String,
        value: String,
        #[serde(default)]
        case_sensitive: bool,
    },

    /// Check if field matches a regex pattern
    Matches {
        field: String,
        pattern: String,
    },

    /// Check if field equals a value
    Equals {
        field: String,
        value: Value,
    },

    /// Check if field starts with a prefix
    StartsWith {
        field: String,
        value: String,
        #[serde(default)]
        case_sensitive: bool,
    },

    /// Check if field ends with a suffix
    EndsWith {
        field: String,
        value: String,
        #[serde(default)]
        case_sensitive: bool,
    },

    /// Check if field is empty or null
    IsEmpty {
        field: String,
    },

    /// Check if field is not empty
    IsNotEmpty {
        field: String,
    },

    // =========================================================================
    // Numeric Conditions
    // =========================================================================
    /// Check if field is greater than a value
    GreaterThan {
        field: String,
        value: f64,
    },

    /// Check if field is greater than or equal to a value
    GreaterThanOrEqual {
        field: String,
        value: f64,
    },

    /// Check if field is less than a value
    LessThan {
        field: String,
        value: f64,
    },

    /// Check if field is less than or equal to a value
    LessThanOrEqual {
        field: String,
        value: f64,
    },

    /// Check if field is between two values (inclusive)
    Between {
        field: String,
        min: f64,
        max: f64,
    },

    // =========================================================================
    // Collection Conditions
    // =========================================================================
    /// Check if field value is in a list
    InList {
        field: String,
        values: Vec<Value>,
    },

    /// Check if field value is not in a list
    NotInList {
        field: String,
        values: Vec<Value>,
    },

    /// Check if field (object) has a specific key
    HasKey {
        field: String,
        key: String,
    },

    /// Check if field (array) has a specific length
    HasLength {
        field: String,
        length: usize,
    },

    /// Check if field (array) contains a value
    ArrayContains {
        field: String,
        value: Value,
    },

    // =========================================================================
    // Temporal Conditions
    // =========================================================================
    /// Check if timestamp is before a datetime
    Before {
        field: String,
        datetime: String,
    },

    /// Check if timestamp is after a datetime
    After {
        field: String,
        datetime: String,
    },

    /// Check if timestamp is within last N duration
    WithinLast {
        field: String,
        duration: String,
    },

    // =========================================================================
    // Logical Conditions
    // =========================================================================
    /// Logical AND of conditions
    And {
        conditions: Vec<Condition>,
    },

    /// Logical OR of conditions
    Or {
        conditions: Vec<Condition>,
    },

    /// Logical NOT of a condition
    Not {
        condition: Box<Condition>,
    },

    // =========================================================================
    // Special Conditions
    // =========================================================================
    /// Always true
    Always,

    /// Always false
    Never,

    /// Custom condition (for extensibility)
    Custom {
        name: String,
        #[serde(default)]
        params: HashMap<String, Value>,
    },
}

/// Context for condition evaluation
#[derive(Debug, Clone)]
pub struct ConditionContext {
    /// The event being evaluated
    pub event: Event,
}

/// Result of condition evaluation
#[derive(Debug, Clone)]
pub struct ConditionResult {
    /// Whether the condition matched
    pub matched: bool,
    /// Evidence for the match
    pub evidence: Vec<String>,
}

impl ConditionResult {
    /// Create a matched result
    pub fn matched(evidence: impl Into<String>) -> Self {
        Self {
            matched: true,
            evidence: vec![evidence.into()],
        }
    }

    /// Create a non-matched result
    pub fn not_matched() -> Self {
        Self {
            matched: false,
            evidence: vec![],
        }
    }
}

/// Condition evaluator
pub struct ConditionEvaluator {
    /// Compiled regex cache
    regex_cache: std::sync::RwLock<HashMap<String, Regex>>,
}

impl ConditionEvaluator {
    /// Create a new evaluator
    pub fn new() -> Self {
        Self {
            regex_cache: std::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Evaluate a condition against a context
    pub async fn evaluate(
        &self,
        condition: &Condition,
        context: &ConditionContext,
    ) -> Result<ConditionResult, PolicyError> {
        match condition {
            // String conditions
            Condition::Contains {
                field,
                value,
                case_sensitive,
            } => self.eval_contains(field, value, *case_sensitive, context),

            Condition::Matches { field, pattern } => self.eval_matches(field, pattern, context),

            Condition::Equals { field, value } => self.eval_equals(field, value, context),

            Condition::StartsWith {
                field,
                value,
                case_sensitive,
            } => self.eval_starts_with(field, value, *case_sensitive, context),

            Condition::EndsWith {
                field,
                value,
                case_sensitive,
            } => self.eval_ends_with(field, value, *case_sensitive, context),

            Condition::IsEmpty { field } => self.eval_is_empty(field, context),

            Condition::IsNotEmpty { field } => self.eval_is_not_empty(field, context),

            // Numeric conditions
            Condition::GreaterThan { field, value } => {
                self.eval_greater_than(field, *value, context)
            }

            Condition::GreaterThanOrEqual { field, value } => {
                self.eval_greater_than_or_equal(field, *value, context)
            }

            Condition::LessThan { field, value } => self.eval_less_than(field, *value, context),

            Condition::LessThanOrEqual { field, value } => {
                self.eval_less_than_or_equal(field, *value, context)
            }

            Condition::Between { field, min, max } => {
                self.eval_between(field, *min, *max, context)
            }

            // Collection conditions
            Condition::InList { field, values } => self.eval_in_list(field, values, context),

            Condition::NotInList { field, values } => self.eval_not_in_list(field, values, context),

            Condition::HasKey { field, key } => self.eval_has_key(field, key, context),

            Condition::HasLength { field, length } => self.eval_has_length(field, *length, context),

            Condition::ArrayContains { field, value } => {
                self.eval_array_contains(field, value, context)
            }

            // Temporal conditions
            Condition::Before { field, datetime } => self.eval_before(field, datetime, context),

            Condition::After { field, datetime } => self.eval_after(field, datetime, context),

            Condition::WithinLast { field, duration } => {
                self.eval_within_last(field, duration, context)
            }

            // Logical conditions
            Condition::And { conditions } => self.eval_and(conditions, context).await,

            Condition::Or { conditions } => self.eval_or(conditions, context).await,

            Condition::Not { condition } => self.eval_not(condition, context).await,

            // Special conditions
            Condition::Always => Ok(ConditionResult::matched("Always condition")),

            Condition::Never => Ok(ConditionResult::not_matched()),

            Condition::Custom { name, params } => self.eval_custom(name, params, context),
        }
    }

    // =========================================================================
    // String condition implementations
    // =========================================================================

    fn eval_contains(
        &self,
        field: &str,
        value: &str,
        case_sensitive: bool,
        context: &ConditionContext,
    ) -> Result<ConditionResult, PolicyError> {
        let field_value = self.get_string_field(field, context)?;

        let matches = if case_sensitive {
            field_value.contains(value)
        } else {
            field_value.to_lowercase().contains(&value.to_lowercase())
        };

        if matches {
            Ok(ConditionResult::matched(format!(
                "Field '{}' contains '{}'",
                field, value
            )))
        } else {
            Ok(ConditionResult::not_matched())
        }
    }

    fn eval_matches(
        &self,
        field: &str,
        pattern: &str,
        context: &ConditionContext,
    ) -> Result<ConditionResult, PolicyError> {
        let field_value = self.get_string_field(field, context)?;
        let regex = self.get_or_compile_regex(pattern)?;

        if regex.is_match(&field_value) {
            Ok(ConditionResult::matched(format!(
                "Field '{}' matches pattern '{}'",
                field, pattern
            )))
        } else {
            Ok(ConditionResult::not_matched())
        }
    }

    fn eval_equals(
        &self,
        field: &str,
        value: &Value,
        context: &ConditionContext,
    ) -> Result<ConditionResult, PolicyError> {
        let field_value = context.event.get_field(field);

        match field_value {
            Some(fv) if fv == value => Ok(ConditionResult::matched(format!(
                "Field '{}' equals {:?}",
                field, value
            ))),
            Some(_) => Ok(ConditionResult::not_matched()),
            None => Ok(ConditionResult::not_matched()),
        }
    }

    fn eval_starts_with(
        &self,
        field: &str,
        value: &str,
        case_sensitive: bool,
        context: &ConditionContext,
    ) -> Result<ConditionResult, PolicyError> {
        let field_value = self.get_string_field(field, context)?;

        let matches = if case_sensitive {
            field_value.starts_with(value)
        } else {
            field_value.to_lowercase().starts_with(&value.to_lowercase())
        };

        if matches {
            Ok(ConditionResult::matched(format!(
                "Field '{}' starts with '{}'",
                field, value
            )))
        } else {
            Ok(ConditionResult::not_matched())
        }
    }

    fn eval_ends_with(
        &self,
        field: &str,
        value: &str,
        case_sensitive: bool,
        context: &ConditionContext,
    ) -> Result<ConditionResult, PolicyError> {
        let field_value = self.get_string_field(field, context)?;

        let matches = if case_sensitive {
            field_value.ends_with(value)
        } else {
            field_value.to_lowercase().ends_with(&value.to_lowercase())
        };

        if matches {
            Ok(ConditionResult::matched(format!(
                "Field '{}' ends with '{}'",
                field, value
            )))
        } else {
            Ok(ConditionResult::not_matched())
        }
    }

    fn eval_is_empty(&self, field: &str, context: &ConditionContext) -> Result<ConditionResult, PolicyError> {
        let field_value = context.event.get_field(field);

        let is_empty = match field_value {
            None => true,
            Some(Value::Null) => true,
            Some(Value::String(s)) => s.is_empty(),
            Some(Value::Array(arr)) => arr.is_empty(),
            Some(Value::Object(obj)) => obj.is_empty(),
            _ => false,
        };

        if is_empty {
            Ok(ConditionResult::matched(format!("Field '{}' is empty", field)))
        } else {
            Ok(ConditionResult::not_matched())
        }
    }

    fn eval_is_not_empty(&self, field: &str, context: &ConditionContext) -> Result<ConditionResult, PolicyError> {
        let result = self.eval_is_empty(field, context)?;
        if result.matched {
            Ok(ConditionResult::not_matched())
        } else {
            Ok(ConditionResult::matched(format!("Field '{}' is not empty", field)))
        }
    }

    // =========================================================================
    // Numeric condition implementations
    // =========================================================================

    fn eval_greater_than(
        &self,
        field: &str,
        value: f64,
        context: &ConditionContext,
    ) -> Result<ConditionResult, PolicyError> {
        let field_value = self.get_numeric_field(field, context)?;

        if field_value > value {
            Ok(ConditionResult::matched(format!(
                "Field '{}' ({}) > {}",
                field, field_value, value
            )))
        } else {
            Ok(ConditionResult::not_matched())
        }
    }

    fn eval_greater_than_or_equal(
        &self,
        field: &str,
        value: f64,
        context: &ConditionContext,
    ) -> Result<ConditionResult, PolicyError> {
        let field_value = self.get_numeric_field(field, context)?;

        if field_value >= value {
            Ok(ConditionResult::matched(format!(
                "Field '{}' ({}) >= {}",
                field, field_value, value
            )))
        } else {
            Ok(ConditionResult::not_matched())
        }
    }

    fn eval_less_than(
        &self,
        field: &str,
        value: f64,
        context: &ConditionContext,
    ) -> Result<ConditionResult, PolicyError> {
        let field_value = self.get_numeric_field(field, context)?;

        if field_value < value {
            Ok(ConditionResult::matched(format!(
                "Field '{}' ({}) < {}",
                field, field_value, value
            )))
        } else {
            Ok(ConditionResult::not_matched())
        }
    }

    fn eval_less_than_or_equal(
        &self,
        field: &str,
        value: f64,
        context: &ConditionContext,
    ) -> Result<ConditionResult, PolicyError> {
        let field_value = self.get_numeric_field(field, context)?;

        if field_value <= value {
            Ok(ConditionResult::matched(format!(
                "Field '{}' ({}) <= {}",
                field, field_value, value
            )))
        } else {
            Ok(ConditionResult::not_matched())
        }
    }

    fn eval_between(
        &self,
        field: &str,
        min: f64,
        max: f64,
        context: &ConditionContext,
    ) -> Result<ConditionResult, PolicyError> {
        let field_value = self.get_numeric_field(field, context)?;

        if field_value >= min && field_value <= max {
            Ok(ConditionResult::matched(format!(
                "Field '{}' ({}) is between {} and {}",
                field, field_value, min, max
            )))
        } else {
            Ok(ConditionResult::not_matched())
        }
    }

    // =========================================================================
    // Collection condition implementations
    // =========================================================================

    fn eval_in_list(
        &self,
        field: &str,
        values: &[Value],
        context: &ConditionContext,
    ) -> Result<ConditionResult, PolicyError> {
        let field_value = context
            .event
            .get_field(field)
            .ok_or_else(|| PolicyError::FieldNotFound {
                field: field.to_string(),
            })?;

        if values.contains(field_value) {
            Ok(ConditionResult::matched(format!(
                "Field '{}' is in list",
                field
            )))
        } else {
            Ok(ConditionResult::not_matched())
        }
    }

    fn eval_not_in_list(
        &self,
        field: &str,
        values: &[Value],
        context: &ConditionContext,
    ) -> Result<ConditionResult, PolicyError> {
        let result = self.eval_in_list(field, values, context)?;
        if result.matched {
            Ok(ConditionResult::not_matched())
        } else {
            Ok(ConditionResult::matched(format!(
                "Field '{}' is not in list",
                field
            )))
        }
    }

    fn eval_has_key(
        &self,
        field: &str,
        key: &str,
        context: &ConditionContext,
    ) -> Result<ConditionResult, PolicyError> {
        let field_value = context.event.get_field(field);

        match field_value {
            Some(Value::Object(obj)) if obj.contains_key(key) => Ok(ConditionResult::matched(
                format!("Field '{}' has key '{}'", field, key),
            )),
            _ => Ok(ConditionResult::not_matched()),
        }
    }

    fn eval_has_length(
        &self,
        field: &str,
        length: usize,
        context: &ConditionContext,
    ) -> Result<ConditionResult, PolicyError> {
        let field_value = context.event.get_field(field);

        let actual_length = match field_value {
            Some(Value::Array(arr)) => arr.len(),
            Some(Value::String(s)) => s.len(),
            _ => return Ok(ConditionResult::not_matched()),
        };

        if actual_length == length {
            Ok(ConditionResult::matched(format!(
                "Field '{}' has length {}",
                field, length
            )))
        } else {
            Ok(ConditionResult::not_matched())
        }
    }

    fn eval_array_contains(
        &self,
        field: &str,
        value: &Value,
        context: &ConditionContext,
    ) -> Result<ConditionResult, PolicyError> {
        let field_value = context.event.get_field(field);

        match field_value {
            Some(Value::Array(arr)) if arr.contains(value) => Ok(ConditionResult::matched(
                format!("Field '{}' contains {:?}", field, value),
            )),
            _ => Ok(ConditionResult::not_matched()),
        }
    }

    // =========================================================================
    // Temporal condition implementations
    // =========================================================================

    fn eval_before(
        &self,
        field: &str,
        datetime: &str,
        context: &ConditionContext,
    ) -> Result<ConditionResult, PolicyError> {
        let field_value = self.get_datetime_field(field, context)?;
        let target = self.parse_datetime(datetime)?;

        if field_value < target {
            Ok(ConditionResult::matched(format!(
                "Field '{}' is before {}",
                field, datetime
            )))
        } else {
            Ok(ConditionResult::not_matched())
        }
    }

    fn eval_after(
        &self,
        field: &str,
        datetime: &str,
        context: &ConditionContext,
    ) -> Result<ConditionResult, PolicyError> {
        let field_value = self.get_datetime_field(field, context)?;
        let target = self.parse_datetime(datetime)?;

        if field_value > target {
            Ok(ConditionResult::matched(format!(
                "Field '{}' is after {}",
                field, datetime
            )))
        } else {
            Ok(ConditionResult::not_matched())
        }
    }

    fn eval_within_last(
        &self,
        field: &str,
        duration_str: &str,
        context: &ConditionContext,
    ) -> Result<ConditionResult, PolicyError> {
        let field_value = self.get_datetime_field(field, context)?;
        let duration = self.parse_duration(duration_str)?;
        let threshold = Utc::now() - duration;

        if field_value >= threshold {
            Ok(ConditionResult::matched(format!(
                "Field '{}' is within last {}",
                field, duration_str
            )))
        } else {
            Ok(ConditionResult::not_matched())
        }
    }

    // =========================================================================
    // Logical condition implementations
    // =========================================================================

    async fn eval_and(
        &self,
        conditions: &[Condition],
        context: &ConditionContext,
    ) -> Result<ConditionResult, PolicyError> {
        let mut evidence = Vec::new();

        for condition in conditions {
            let result = Box::pin(self.evaluate(condition, context)).await?;
            if !result.matched {
                return Ok(ConditionResult::not_matched());
            }
            evidence.extend(result.evidence);
        }

        Ok(ConditionResult {
            matched: true,
            evidence,
        })
    }

    async fn eval_or(
        &self,
        conditions: &[Condition],
        context: &ConditionContext,
    ) -> Result<ConditionResult, PolicyError> {
        for condition in conditions {
            let result = Box::pin(self.evaluate(condition, context)).await?;
            if result.matched {
                return Ok(result);
            }
        }

        Ok(ConditionResult::not_matched())
    }

    async fn eval_not(
        &self,
        condition: &Condition,
        context: &ConditionContext,
    ) -> Result<ConditionResult, PolicyError> {
        let result = Box::pin(self.evaluate(condition, context)).await?;
        if result.matched {
            Ok(ConditionResult::not_matched())
        } else {
            Ok(ConditionResult::matched("NOT condition matched"))
        }
    }

    // =========================================================================
    // Custom condition
    // =========================================================================

    fn eval_custom(
        &self,
        name: &str,
        _params: &HashMap<String, Value>,
        _context: &ConditionContext,
    ) -> Result<ConditionResult, PolicyError> {
        // Custom conditions are not implemented by default
        // Users can extend this by implementing their own evaluator
        Err(PolicyError::InvalidCondition {
            reason: format!("Custom condition '{}' not implemented", name),
        })
    }

    // =========================================================================
    // Helper methods
    // =========================================================================

    fn get_string_field(&self, field: &str, context: &ConditionContext) -> Result<String, PolicyError> {
        let value = context
            .event
            .get_field(field)
            .ok_or_else(|| PolicyError::FieldNotFound {
                field: field.to_string(),
            })?;

        match value {
            Value::String(s) => Ok(s.clone()),
            Value::Number(n) => Ok(n.to_string()),
            Value::Bool(b) => Ok(b.to_string()),
            _ => Err(PolicyError::TypeMismatch {
                field: field.to_string(),
                expected: "string".to_string(),
                actual: format!("{:?}", value),
            }),
        }
    }

    fn get_numeric_field(&self, field: &str, context: &ConditionContext) -> Result<f64, PolicyError> {
        let value = context
            .event
            .get_field(field)
            .ok_or_else(|| PolicyError::FieldNotFound {
                field: field.to_string(),
            })?;

        match value {
            Value::Number(n) => n.as_f64().ok_or_else(|| PolicyError::TypeMismatch {
                field: field.to_string(),
                expected: "number".to_string(),
                actual: format!("{:?}", value),
            }),
            Value::String(s) => s.parse::<f64>().map_err(|_| PolicyError::TypeMismatch {
                field: field.to_string(),
                expected: "number".to_string(),
                actual: format!("string: {}", s),
            }),
            _ => Err(PolicyError::TypeMismatch {
                field: field.to_string(),
                expected: "number".to_string(),
                actual: format!("{:?}", value),
            }),
        }
    }

    fn get_datetime_field(
        &self,
        field: &str,
        context: &ConditionContext,
    ) -> Result<DateTime<Utc>, PolicyError> {
        let value = context
            .event
            .get_field(field)
            .ok_or_else(|| PolicyError::FieldNotFound {
                field: field.to_string(),
            })?;

        match value {
            Value::String(s) => self.parse_datetime(s),
            _ => Err(PolicyError::TypeMismatch {
                field: field.to_string(),
                expected: "datetime string".to_string(),
                actual: format!("{:?}", value),
            }),
        }
    }

    fn parse_datetime(&self, s: &str) -> Result<DateTime<Utc>, PolicyError> {
        // Try various formats
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return Ok(dt.with_timezone(&Utc));
        }

        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
            return Ok(dt.and_utc());
        }

        if let Ok(dt) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
            return Ok(dt.and_hms_opt(0, 0, 0).unwrap().and_utc());
        }

        Err(PolicyError::evaluation(format!("Invalid datetime: {}", s)))
    }

    fn parse_duration(&self, s: &str) -> Result<Duration, PolicyError> {
        // Parse duration strings like "5m", "1h", "30s", "1d"
        let s = s.trim();
        if s.is_empty() {
            return Err(PolicyError::evaluation("Empty duration string"));
        }

        let (num_str, unit) = s.split_at(s.len() - 1);
        let num: i64 = num_str
            .parse()
            .map_err(|_| PolicyError::evaluation(format!("Invalid duration number: {}", num_str)))?;

        match unit {
            "s" => Ok(Duration::seconds(num)),
            "m" => Ok(Duration::minutes(num)),
            "h" => Ok(Duration::hours(num)),
            "d" => Ok(Duration::days(num)),
            "w" => Ok(Duration::weeks(num)),
            _ => Err(PolicyError::evaluation(format!(
                "Unknown duration unit: {}",
                unit
            ))),
        }
    }

    fn get_or_compile_regex(&self, pattern: &str) -> Result<Regex, PolicyError> {
        // Check cache first
        {
            let cache = self.regex_cache.read().unwrap();
            if let Some(regex) = cache.get(pattern) {
                return Ok(regex.clone());
            }
        }

        // Compile and cache
        let regex = Regex::new(pattern)?;
        {
            let mut cache = self.regex_cache.write().unwrap();
            cache.insert(pattern.to_string(), regex.clone());
        }

        Ok(regex)
    }
}

impl Default for ConditionEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policies::rule_engine::EventType;

    fn create_test_context() -> ConditionContext {
        let event = Event::new(EventType::ToolExecution)
            .with_data("tool_name", "bash")
            .with_data("command", "rm -rf /tmp/test")
            .with_data("count", 42)
            .with_data("price", 19.99)
            .with_data("tags", vec!["admin", "dangerous"])
            .with_data("empty_string", "")
            .with_data("config", serde_json::json!({"key": "value", "nested": {"deep": true}}));

        ConditionContext { event }
    }

    #[tokio::test]
    async fn test_contains_case_insensitive() {
        let evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::Contains {
            field: "command".to_string(),
            value: "RM".to_string(),
            case_sensitive: false,
        };

        let result = evaluator.evaluate(&condition, &context).await.unwrap();
        assert!(result.matched);
    }

    #[tokio::test]
    async fn test_contains_case_sensitive() {
        let evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::Contains {
            field: "command".to_string(),
            value: "RM".to_string(),
            case_sensitive: true,
        };

        let result = evaluator.evaluate(&condition, &context).await.unwrap();
        assert!(!result.matched);
    }

    #[tokio::test]
    async fn test_matches_regex() {
        let evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::Matches {
            field: "command".to_string(),
            pattern: r"rm\s+-rf".to_string(),
        };

        let result = evaluator.evaluate(&condition, &context).await.unwrap();
        assert!(result.matched);
    }

    #[tokio::test]
    async fn test_equals() {
        let evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::Equals {
            field: "tool_name".to_string(),
            value: Value::String("bash".to_string()),
        };

        let result = evaluator.evaluate(&condition, &context).await.unwrap();
        assert!(result.matched);
    }

    #[tokio::test]
    async fn test_starts_with() {
        let evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::StartsWith {
            field: "command".to_string(),
            value: "rm".to_string(),
            case_sensitive: true,
        };

        let result = evaluator.evaluate(&condition, &context).await.unwrap();
        assert!(result.matched);
    }

    #[tokio::test]
    async fn test_ends_with() {
        let evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::EndsWith {
            field: "command".to_string(),
            value: "test".to_string(),
            case_sensitive: true,
        };

        let result = evaluator.evaluate(&condition, &context).await.unwrap();
        assert!(result.matched);
    }

    #[tokio::test]
    async fn test_is_empty() {
        let evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::IsEmpty {
            field: "empty_string".to_string(),
        };

        let result = evaluator.evaluate(&condition, &context).await.unwrap();
        assert!(result.matched);
    }

    #[tokio::test]
    async fn test_greater_than() {
        let evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::GreaterThan {
            field: "count".to_string(),
            value: 40.0,
        };

        let result = evaluator.evaluate(&condition, &context).await.unwrap();
        assert!(result.matched);
    }

    #[tokio::test]
    async fn test_between() {
        let evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::Between {
            field: "price".to_string(),
            min: 10.0,
            max: 20.0,
        };

        let result = evaluator.evaluate(&condition, &context).await.unwrap();
        assert!(result.matched);
    }

    #[tokio::test]
    async fn test_in_list() {
        let evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::InList {
            field: "tool_name".to_string(),
            values: vec![
                Value::String("bash".to_string()),
                Value::String("sh".to_string()),
            ],
        };

        let result = evaluator.evaluate(&condition, &context).await.unwrap();
        assert!(result.matched);
    }

    #[tokio::test]
    async fn test_has_key() {
        let evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::HasKey {
            field: "config".to_string(),
            key: "nested".to_string(),
        };

        let result = evaluator.evaluate(&condition, &context).await.unwrap();
        assert!(result.matched);
    }

    #[tokio::test]
    async fn test_array_contains() {
        let evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::ArrayContains {
            field: "tags".to_string(),
            value: Value::String("admin".to_string()),
        };

        let result = evaluator.evaluate(&condition, &context).await.unwrap();
        assert!(result.matched);
    }

    #[tokio::test]
    async fn test_and_condition() {
        let evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::And {
            conditions: vec![
                Condition::Equals {
                    field: "tool_name".to_string(),
                    value: Value::String("bash".to_string()),
                },
                Condition::Contains {
                    field: "command".to_string(),
                    value: "rm".to_string(),
                    case_sensitive: true,
                },
            ],
        };

        let result = evaluator.evaluate(&condition, &context).await.unwrap();
        assert!(result.matched);
    }

    #[tokio::test]
    async fn test_or_condition() {
        let evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::Or {
            conditions: vec![
                Condition::Equals {
                    field: "tool_name".to_string(),
                    value: Value::String("python".to_string()),
                },
                Condition::Equals {
                    field: "tool_name".to_string(),
                    value: Value::String("bash".to_string()),
                },
            ],
        };

        let result = evaluator.evaluate(&condition, &context).await.unwrap();
        assert!(result.matched);
    }

    #[tokio::test]
    async fn test_not_condition() {
        let evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::Not {
            condition: Box::new(Condition::Equals {
                field: "tool_name".to_string(),
                value: Value::String("python".to_string()),
            }),
        };

        let result = evaluator.evaluate(&condition, &context).await.unwrap();
        assert!(result.matched);
    }

    #[tokio::test]
    async fn test_always_never() {
        let evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let always = evaluator.evaluate(&Condition::Always, &context).await.unwrap();
        assert!(always.matched);

        let never = evaluator.evaluate(&Condition::Never, &context).await.unwrap();
        assert!(!never.matched);
    }

    #[test]
    fn test_parse_duration() {
        let evaluator = ConditionEvaluator::new();

        assert_eq!(evaluator.parse_duration("5s").unwrap(), Duration::seconds(5));
        assert_eq!(evaluator.parse_duration("30m").unwrap(), Duration::minutes(30));
        assert_eq!(evaluator.parse_duration("24h").unwrap(), Duration::hours(24));
        assert_eq!(evaluator.parse_duration("7d").unwrap(), Duration::days(7));
        assert_eq!(evaluator.parse_duration("2w").unwrap(), Duration::weeks(2));
    }

    #[test]
    fn test_parse_datetime() {
        let evaluator = ConditionEvaluator::new();

        // RFC3339
        assert!(evaluator.parse_datetime("2024-01-15T10:30:00Z").is_ok());

        // Date only
        assert!(evaluator.parse_datetime("2024-01-15").is_ok());

        // Invalid
        assert!(evaluator.parse_datetime("not-a-date").is_err());
    }
}
