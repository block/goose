use futures::StreamExt;
use goose::agents::AgentFactory;
use goose::message::Message;
use goose::model::ModelConfig;
use goose::providers::databricks::{DatabricksProvider, DatabricksAuth};
use libc::c_char;
use once_cell::sync::Lazy;
use std::ffi::{CStr, CString};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use std::time::Duration;
use anyhow::Result;

// Global runtime for async operations
static RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    Runtime::new().expect("Failed to create Tokio runtime")
});

// Global agent initialization flag
static AGENT_INITIALIZED: Lazy<Arc<Mutex<bool>>> = Lazy::new(|| Arc::new(Mutex::new(false)));

// Helper to convert Rust string to C string
fn to_c_string(s: &str) -> *mut c_char {
    CString::new(s).unwrap().into_raw()
}

// Helper to convert C string to Rust string
unsafe fn from_c_string(s: *const c_char) -> String {
    CStr::from_ptr(s).to_string_lossy().into_owned()
}

// Custom function to create a DatabricksProvider with our own configuration
fn create_databricks_provider(model_name: &str, token: &str) -> Result<DatabricksProvider> {
    // Create a provider with Databricks host and the provided token
    let host = "https://block-lakehouse-staging.cloud.databricks.com/";
    
    // Create auth with the token
    let _auth = DatabricksAuth::token(token.to_string());
    
    // Use the specified model
    let model = ModelConfig::new(model_name.to_string());
    
    // Create the client
    let _client = reqwest::Client::builder()
        .timeout(Duration::from_secs(600))
        .build()?;
    
    // Set environment variables for the from_env method
    std::env::set_var("DATABRICKS_HOST", host);
    std::env::set_var("DATABRICKS_TOKEN", token);
    
    // Use from_env to create the provider
    DatabricksProvider::from_env(model)
}

// Original C FFI functions (kept for backward compatibility)
#[no_mangle]
pub extern "C" fn goose_initialize(token: *const c_char) -> bool {
    if token.is_null() {
        return false;
    }
    
    let token_str = unsafe { from_c_string(token) };
    
    let result = RUNTIME.block_on(async {
        // Create the provider
        let provider = match create_databricks_provider("databricks-claude-3-7-sonnet", &token_str) {
            Ok(provider) => provider,
            Err(_) => return false,
        };
        
        // Check if we can create an agent
        let agent = match AgentFactory::create("reference", Box::new(provider)) {
            Some(agent) => agent,
            None => return false,
        };
        
        // We don't need to keep the agent, just check if it can be created
        drop(agent);
        
        // Mark as initialized
        let mut initialized = AGENT_INITIALIZED.lock().unwrap();
        *initialized = true;
        
        true
    });
    
    result
}

#[no_mangle]
pub extern "C" fn goose_send_message(message: *const c_char, token: *const c_char) -> *mut c_char {
    if message.is_null() {
        return to_c_string("{\"error\": \"Message is null\"}");
    }
    
    if token.is_null() {
        return to_c_string("{\"error\": \"Token is null\"}");
    }
    
    let message_str = unsafe { from_c_string(message) };
    let token_str = unsafe { from_c_string(token) };
    
    let result = RUNTIME.block_on(async {
        // Check if initialized
        let initialized = AGENT_INITIALIZED.lock().unwrap();
        if !*initialized {
            return "{\"error\": \"Agent not initialized\"}".to_string();
        }
        
        // Create the provider
        let provider = match create_databricks_provider("databricks-claude-3-7-sonnet", &token_str) {
            Ok(provider) => provider,
            Err(e) => return format!("{{\"error\": \"Failed to create provider: {}\"}}", e),
        };
        
        let agent = match AgentFactory::create("reference", Box::new(provider)) {
            Some(agent) => agent,
            None => return "{\"error\": \"Failed to create agent\"}".to_string(),
        };
        
        let messages = vec![Message::user().with_text(&message_str)];
        
        // Create a scope to ensure agent lives long enough
        let response = {
            let reply_result = agent.reply(&messages, None).await;
            
            match reply_result {
                Ok(mut stream) => {
                    let mut responses = Vec::new();
                    
                    while let Some(message_result) = stream.next().await {
                        match message_result {
                            Ok(msg) => {
                                if let Ok(json) = serde_json::to_string(&msg) {
                                    responses.push(json);
                                }
                            }
                            Err(e) => {
                                return format!("{{\"error\": \"{}\"}}", e);
                            }
                        }
                    }
                    
                    format!("{{\"responses\": [{}]}}", responses.join(", "))
                }
                Err(e) => {
                    format!("{{\"error\": \"{}\"}}", e)
                }
            }
        };
        
        response
    });
    
    to_c_string(&result)
}

#[no_mangle]
pub extern "C" fn goose_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

#[no_mangle]
pub extern "C" fn goose_shutdown() -> bool {
    let mut initialized = AGENT_INITIALIZED.lock().unwrap();
    *initialized = false;
    true
}