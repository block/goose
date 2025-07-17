use crate::session::storage::SessionMetadata;
use crate::telemetry::config::UsageType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub trait TelemetryExecution {
    type ResultType: Clone;

    fn start_time(&self) -> u64;
    fn end_time(&self) -> Option<u64>;
    fn set_end_time(&mut self, time: u64);
    fn duration_ms(&self) -> Option<u64>;
    fn set_duration_ms(&mut self, duration: u64);
    fn result(&self) -> Option<&Self::ResultType>;
    fn set_result(&mut self, result: Self::ResultType);
    fn user_id(&self) -> &str;
    fn set_user_id(&mut self, user_id: String);
    fn usage_type(&self) -> &UsageType;
    fn set_usage_type(&mut self, usage_type: UsageType);
    fn environment(&self) -> Option<&String>;
    fn set_environment(&mut self, environment: String);
    fn metadata(&self) -> &HashMap<String, String>;
    fn metadata_mut(&mut self) -> &mut HashMap<String, String>;
    fn error_details(&self) -> Option<&ErrorDetails>;
    fn set_error_details(&mut self, error_details: ErrorDetails);

    fn with_result(mut self, result: Self::ResultType) -> Self
    where
        Self: Sized,
    {
        let end_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.set_end_time(end_time);
        self.set_duration_ms((end_time - self.start_time()) * 1000);
        self.set_result(result);
        self
    }

    fn with_duration(mut self, duration: Duration) -> Self
    where
        Self: Sized,
    {
        self.set_duration_ms(duration.as_millis() as u64);
        self
    }

    fn with_metadata(mut self, key: &str, value: &str) -> Self
    where
        Self: Sized,
    {
        self.metadata_mut()
            .insert(key.to_string(), value.to_string());
        self
    }

    fn with_environment(mut self, environment: &str) -> Self
    where
        Self: Sized,
    {
        self.set_environment(environment.to_string());
        self
    }

    fn with_error_details(mut self, error_details: ErrorDetails) -> Self
    where
        Self: Sized,
    {
        self.set_error_details(error_details);
        self
    }

    fn complete(&mut self) {
        if self.end_time().is_none() {
            let end_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            self.set_end_time(end_time);
            self.set_duration_ms((end_time - self.start_time()) * 1000);
        }
    }
}

pub trait SessionMetadataSupport {
    fn token_usage(&self) -> Option<&TokenUsage>;
    fn set_token_usage(&mut self, token_usage: TokenUsage);
    fn tool_usage(&self) -> &Vec<ToolUsage>;
    fn tool_usage_mut(&mut self) -> &mut Vec<ToolUsage>;
    fn message_count(&self) -> u64;
    fn set_message_count(&mut self, count: u64);
    fn turn_count(&self) -> u64;
    fn set_turn_count(&mut self, count: u64);

    fn with_token_usage(mut self, token_usage: TokenUsage) -> Self
    where
        Self: Sized,
    {
        self.set_token_usage(token_usage);
        self
    }

    fn add_tool_usage(&mut self, tool_usage: ToolUsage) {
        self.tool_usage_mut().push(tool_usage);
    }

    fn with_message_count(mut self, count: u64) -> Self
    where
        Self: Sized,
    {
        self.set_message_count(count);
        self
    }

    fn with_turn_count(mut self, count: u64) -> Self
    where
        Self: Sized,
    {
        self.set_turn_count(count);
        self
    }

