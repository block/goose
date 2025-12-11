use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ==================== COMPLETION ENDPOINT ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionPostReq {
    pub config: OrchestrationConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder_values: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages_history: Option<Vec<ChatMessage>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionPostRes {
    pub request_id: String,
    pub intermediate_results: ModuleResults,
    pub final_result: LlmModuleResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionPostStreamingRes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intermediate_results: Option<ModuleResultsStreaming>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_result: Option<LlmModuleResultStreaming>,
}

// ==================== EMBEDDINGS ENDPOINT ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsPostReq {
    pub config: EmbeddingsOrchestrationConfig,
    pub input: EmbeddingsInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsPostRes {
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intermediate_results: Option<ModuleResultsBase>,
    pub final_result: EmbeddingsResponse,
}

// ==================== ORCHESTRATION CONFIG ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationConfig {
    pub modules: ModuleConfigs,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<GlobalStreamOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsOrchestrationConfig {
    pub modules: EmbeddingsModuleConfigs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleConfigs {
    pub prompt_templating: PromptTemplatingModuleConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filtering: Option<FilteringModuleConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub masking: Option<MaskingModuleConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounding: Option<GroundingModuleConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translation: Option<TranslationModuleConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsModuleConfigs {
    pub embeddings: EmbeddingsModelConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub masking: Option<MaskingModuleConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalStreamOptions {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_chunk_size")]
    pub chunk_size: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiters: Option<Vec<String>>,
}

fn default_chunk_size() -> u32 {
    100
}

// ==================== PROMPT TEMPLATING ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplatingModuleConfig {
    pub prompt: TemplateOrRef,
    pub model: LlmModelDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TemplateOrRef {
    Template(Template),
    TemplateRef(TemplateRef),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub template: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defaults: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ChatCompletionTool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateRef {
    pub template_ref: TemplateReference,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TemplateReference {
    ById {
        id: String,
    },
    ByScenarioNameVersion {
        scenario: String,
        name: String,
        version: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseFormat {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "json_object")]
    JsonObject,
    #[serde(rename = "json_schema")]
    JsonSchema { json_schema: JsonSchemaSpec },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchemaSpec {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,
    #[serde(default)]
    pub strict: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionTool {
    #[serde(rename = "type")]
    pub tool_type: String, // Always "function"
    pub function: FunctionObject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionObject {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
    #[serde(default)]
    pub strict: bool,
}

// ==================== MODEL DETAILS ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmModelDetails {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsModelConfig {
    pub model: EmbeddingsModelDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsModelDetails {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<EmbeddingsModelParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsModelParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<EmbeddingEncodingFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalize: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingEncodingFormat {
    Float,
    Base64,
    Binary,
}

fn default_version() -> String {
    "latest".to_string()
}

// ==================== CHAT MESSAGES ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum ChatMessage {
    #[serde(rename = "system")]
    System { content: ChatMessageContent },
    #[serde(rename = "user")]
    User { content: UserChatMessageContent },
    #[serde(rename = "assistant")]
    Assistant {
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<ChatMessageContent>,
        #[serde(skip_serializing_if = "Option::is_none")]
        refusal: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_calls: Option<Vec<MessageToolCall>>,
    },
    #[serde(rename = "tool")]
    Tool {
        tool_call_id: String,
        content: ChatMessageContent,
    },
    #[serde(rename = "developer")]
    Developer { content: ChatMessageContent },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatMessageContent {
    Text(String),
    Array(Vec<TextContent>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UserChatMessageContent {
    Text(String),
    Array(Vec<UserChatMessageContentItem>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UserChatMessageContentItem {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageContentUrl },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContentUrl {
    pub url: String,
    #[serde(default = "default_detail")]
    pub detail: String,
}

fn default_detail() -> String {
    "auto".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    #[serde(rename = "type")]
    pub content_type: String, // Always "text"
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub tool_type: String, // Always "function"
    pub function: ToolCallFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>, // JSON string
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseChatMessage {
    pub role: String, // Always "assistant"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<MessageToolCall>>,
}

// ==================== EMBEDDINGS INPUT/OUTPUT ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsInput {
    pub text: EmbeddingsInputText,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub input_type: Option<EmbeddingsInputType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EmbeddingsInputText {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingsInputType {
    Text,
    Document,
    Query,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsResponse {
    pub object: String, // Always "list"
    pub data: Vec<EmbeddingResult>,
    pub model: String,
    pub usage: EmbeddingsUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResult {
    pub object: String, // Always "embedding"
    pub embedding: Embedding,
    pub index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Embedding {
    FloatArray(Vec<f64>),
    Base64String(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsUsage {
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}

// ==================== MODULE RESULTS ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleResultsBase {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounding: Option<GenericModuleResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub templating: Option<Vec<ChatMessage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_translation: Option<GenericModuleResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_masking: Option<GenericModuleResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_filtering: Option<GenericModuleResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_filtering: Option<GenericModuleResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_translation: Option<GenericModuleResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleResults {
    #[serde(flatten)]
    pub base: ModuleResultsBase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm: Option<LlmModuleResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_unmasking: Option<Vec<LlmChoice>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleResultsStreaming {
    #[serde(flatten)]
    pub base: ModuleResultsBase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm: Option<LlmModuleResultStreaming>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_unmasking: Option<Vec<LlmChoiceStreaming>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericModuleResult {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

// ==================== LLM MODULE RESULTS ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmModuleResult {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    pub choices: Vec<LlmChoice>,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmModuleResultStreaming {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    pub choices: Vec<LlmChoiceStreaming>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmChoice {
    pub index: u32,
    pub message: ResponseChatMessage,
    pub finish_reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<ChoiceLogprobs>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmChoiceStreaming {
    pub index: u32,
    pub delta: ChatDelta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<ChoiceLogprobs>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatDelta {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallChunk>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallChunk {
    pub index: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub tool_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<ToolCallFunction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub completion_tokens: u32,
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoiceLogprobs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<ChatCompletionTokenLogprob>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<Vec<ChatCompletionTokenLogprob>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionTokenLogprob {
    pub token: String,
    pub logprob: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<Vec<i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<Vec<TopLogprob>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopLogprob {
    pub token: String,
    pub logprob: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<Vec<i32>>,
}

// ==================== FILTERING MODULE ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilteringModuleConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<InputFilteringConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputFilteringConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputFilteringConfig {
    pub filters: Vec<InputFilterConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputFilteringConfig {
    pub filters: Vec<OutputFilterConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<FilteringStreamOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilteringStreamOptions {
    #[serde(default)]
    pub overlap: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum InputFilterConfig {
    #[serde(rename = "azure_content_safety")]
    AzureContentSafety { config: AzureContentSafetyInput },
    #[serde(rename = "llama_guard_3_8b")]
    LlamaGuard38b { config: LlamaGuard38b },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OutputFilterConfig {
    #[serde(rename = "azure_content_safety")]
    AzureContentSafety { config: AzureContentSafetyOutput },
    #[serde(rename = "llama_guard_3_8b")]
    LlamaGuard38b { config: LlamaGuard38b },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureContentSafetyInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hate: Option<AzureThreshold>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub self_harm: Option<AzureThreshold>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sexual: Option<AzureThreshold>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub violence: Option<AzureThreshold>,
    #[serde(default)]
    pub prompt_shield: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureContentSafetyOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hate: Option<AzureThreshold>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub self_harm: Option<AzureThreshold>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sexual: Option<AzureThreshold>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub violence: Option<AzureThreshold>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AzureThreshold {
    #[serde(rename = "0")]
    Zero = 0,
    #[serde(rename = "2")]
    Two = 2,
    #[serde(rename = "4")]
    Four = 4,
    #[serde(rename = "6")]
    Six = 6,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlamaGuard38b {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub violent_crimes: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub non_violent_crimes: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sex_crimes: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_exploitation: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defamation: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specialized_advice: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intellectual_property: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indiscriminate_weapons: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hate: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub self_harm: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sexual_content: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elections: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_interpreter_abuse: Option<bool>,
}

// ==================== MASKING MODULE ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskingModuleConfig {
    pub masking_providers: Vec<MaskingProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MaskingProviderConfig {
    #[serde(rename = "sap_data_privacy_integration")]
    SapDataPrivacyIntegration {
        method: DpiMethod,
        entities: Vec<DpiEntityConfig>,
        #[serde(skip_serializing_if = "Option::is_none")]
        allowlist: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        mask_grounding_input: Option<MaskGroundingInputConfig>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DpiMethod {
    Anonymization,
    Pseudonymization,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskGroundingInputConfig {
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DpiEntityConfig {
    Standard(DpiStandardEntity),
    Custom(DpiCustomEntity),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DpiStandardEntity {
    #[serde(rename = "type")]
    pub entity_type: DpiEntityType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement_strategy: Option<DpiReplacementStrategy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DpiCustomEntity {
    pub regex: String,
    pub replacement_strategy: DpiReplacementStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum DpiReplacementStrategy {
    #[serde(rename = "constant")]
    Constant { value: String },
    #[serde(rename = "fabricated_data")]
    FabricatedData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DpiEntityType {
    ProfilePerson,
    ProfileOrg,
    ProfileUniversity,
    ProfileLocation,
    ProfileEmail,
    ProfilePhone,
    ProfileAddress,
    ProfileSapidsInternal,
    ProfileSapidsPublic,
    ProfileUrl,
    ProfileUsernamePassword,
    ProfileNationalid,
    ProfileIban,
    ProfileSsn,
    ProfileCreditCardNumber,
    ProfilePassport,
    ProfileDriverlicense,
    ProfileNationality,
    ProfileReligiousGroup,
    ProfilePoliticalGroup,
    ProfilePronounsGender,
    ProfileEthnicity,
    ProfileGender,
    ProfileSexualOrientation,
    ProfileTradeUnion,
    ProfileSensitiveData,
}

// ==================== GROUNDING MODULE ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundingModuleConfig {
    #[serde(rename = "type")]
    pub grounding_type: String, // Usually "document_grounding_service"
    pub config: GroundingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundingConfig {
    pub placeholders: GroundingPlaceholders,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<DocumentGroundingFilter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_params: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundingPlaceholders {
    pub input: Vec<String>,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentGroundingFilter {
    pub data_repository_type: DataRepositoryType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_config: Option<GroundingFilterSearchConfig>,
    #[serde(default = "default_data_repositories")]
    pub data_repositories: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_repository_metadata: Option<Vec<KeyValueListPair>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_metadata: Option<Vec<SearchDocumentKeyValueListPair>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_metadata: Option<Vec<KeyValueListPair>>,
}

fn default_data_repositories() -> Vec<String> {
    vec!["*".to_string()]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataRepositoryType {
    Vector,
    #[serde(rename = "help.sap.com")]
    HelpSapCom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundingFilterSearchConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_chunk_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_document_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyValueListPair {
    pub key: String,
    pub value: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchDocumentKeyValueListPair {
    pub key: String,
    pub value: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select_mode: Option<Vec<SearchSelectOption>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SearchSelectOption {
    IgnoreIfKeyAbsent,
}

// ==================== TRANSLATION MODULE ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationModuleConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<SapDocumentTranslation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<SapDocumentTranslation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SapDocumentTranslation {
    #[serde(rename = "type")]
    pub translation_type: String, // Usually "sap_document_translation"
    pub config: TranslationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationConfig {
    pub target_language: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_language: Option<String>,
}

// ==================== ERROR TYPES ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorStreamingResponse {
    pub error: ErrorStreaming,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    pub request_id: String,
    pub code: u32,
    pub message: String,
    pub location: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intermediate_results: Option<ModuleResults>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorStreaming {
    pub request_id: String,
    pub code: u32,
    pub message: String,
    pub location: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intermediate_results: Option<ModuleResultsStreaming>,
}
