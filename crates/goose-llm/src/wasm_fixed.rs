// This is the updated implementation of wasm_complete_with_mock_async

/// Async version of the mock completion function
/// This function demonstrates how to properly handle async in WebAssembly
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub async fn wasm_complete_with_mock_async(request: WasmCompletionRequest) -> Result<WasmCompletionResponse, JsValue> {
    #[cfg(feature = "wasm")]
    {
        web_sys::console::log_1(&JsValue::from_str("Starting async mock completion..."));
        if let Ok(req_json) = request.to_json() {
            web_sys::console::log_1(&JsValue::from_str(&format!("Request: {}", req_json)));
        }
    }
    
    // Parse the mock config
    let config: MockProviderConfig = match serde_json::from_value(request.inner.provider_config.clone()) {
        Ok(config) => config,
        Err(e) => {
            let error_msg = format!("Failed to parse mock config: {}", e);
            #[cfg(feature = "wasm")]
            web_sys::console::error_1(&JsValue::from_str(&error_msg));
            return Err(JsValue::from_str(&error_msg));
        }
    };
    
    // Check if we should force an error
    if let Some(error) = &config.force_error {
        let error_msg = match error.as_str() {
            "auth" => format!("Authentication error: Mock authentication error"),
            "context" => format!("Context length exceeded: Mock context length exceeded"),
            "rate" => format!("Rate limit exceeded: Mock rate limit exceeded"),
            "server" => format!("Server error: Mock server error"),
            "request" => format!("Request failed: Mock request failed"),
            "execution" => format!("Execution error: Mock execution error"),
            "usage" => format!("Usage error: Mock usage error"),
            "parse" => format!("Response parse error: Mock parse error"),
            _ => format!("Unknown error type: {}", error),
        };
        
        #[cfg(feature = "wasm")]
        web_sys::console::error_1(&JsValue::from_str(&error_msg));
        
        return Err(JsValue::from_str(&error_msg));
    }
    
    // Simulate delay if configured
    if let Some(delay_ms) = config.delay_ms {
        #[cfg(feature = "wasm")]
        {
            use js_sys::Promise;
            use wasm_bindgen_futures::JsFuture;
            
            // Create a promise that resolves after the specified delay
            let promise = Promise::new(&mut move |resolve, _| {
                let window = web_sys::window().expect("Should have a window");
                let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                    &resolve,
                    delay_ms as i32,
                );
            });
            
            // Wait for the promise to resolve
            let _ = JsFuture::from(promise).await;
        }
    }
    
    // Get the last user message
    let user_message = request.inner.messages.iter().rev()
        .find(|m| m.role == Role::User)
        .map(|m| m.content.concat_text_str())
        .unwrap_or_else(|| "No user message found".to_string());
    
    // Create a mock response
    let response_text = format!(
        "This is a mock response to: '{}'\n\nSystem prompt was: '{}'", 
        user_message, 
        request.inner.system_preamble
    );
    
    // Create the message
    let message = Message::assistant().with_text(response_text);
    
    // Create usage statistics
    let usage = if let Some(mock_tokens) = &config.mock_tokens {
        Usage::new(
            mock_tokens.input_tokens,
            mock_tokens.output_tokens,
            mock_tokens.total_tokens,
        )
    } else {
        Usage::default()
    };
    
    // Create runtime metrics
    let delay_sec = config.delay_ms.unwrap_or(0) as f32 / 1000.0;
    let total_time = delay_sec + 0.1;
    let provider_time = delay_sec;
    let tokens_per_sec = usage.total_tokens.map(|t| t as f64 / provider_time.max(0.1) as f64);
    
    // Create the completion response
    let completion_response = crate::types::completion::CompletionResponse::new(
        message,
        request.inner.model_config.model_name.clone(),
        usage,
        crate::types::completion::RuntimeMetrics::new(
            total_time,
            provider_time,
            tokens_per_sec,
        ),
    );
    
    #[cfg(feature = "wasm")]
    web_sys::console::log_1(&JsValue::from_str("Mock completion successful"));
    
    Ok(WasmCompletionResponse { inner: completion_response })
}