    fn with_session_metadata(mut self, session_metadata: &SessionMetadata) -> Self
    where
        Self: Sized + TelemetryExecution,
    {
        self.set_message_count(session_metadata.message_count as u64);

        self.metadata_mut().insert(
            "working_dir".to_string(),
            session_metadata.working_dir.to_string_lossy().to_string(),
        );

        if let Some(schedule_id) = &session_metadata.schedule_id {
            self.metadata_mut()
                .insert("schedule_id".to_string(), schedule_id.clone());
        }

        if let (Some(input), Some(output)) = (
            session_metadata.input_tokens,
            session_metadata.output_tokens,
        ) {
            self.set_token_usage(TokenUsage::new(input as u64, output as u64));
        }

        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeExecution {
    pub recipe_name: String,
    pub recipe_version: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub duration_ms: Option<u64>,
    pub result: Option<SessionResult>,
    pub user_id: String,
    pub usage_type: UsageType,
    pub environment: Option<String>,
    pub metadata: HashMap<String, String>,
    pub token_usage: Option<TokenUsage>,
    pub tool_usage: Vec<ToolUsage>,
    pub message_count: u64,
    pub turn_count: u64,
    pub error_details: Option<ErrorDetails>,
}

impl RecipeExecution {
    pub fn new(recipe_name: &str, recipe_version: &str) -> Self {
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            recipe_name: recipe_name.to_string(),
            recipe_version: recipe_version.to_string(),
            start_time,
            end_time: None,
            duration_ms: None,
            result: None,
            user_id: String::new(),
            usage_type: UsageType::Human,
            environment: None,
            metadata: HashMap::new(),
            token_usage: None,
            tool_usage: Vec::new(),
            message_count: 0,
            turn_count: 0,
            error_details: None,
        }
    }
}

impl TelemetryExecution for RecipeExecution {
    type ResultType = SessionResult;

    fn start_time(&self) -> u64 {
        self.start_time
    }
    fn end_time(&self) -> Option<u64> {
        self.end_time
    }
    fn set_end_time(&mut self, time: u64) {
        self.end_time = Some(time);
    }
    fn duration_ms(&self) -> Option<u64> {
        self.duration_ms
    }
    fn set_duration_ms(&mut self, duration: u64) {
        self.duration_ms = Some(duration);
    }
    fn result(&self) -> Option<&Self::ResultType> {
        self.result.as_ref()
    }
    fn set_result(&mut self, result: Self::ResultType) {
        self.result = Some(result);
    }
    fn user_id(&self) -> &str {
        &self.user_id
    }
    fn set_user_id(&mut self, user_id: String) {
        self.user_id = user_id;
    }
    fn usage_type(&self) -> &UsageType {
        &self.usage_type
    }
    fn set_usage_type(&mut self, usage_type: UsageType) {
        self.usage_type = usage_type;
    }
    fn environment(&self) -> Option<&String> {
        self.environment.as_ref()
    }
    fn set_environment(&mut self, environment: String) {
        self.environment = Some(environment);
    }
    fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }
    fn metadata_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.metadata
    }
    fn error_details(&self) -> Option<&ErrorDetails> {
        self.error_details.as_ref()
    }
    fn set_error_details(&mut self, error_details: ErrorDetails) {
        self.error_details = Some(error_details);
    }
}

