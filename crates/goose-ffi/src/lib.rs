use std::ffi::{c_char, CStr, CString};
use std::ptr;
use std::sync::Arc;

use futures::StreamExt;
use goose::agents::Agent;
use goose::message::Message;
use goose::model::ModelConfig;
use goose::providers::databricks::DatabricksProvider;
use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;

// This class is in alpha and not yet ready for production use
// and the API is not yet stable. Use at your own risk.

// Thread-safe global runtime
static RUNTIME: OnceCell<Runtime> = OnceCell::new();

// Get or initialize the global runtime
fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        // Runtime with all features enabled
        Runtime::new().expect("Failed to create Tokio runtime")
    })
}

/// Pointer type for the agent
pub type AgentPtr = *mut Agent;
/// Provider Type enumeration
/// Currently only Databricks is supported
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum ProviderType {
    /// Databricks AI provider
    Databricks = 0,
}

/// Provider configuration used to initialize an AI provider
///
/// - provider_type: Provider type (0 = Databricks, other values will produce an error)
/// - api_key: Provider API key (null for default from environment variables)
/// - model_name: Model name to use (null for provider default)
/// - host: Provider host URL (null for default from environment variables)
#[repr(C)]
pub struct ProviderConfigFFI {
    pub provider_type: ProviderType,
    pub api_key: *const c_char,
    pub model_name: *const c_char,
    pub host: *const c_char,
}

// Extension configuration will be implemented in a future commit

/// Role enum for message participants
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum MessageRole {
    /// User message role
    User = 0,
    /// Assistant message role
    Assistant = 1,
    /// System message role
    System = 2,
}

/// Message structure for agent interactions
///
/// - role: Message role (User, Assistant, or System)
/// - content: Text content of the message
#[repr(C)]
pub struct MessageFFI {
    pub role: MessageRole,
    pub content: *const c_char,
}

// Tool callbacks will be implemented in a future commit

/// Result type for async operations
///
/// - succeeded: true if the operation succeeded, false otherwise
/// - error_message: Error message if succeeded is false, NULL otherwise
#[repr(C)]
pub struct AsyncResult {
    pub succeeded: bool,
    pub error_message: *mut c_char,
}

/// Free an async result structure
///
/// This function frees the memory allocated for an AsyncResult structure,
/// including any error message it contains.
///
/// # Safety
///
/// The result pointer must be a valid pointer returned by a goose FFI function,
/// or NULL.
#[no_mangle]
pub unsafe extern "C" fn goose_free_async_result(result: *mut AsyncResult) {
    if !result.is_null() {
        let result = &mut *result;
        if !result.error_message.is_null() {
            let _ = CString::from_raw(result.error_message);
        }
        let _ = Box::from_raw(result);
    }
}

/// Create a new agent with the given provider configuration
///
/// # Parameters
///
/// - config: Provider configuration
///
/// # Returns
///
/// A new agent pointer, or a null pointer if creation failed
///
/// # Safety
///
/// The config pointer must be valid or NULL. The resulting agent must be freed
/// with goose_agent_free when no longer needed.
#[no_mangle]
pub unsafe extern "C" fn goose_agent_new(config: *const ProviderConfigFFI) -> AgentPtr {
    // Check for null pointer
    if config.is_null() {
        eprintln!("Error: config pointer is null");
        return ptr::null_mut();
    }

    let config = &*config;

    // We currently only support Databricks provider
    // This match ensures future compiler errors if new provider types are added without handling
    match config.provider_type {
        ProviderType::Databricks => (), // Databricks provider is supported
    }

    // Get api_key from config or environment
    let api_key = if !config.api_key.is_null() {
        CStr::from_ptr(config.api_key).to_string_lossy().to_string()
    } else {
        match std::env::var("DATABRICKS_API_KEY") {
            Ok(key) => key,
            Err(_) => {
                eprintln!("Error: api_key not provided and DATABRICKS_API_KEY environment variable not set");
                return ptr::null_mut();
            }
        }
    };

    // Check and get required model_name (no env fallback for model)
    if config.model_name.is_null() {
        eprintln!("Error: model_name is required but was null");
        return ptr::null_mut();
    }
    let model_name = CStr::from_ptr(config.model_name)
        .to_string_lossy()
        .to_string();

    // Get host from config or environment
    let host = if !config.host.is_null() {
        CStr::from_ptr(config.host).to_string_lossy().to_string()
    } else {
        match std::env::var("DATABRICKS_HOST") {
            Ok(url) => url,
            Err(_) => {
                eprintln!(
                    "Error: host not provided and DATABRICKS_HOST environment variable not set"
                );
                return ptr::null_mut();
            }
        }
    };

    // Create model config with model name
    let model_config = ModelConfig::new(model_name);

    // Create Databricks provider with required parameters
    match DatabricksProvider::from_params(host, api_key, model_config) {
        Ok(provider) => {
            let agent = Agent::new(Arc::new(provider));
            Box::into_raw(Box::new(agent))
        }
        Err(e) => {
            eprintln!("Error creating Databricks provider: {:?}", e);
            ptr::null_mut()
        }
    }
}

/// Free an agent
///
/// This function frees the memory allocated for an agent.
///
/// # Parameters
///
/// - agent_ptr: Agent pointer returned by goose_agent_new
///
/// # Safety
///
/// The agent_ptr must be a valid pointer returned by goose_agent_new,
/// or have a null internal pointer. The agent_ptr must not be used after
/// calling this function.
#[no_mangle]
pub unsafe extern "C" fn goose_agent_free(agent_ptr: AgentPtr) {
    if !agent_ptr.is_null() {
        let _ = Box::from_raw(agent_ptr);
    }
}

/// Send a message to the agent and get the response
///
/// This function sends a message to the agent and returns the response.
/// Tool handling is not yet supported and will be implemented in a future commit
/// so this may change significantly
///
/// # Parameters
///
/// - agent_ptr: Agent pointer
/// - message: Message to send
///
/// # Returns
///
/// A C string with the agent's response, or NULL on error.
/// This string must be freed with goose_free_string when no longer needed.
///
/// # Safety
///
/// The agent_ptr must be a valid pointer returned by goose_agent_new.
/// The message must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn goose_agent_send_message(
    agent_ptr: AgentPtr,
    message: *const c_char,
) -> *mut c_char {
    if agent_ptr.is_null() || message.is_null() {
        return ptr::null_mut();
    }

    let agent = &mut *agent_ptr;
    let message = CStr::from_ptr(message).to_string_lossy().to_string();

    let messages = vec![Message::user().with_text(&message)];

    // Block on the async call using our global runtime
    let response = get_runtime().block_on(async {
        let mut stream = match agent.reply(&messages, None).await {
            Ok(stream) => stream,
            Err(e) => return format!("Error getting reply from agent: {}", e),
        };

        let mut full_response = String::new();

        while let Some(message_result) = stream.next().await {
            match message_result {
                Ok(message) => {
                    // Get text or serialize to JSON
                    // Note: Message doesn't have as_text method, we'll serialize to JSON
                    if let Ok(json) = serde_json::to_string(&message) {
                        full_response.push_str(&json);
                    }
                }
                Err(e) => {
                    full_response.push_str(&format!("\nError in message stream: {}", e));
                }
            }
        }
        full_response
    });

    string_to_c_char(&response)
}

// Tool schema creation will be implemented in a future commit

/// Free a string allocated by goose FFI functions
///
/// This function frees memory allocated for strings returned by goose FFI functions.
///
/// # Parameters
///
/// - s: String to free
///
/// # Safety
///
/// The string must have been allocated by a goose FFI function, or be NULL.
/// The string must not be used after calling this function.
#[no_mangle]
pub unsafe extern "C" fn goose_free_string(s: *mut c_char) {
    if !s.is_null() {
        let _ = CString::from_raw(s);
    }
}

// Helper function to convert a Rust string to a C char pointer
fn string_to_c_char(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}