impl SessionMetadataSupport for RecipeExecution {
    fn token_usage(&self) -> Option<&TokenUsage> {
        self.token_usage.as_ref()
    }
    fn set_token_usage(&mut self, token_usage: TokenUsage) {
        self.token_usage = Some(token_usage);
    }
    fn tool_usage(&self) -> &Vec<ToolUsage> {
        &self.tool_usage
    }
    fn tool_usage_mut(&mut self) -> &mut Vec<ToolUsage> {
        &mut self.tool_usage
    }
    fn message_count(&self) -> u64 {
        self.message_count
    }
    fn set_message_count(&mut self, count: u64) {
        self.message_count = count;
    }
    fn turn_count(&self) -> u64 {
        self.turn_count
    }
    fn set_turn_count(&mut self, count: u64) {
        self.turn_count = count;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub estimated_cost: Option<f64>,
    pub model: Option<String>,
    pub provider: Option<String>,
}

impl TokenUsage {
    pub fn new(input_tokens: u64, output_tokens: u64) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
            estimated_cost: None,
            model: None,
            provider: None,
        }
    }

    pub fn with_cost(mut self, cost: f64) -> Self {
        self.estimated_cost = Some(cost);
        self
    }

    pub fn with_model(mut self, model: &str, provider: &str) -> Self {
        self.model = Some(model.to_string());
        self.provider = Some(provider.to_string());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsage {
    pub tool_name: String,
    pub call_count: u64,
    pub total_duration_ms: u64,
    pub avg_duration_ms: u64,
    pub success_count: u64,
    pub error_count: u64,
}

impl ToolUsage {
    pub fn new(tool_name: &str) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            call_count: 0,
            total_duration_ms: 0,
            avg_duration_ms: 0,
            success_count: 0,
            error_count: 0,
        }
    }

    pub fn add_call(&mut self, duration: Duration, success: bool) {
        self.call_count += 1;
        let duration_ms = duration.as_millis() as u64;
        self.total_duration_ms += duration_ms;
        self.avg_duration_ms = self.total_duration_ms / self.call_count;

        if success {
            self.success_count += 1;
        } else {
            self.error_count += 1;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetails {
    pub error_type: String,
    pub error_message: String,
    pub stack_trace: Option<String>,
    pub failing_tool: Option<String>,
    pub context: HashMap<String, String>,
}

impl ErrorDetails {
    pub fn new(error_type: &str, error_message: &str) -> Self {
        Self {
            error_type: error_type.to_string(),
            error_message: error_message.to_string(),
            stack_trace: None,
            failing_tool: None,
            context: HashMap::new(),
        }
    }

    pub fn with_stack_trace(mut self, stack_trace: &str) -> Self {
        self.stack_trace = Some(stack_trace.to_string());
        self
    }

    pub fn with_failing_tool(mut self, tool_name: &str) -> Self {
        self.failing_tool = Some(tool_name.to_string());
        self
    }

    pub fn with_context(mut self, key: &str, value: &str) -> Self {
        self.context.insert(key.to_string(), value.to_string());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum TelemetryEvent {
    RecipeExecution(RecipeExecution),
    SessionExecution(SessionExecution),
    CommandExecution(CommandExecution),
    SystemMetrics(SystemMetrics),
    UserSession(UserSession),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionExecution {
    pub session_id: String,
    pub session_type: SessionType,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub duration_ms: Option<u64>,
    pub result: Option<SessionResult>,
    pub user_id: String,
    pub usage_type: UsageType,
    pub environment: Option<String>,
    pub metadata: HashMap<String, String>,
    pub token_usage: Option<TokenUsage>,
    pub tool_usage: Vec<ToolUsage>,
    pub message_count: u64,
    pub turn_count: u64,
    pub error_details: Option<ErrorDetails>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionType {
    Interactive,
    Headless,
    Scheduled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionResult {
    Success,
    Error(String),
    Interrupted,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecution {
    pub command_name: String,
    pub command_type: CommandType,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub duration_ms: Option<u64>,
    pub result: Option<CommandResult>,
    pub user_id: String,
    pub usage_type: UsageType,
    pub environment: Option<String>,
    pub metadata: HashMap<String, String>,
    pub arguments: HashMap<String, String>,
    pub error_details: Option<ErrorDetails>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandType {
    Configure,
    Info,
    SessionList,
    SessionRemove,
    SessionExport,
    ProjectOpen,
    ProjectsList,
    RecipeValidate,
    RecipeDeeplink,
    ScheduleAdd,
    ScheduleList,
    ScheduleRemove,
    ScheduleRunNow,
    ScheduleSessions,
    Update,
    Bench,
    Web,
    Mcp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandResult {
    Success,
    Error(String),
    Cancelled,
}

impl SessionExecution {
    pub fn new(session_id: &str, session_type: SessionType) -> Self {
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            session_id: session_id.to_string(),
            session_type,
            start_time,
            end_time: None,
            duration_ms: None,
            result: None,
            user_id: String::new(),
            usage_type: UsageType::Human,
            environment: None,
            metadata: HashMap::new(),
            token_usage: None,
            tool_usage: Vec::new(),
            message_count: 0,
            turn_count: 0,
            error_details: None,
        }
    }
}

impl TelemetryExecution for SessionExecution {
    type ResultType = SessionResult;

    fn start_time(&self) -> u64 {
        self.start_time
    }
    fn end_time(&self) -> Option<u64> {
        self.end_time
    }
    fn set_end_time(&mut self, time: u64) {
        self.end_time = Some(time);
    }
    fn duration_ms(&self) -> Option<u64> {
        self.duration_ms
    }
    fn set_duration_ms(&mut self, duration: u64) {
        self.duration_ms = Some(duration);
    }
    fn result(&self) -> Option<&Self::ResultType> {
        self.result.as_ref()
    }
    fn set_result(&mut self, result: Self::ResultType) {
        self.result = Some(result);
    }
    fn user_id(&self) -> &str {
        &self.user_id
    }
    fn set_user_id(&mut self, user_id: String) {
        self.user_id = user_id;
    }
    fn usage_type(&self) -> &UsageType {
        &self.usage_type
    }
    fn set_usage_type(&mut self, usage_type: UsageType) {
        self.usage_type = usage_type;
    }
    fn environment(&self) -> Option<&String> {
        self.environment.as_ref()
    }
    fn set_environment(&mut self, environment: String) {
        self.environment = Some(environment);
    }
    fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }
    fn metadata_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.metadata
    }
    fn error_details(&self) -> Option<&ErrorDetails> {
        self.error_details.as_ref()
    }
    fn set_error_details(&mut self, error_details: ErrorDetails) {
        self.error_details = Some(error_details);
    }
}

impl SessionMetadataSupport for SessionExecution {
    fn token_usage(&self) -> Option<&TokenUsage> {
        self.token_usage.as_ref()
    }
    fn set_token_usage(&mut self, token_usage: TokenUsage) {
        self.token_usage = Some(token_usage);
    }
    fn tool_usage(&self) -> &Vec<ToolUsage> {
        &self.tool_usage
    }
    fn tool_usage_mut(&mut self) -> &mut Vec<ToolUsage> {
        &mut self.tool_usage
    }
    fn message_count(&self) -> u64 {
        self.message_count
    }
    fn set_message_count(&mut self, count: u64) {
        self.message_count = count;
    }
    fn turn_count(&self) -> u64 {
        self.turn_count
    }
    fn set_turn_count(&mut self, count: u64) {
        self.turn_count = count;
    }
}

impl CommandExecution {
    pub fn new(command_name: &str, command_type: CommandType) -> Self {
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            command_name: command_name.to_string(),
            command_type,
            start_time,
            end_time: None,
            duration_ms: None,
            result: None,
            user_id: String::new(),
            usage_type: UsageType::Human,
            environment: None,
            metadata: HashMap::new(),
            arguments: HashMap::new(),
            error_details: None,
        }
    }

    pub fn with_argument(mut self, key: &str, value: &str) -> Self {
        self.arguments.insert(key.to_string(), value.to_string());
        self
    }
}

impl TelemetryExecution for CommandExecution {
    type ResultType = CommandResult;

    fn start_time(&self) -> u64 {
        self.start_time
    }
    fn end_time(&self) -> Option<u64> {
        self.end_time
    }
    fn set_end_time(&mut self, time: u64) {
        self.end_time = Some(time);
    }
    fn duration_ms(&self) -> Option<u64> {
        self.duration_ms
    }
    fn set_duration_ms(&mut self, duration: u64) {
        self.duration_ms = Some(duration);
    }
    fn result(&self) -> Option<&Self::ResultType> {
        self.result.as_ref()
    }
    fn set_result(&mut self, result: Self::ResultType) {
        self.result = Some(result);
    }
    fn user_id(&self) -> &str {
        &self.user_id
    }
    fn set_user_id(&mut self, user_id: String) {
        self.user_id = user_id;
    }
    fn usage_type(&self) -> &UsageType {
        &self.usage_type
    }
    fn set_usage_type(&mut self, usage_type: UsageType) {
        self.usage_type = usage_type;
    }
    fn environment(&self) -> Option<&String> {
        self.environment.as_ref()
    }
    fn set_environment(&mut self, environment: String) {
        self.environment = Some(environment);
    }
    fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }
    fn metadata_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.metadata
    }
    fn error_details(&self) -> Option<&ErrorDetails> {
        self.error_details.as_ref()
    }
    fn set_error_details(&mut self, error_details: ErrorDetails) {
        self.error_details = Some(error_details);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: u64,
    pub memory_usage: Option<u64>,
    pub cpu_usage: Option<f64>,
    pub active_sessions: u64,
    pub load_average: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub session_id: String,
    pub user_id: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub duration_seconds: Option<u64>,
    pub usage_type: UsageType,
    pub recipe_count: u64,
    pub total_tokens: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recipe_execution_creation() {
        let execution = RecipeExecution::new("test-recipe", "1.0.0");

        assert_eq!(execution.recipe_name, "test-recipe");
        assert_eq!(execution.recipe_version, "1.0.0");
        assert!(execution.start_time > 0);
        assert!(execution.end_time.is_none());
        assert!(execution.result.is_none());
    }

    #[test]
    fn test_recipe_execution_completion() {
        let execution =
            RecipeExecution::new("test-recipe", "1.0.0").with_result(SessionResult::Success);

        assert!(execution.end_time.is_some());
        assert!(execution.duration_ms.is_some());
        assert_eq!(execution.result, Some(SessionResult::Success));
    }

    #[test]
    fn test_recipe_execution_metadata() {
        let execution = RecipeExecution::new("test-recipe", "1.0.0")
            .with_metadata("key1", "value1")
            .with_metadata("key2", "value2");

        assert_eq!(execution.metadata.get("key1"), Some(&"value1".to_string()));
        assert_eq!(execution.metadata.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_recipe_execution_environment() {
        let execution =
            RecipeExecution::new("test-recipe", "1.0.0").with_environment("macos,x86_64,iterm");

        assert_eq!(
            execution.environment,
            Some("macos,x86_64,iterm".to_string())
        );
    }

    #[test]
    fn test_token_usage() {
        let token_usage = TokenUsage::new(100, 50)
            .with_cost(0.01)
            .with_model("gpt-4", "openai");

        assert_eq!(token_usage.input_tokens, 100);
        assert_eq!(token_usage.output_tokens, 50);
        assert_eq!(token_usage.total_tokens, 150);
        assert_eq!(token_usage.estimated_cost, Some(0.01));
        assert_eq!(token_usage.model, Some("gpt-4".to_string()));
        assert_eq!(token_usage.provider, Some("openai".to_string()));
    }

    #[test]
    fn test_tool_usage() {
        let mut tool_usage = ToolUsage::new("test-tool");

        tool_usage.add_call(Duration::from_millis(100), true);
        tool_usage.add_call(Duration::from_millis(200), false);

        assert_eq!(tool_usage.call_count, 2);
        assert_eq!(tool_usage.total_duration_ms, 300);
        assert_eq!(tool_usage.avg_duration_ms, 150);
        assert_eq!(tool_usage.success_count, 1);
        assert_eq!(tool_usage.error_count, 1);
    }

    #[test]
    fn test_error_details() {
        let error_details = ErrorDetails::new("validation_error", "Invalid input")
            .with_stack_trace("stack trace here")
            .with_failing_tool("input-validator")
            .with_context("input_type", "json");

        assert_eq!(error_details.error_type, "validation_error");
        assert_eq!(error_details.error_message, "Invalid input");
        assert_eq!(
            error_details.stack_trace,
            Some("stack trace here".to_string())
        );
        assert_eq!(
            error_details.failing_tool,
            Some("input-validator".to_string())
        );
        assert_eq!(
            error_details.context.get("input_type"),
            Some(&"json".to_string())
        );
    }

    #[test]
    fn test_telemetry_event_serialization() {
        let execution = RecipeExecution::new("test-recipe", "1.0.0");
        let event = TelemetryEvent::RecipeExecution(execution);

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: TelemetryEvent = serde_json::from_str(&json).unwrap();

        match deserialized {
            TelemetryEvent::RecipeExecution(exec) => {
                assert_eq!(exec.recipe_name, "test-recipe");
                assert_eq!(exec.recipe_version, "1.0.0");
            }
            _ => panic!("Expected RecipeExecution event"),
        }
    }

    #[test]
    fn test_telemetry_execution_trait() {
        use crate::telemetry::config::UsageType;

        let mut execution = RecipeExecution::new("test-recipe", "1.0.0");

        // Test trait methods
        assert!(execution.start_time() > 0);
        assert!(execution.end_time().is_none());
        assert_eq!(execution.user_id(), "");
        assert_eq!(execution.usage_type(), &UsageType::Human);

        // Test builder methods from trait
        let execution = execution
            .with_metadata("key1", "value1")
            .with_environment("test-env")
            .with_result(SessionResult::Success);

        assert_eq!(
            execution.metadata().get("key1"),
            Some(&"value1".to_string())
        );
        assert_eq!(execution.environment(), Some(&"test-env".to_string()));
        assert_eq!(execution.result(), Some(&SessionResult::Success));
        assert!(execution.end_time().is_some());
    }

    #[test]
    fn test_session_metadata_support_trait() {
        let mut execution = RecipeExecution::new("test-recipe", "1.0.0");

        // Test session metadata support methods
        assert_eq!(execution.message_count(), 0);
        assert_eq!(execution.turn_count(), 0);
        assert!(execution.token_usage().is_none());
        assert!(execution.tool_usage().is_empty());

        // Test builder methods from trait
        let token_usage = TokenUsage::new(100, 50);
        let execution = execution
            .with_message_count(10)
            .with_turn_count(5)
            .with_token_usage(token_usage);

        assert_eq!(execution.message_count(), 10);
        assert_eq!(execution.turn_count(), 5);
        assert!(execution.token_usage().is_some());
        assert_eq!(execution.token_usage().unwrap().total_tokens, 150);
    }

    #[test]
    fn test_command_execution_trait() {
        let mut execution = CommandExecution::new("test-command", CommandType::Configure);

        // Test trait methods
        assert!(execution.start_time() > 0);
        assert!(execution.end_time().is_none());
        assert_eq!(execution.user_id(), "");

        // Test builder methods from trait
        let execution = execution
            .with_metadata("key1", "value1")
            .with_environment("test-env")
            .with_result(CommandResult::Success);

        assert_eq!(
            execution.metadata().get("key1"),
            Some(&"value1".to_string())
        );
        assert_eq!(execution.environment(), Some(&"test-env".to_string()));
        assert_eq!(execution.result(), Some(&CommandResult::Success));
        assert!(execution.end_time().is_some());
    }

    #[test]
    fn test_session_execution_trait() {
        let mut execution = SessionExecution::new("test-session", SessionType::Interactive);

        // Test trait methods
        assert!(execution.start_time() > 0);
        assert!(execution.end_time().is_none());
        assert_eq!(execution.user_id(), "");

        // Test builder methods from trait
        let execution = execution
            .with_metadata("key1", "value1")
            .with_environment("test-env")
            .with_result(SessionResult::Success);

        assert_eq!(
            execution.metadata().get("key1"),
            Some(&"value1".to_string())
        );
        assert_eq!(execution.environment(), Some(&"test-env".to_string()));
        assert_eq!(execution.result(), Some(&SessionResult::Success));
        assert!(execution.end_time().is_some());
    }
}
