/// WebAssembly provider wrappers for real providers
/// This module provides WebAssembly-compatible wrappers for the standard providers
/// like OpenAI and Databricks without duplicating their implementation.

#[cfg(all(feature = "wasm", feature = "http"))]
use wasm_bindgen::prelude::*;
use crate::wasm::{WasmMessage, WasmUsage, WasmRuntimeMetrics, WasmCompletionResponse};
use crate::providers::openai::OpenAiProviderConfig; 
use crate::providers::databricks::DatabricksProviderConfig;
use crate::providers::errors::ProviderError;
use serde_json::Value; // For json manipulation


/// WebAssembly wrapper for OpenAI provider
#[cfg(all(feature = "wasm", feature = "http"))]
#[wasm_bindgen]
pub async fn wasm_complete_with_openai(request: crate::wasm::WasmCompletionRequest) -> Result<WasmCompletionResponse, JsValue> {
    // use std::time::Instant; // Timing removed due to WASM issues
    use crate::providers::utils::handle_response_openai_compat; 
    use crate::providers::formats::openai::{create_request as openai_create_request, response_to_message, get_usage};
    use crate::providers::utils::get_model;

    #[cfg(feature = "wasm")]
    web_sys::console::log_1(&JsValue::from_str("[OpenAI WASM] Entered wasm_complete_with_openai"));
    
    if request.provider_name() != "openai" {
        let error_msg = format!("[OpenAI WASM] Invalid provider name: {}. Expected 'openai'", request.provider_name());
        #[cfg(feature = "wasm")]
        web_sys::console::error_1(&JsValue::from_str(&error_msg));
        return Err(JsValue::from_str(&error_msg));
    }
    
    let system_preamble = request.inner_system_preamble();
    let messages_json = request.inner_messages_json()?;
    let model_name_str = request.inner_model_name();
    let provider_config_json_str = request.inner_provider_config_json()?;
    
    let messages: Vec<crate::message::Message> = serde_json::from_str(&messages_json)
        .map_err(|e| JsValue::from_str(&format!("[OpenAI WASM] Failed to parse messages: {}", e)))?;
    
    let model_config = crate::model::ModelConfig::new(model_name_str.clone()).with_max_tokens(Some(2048));
    
    let client = reqwest::Client::new();
    let provider_config_obj: OpenAiProviderConfig = serde_json::from_str(&provider_config_json_str)
        .map_err(|e| JsValue::from_str(&format!("[OpenAI WASM] Failed to parse provider_config_json_str into OpenAiProviderConfig: {}", e)))?;

    let payload = openai_create_request(&model_config, &system_preamble, &messages, &[], &crate::providers::utils::ImageFormat::OpenAi)
        .map_err(|e| JsValue::from_str(&format!("[OpenAI WASM] Failed to create request payload: {}", e)))?;

    #[cfg(feature = "wasm")]
    web_sys::console::log_1(&JsValue::from_str(&format!("[OpenAI WASM] Payload created. Sending request to OpenAI... URL: {}/{}", &provider_config_obj.host, &provider_config_obj.base_path)));
        
    let base_url = url::Url::parse(&provider_config_obj.host)
        .map_err(|e: url::ParseError| JsValue::from_str(&format!("[OpenAI WASM] URL parse error: {}", e.to_string())))?;
    let url_to_request = base_url.join(&provider_config_obj.base_path)
        .map_err(|e: url::ParseError| JsValue::from_str(&format!("[OpenAI WASM] URL join error: {}", e.to_string())))?;
    #[cfg(feature = "wasm")]
    web_sys::console::log_1(&JsValue::from_str(&format!("[OpenAI WASM] Final url_to_request: {}", url_to_request.as_str())));

    let mut request_builder = client
        .post(url_to_request.clone())
        .header("Authorization", format!("Bearer {}", provider_config_obj.api_key));

    if let Some(org) = &provider_config_obj.organization {
        request_builder = request_builder.header("OpenAI-Organization", org);
    }
    if let Some(project) = &provider_config_obj.project {
        request_builder = request_builder.header("OpenAI-Project", project);
    }
    if let Some(custom_headers) = &provider_config_obj.custom_headers {
        for (key, value) in custom_headers {
            request_builder = request_builder.header(key, value);
        }
    }
    
    let final_request_builder = request_builder.json(&payload);
    
    #[cfg(feature = "wasm")]
    web_sys::console::log_1(&JsValue::from_str("[OpenAI WASM] About to .send().await..."));

    let http_response = match final_request_builder.send().await {
        Ok(resp) => resp,
        Err(reqwest_error) => {
            let error_msg = format!("[OpenAI WASM] reqwest .send().await error: {}", reqwest_error);
            #[cfg(feature = "wasm")]
            web_sys::console::error_1(&JsValue::from_str(&error_msg));
            return Err(JsValue::from_str(&error_msg));
        }
    };
    
    #[cfg(feature = "wasm")]
    web_sys::console::log_1(&JsValue::from_str(&format!("[OpenAI WASM] HTTP request .send().await finished. Status: {}.", http_response.status())));
    
    match handle_response_openai_compat(http_response).await {
        Ok(json_response) => {
            #[cfg(feature = "wasm")]
            web_sys::console::log_1(&JsValue::from_str("[OpenAI WASM] Response handled by handle_response_openai_compat. Processing..."));

            let resp_message = response_to_message(json_response.clone())
                .map_err(|e| JsValue::from_str(&format!("[OpenAI WASM] Failed to parse message from response: {}", e)))?;
            let usage = get_usage(&json_response)
                .map_err(|e: ProviderError| JsValue::from_str(&format!("[OpenAI WASM] Failed to get usage from response: {}", e.to_string())))?;
            let model_name_from_response = get_model(&json_response);

            let wasm_message = WasmMessage::from_json(&serde_json::to_string(&resp_message).map_err(|e| JsValue::from_str(&format!("[OpenAI WASM] Failed to serialize final message to JSON: {}",e)))?)?;
            let wasm_usage = WasmUsage::new(usage.input_tokens, usage.output_tokens, usage.total_tokens);
            let wasm_metrics = WasmRuntimeMetrics::new(0.0, 0.0, None); // Timing removed
            
            #[cfg(feature = "wasm")]
            web_sys::console::log_1(&JsValue::from_str("[OpenAI WASM] OpenAI completion successful. Returning WasmCompletionResponse."));
            
            Ok(WasmCompletionResponse::new(wasm_message, model_name_from_response, wasm_usage, wasm_metrics))
        }
        Err(provider_error) => { 
            let error_msg = format!("[OpenAI WASM] Error from handle_response_openai_compat: {}", provider_error.to_string());
            #[cfg(feature = "wasm")]
            web_sys::console::error_1(&JsValue::from_str(&error_msg));
            Err(JsValue::from_str(&error_msg))
        }
    }
}

/// WebAssembly wrapper for Databricks provider
#[cfg(all(feature = "wasm", feature = "http"))]
#[wasm_bindgen]
pub async fn wasm_complete_with_databricks(request: crate::wasm::WasmCompletionRequest) -> Result<WasmCompletionResponse, JsValue> {
    use crate::providers::formats::databricks::{create_request as databricks_create_request, response_to_message as databricks_response_to_message, get_usage as databricks_get_usage};
    use crate::providers::utils::get_model;
    use reqwest::StatusCode; 

    #[cfg(feature = "wasm")]
    web_sys::console::log_1(&JsValue::from_str("[Databricks WASM] Entered wasm_complete_with_databricks"));

    if request.provider_name() != "databricks" {
        let error_msg = format!("[Databricks WASM] Invalid provider name: {}. Expected 'databricks'", request.provider_name());
        #[cfg(feature = "wasm")]
        web_sys::console::error_1(&JsValue::from_str(&error_msg));
        return Err(JsValue::from_str(&error_msg));
    }

    let system_preamble = request.inner_system_preamble();
    let messages_json = request.inner_messages_json()?;
    let model_name_str = request.inner_model_name(); 
    let provider_config_json_str = request.inner_provider_config_json()?;

    let messages: Vec<crate::message::Message> = serde_json::from_str(&messages_json)
        .map_err(|e| JsValue::from_str(&format!("[Databricks WASM] Failed to parse messages: {}", e)))?;
    
    // model_name_str from request is used for the URL path for Databricks serving endpoints
    let model_config = crate::model::ModelConfig::new(model_name_str.clone()).with_max_tokens(Some(2048));
    
    let client = reqwest::Client::new();
    let provider_config_obj: DatabricksProviderConfig = serde_json::from_str(&provider_config_json_str)
        .map_err(|e| JsValue::from_str(&format!("[Databricks WASM] Failed to parse provider_config_json_str into DatabricksProviderConfig: {}", e)))?;

    // Databricks create_request uses model_config to potentially adjust payload based on model type, but model name itself for URL is separate.
    let mut payload = databricks_create_request(&model_config, &system_preamble, &messages, &[], &provider_config_obj.image_format)
        .map_err(|e| JsValue::from_str(&format!("[Databricks WASM] Failed to create request payload: {}", e)))?;
    
    if let Some(obj) = payload.as_object_mut() {
        obj.remove("model"); // Databricks model name for serving endpoints is in URL path, not payload
    }

    #[cfg(feature = "wasm")]
    web_sys::console::log_1(&JsValue::from_str(&format!("[Databricks WASM] Payload created. Sending request to Databricks... Host: {}, Model for URL: {}", &provider_config_obj.host, &model_config.model_name)));
        
    let base_url = url::Url::parse(&provider_config_obj.host)
        .map_err(|e: url::ParseError| JsValue::from_str(&format!("[Databricks WASM] URL parse error: {}", e.to_string())))?;
    let path = format!("serving-endpoints/{}/invocations", &model_config.model_name); // model_name from request used here
    let url_to_request = base_url.join(&path)
        .map_err(|e: url::ParseError| JsValue::from_str(&format!("[Databricks WASM] URL join error: {}", e.to_string())))?;
    #[cfg(feature = "wasm")]
    web_sys::console::log_1(&JsValue::from_str(&format!("[Databricks WASM] Final url_to_request: {}", url_to_request.as_str())));

    let request_builder = client
        .post(url_to_request.clone())
        .header("Authorization", format!("Bearer {}", provider_config_obj.token))
        .json(&payload);
    
    #[cfg(feature = "wasm")]
    web_sys::console::log_1(&JsValue::from_str("[Databricks WASM] About to .send().await..."));

    let http_response = match request_builder.send().await {
        Ok(resp) => resp,
        Err(reqwest_error) => {
            let error_msg = format!("[Databricks WASM] reqwest .send().await error: {}", reqwest_error);
            #[cfg(feature = "wasm")]
            web_sys::console::error_1(&JsValue::from_str(&error_msg));
            return Err(JsValue::from_str(&error_msg));
        }
    };

    #[cfg(feature = "wasm")]
    web_sys::console::log_1(&JsValue::from_str(&format!("[Databricks WASM] HTTP request .send().await finished. Status: {}.", http_response.status())));
    
    let status = http_response.status();
    let response_json_value_opt: Option<Value> = match http_response.json().await {
        Ok(val) => Some(val),
        Err(e) => {
            if status.is_success() {
                let error_msg = format!("[Databricks WASM] Failed to parse JSON from (status {}) response: {}", status, e.to_string());
                #[cfg(feature = "wasm")]
                web_sys::console::error_1(&JsValue::from_str(&error_msg));
                return Err(JsValue::from_str(&error_msg)); 
            }
            None 
        }
    };

    match status {
        StatusCode::OK => {
            match response_json_value_opt {
                Some(json_response) => {
                    let resp_message = databricks_response_to_message(json_response.clone())
                        .map_err(|e| JsValue::from_str(&format!("[Databricks WASM] Failed to parse message from response: {}", e)))?;
                    let usage = databricks_get_usage(&json_response)
                        .map_err(|e: ProviderError| JsValue::from_str(&format!("[Databricks WASM] Failed to get usage from response: {}", e.to_string())))?;
                    let model_name_from_response = get_model(&json_response); 

                    let wasm_message = WasmMessage::from_json(&serde_json::to_string(&resp_message).map_err(|e| JsValue::from_str(&format!("[Databricks WASM] Failed to serialize final message to JSON: {}",e)))?)?;
                    let wasm_usage = WasmUsage::new(usage.input_tokens, usage.output_tokens, usage.total_tokens);
                    let wasm_metrics = WasmRuntimeMetrics::new(0.0, 0.0, None); 
                    
                    #[cfg(feature = "wasm")]
                    web_sys::console::log_1(&JsValue::from_str("[Databricks WASM] Databricks completion successful. Returning WasmCompletionResponse."));
                    Ok(WasmCompletionResponse::new(wasm_message, model_name_from_response, wasm_usage, wasm_metrics))
                }
                None => {
                    let error_msg = "[Databricks WASM] 200 OK but failed to get JSON body (or body was not expected for error status).";
                    web_sys::console::error_1(&JsValue::from_str(error_msg));
                    Err(JsValue::from_str(error_msg))
                }
            }
        }
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            let err_detail = response_json_value_opt.map(|v| serde_json::to_string(&v).unwrap_or_default()).unwrap_or_else(|| "No details".to_string());
            Err(JsValue::from_str(&format!("[Databricks WASM] Authentication failed. Status: {}. Details: {}", status, err_detail)))
        }
        StatusCode::BAD_REQUEST => {
            let err_detail = response_json_value_opt.map(|v| serde_json::to_string(&v).unwrap_or_default()).unwrap_or_else(|| "No details".to_string());
            if err_detail.to_lowercase().contains("context length") {
                 Err(JsValue::from_str(&format!("[Databricks WASM] Context length exceeded. Status: {}. Details: {}", status, err_detail)))
            } else {
                 Err(JsValue::from_str(&format!("[Databricks WASM] Bad Request. Status: {}. Details: {}", status, err_detail)))
            }
        }
        StatusCode::TOO_MANY_REQUESTS => Err(JsValue::from_str(&format!("[Databricks WASM] Rate limit exceeded. Status: {}. Details: {:?}", status, response_json_value_opt))),
        _ => Err(JsValue::from_str(&format!("[Databricks WASM] Request failed with status: {}. Details: {:?}", status, response_json_value_opt))),
    }
}

/// Generic WebAssembly wrapper for any provider
/// This function will route to the appropriate provider based on the request's provider_name
#[cfg(all(feature = "wasm", feature = "http"))]
#[wasm_bindgen]
pub async fn wasm_complete_with_provider(request: crate::wasm::WasmCompletionRequest) -> Result<WasmCompletionResponse, JsValue> { 
    match request.provider_name().as_str() {
        "openai" => {
            wasm_complete_with_openai(request).await 
        }
        "databricks" => wasm_complete_with_databricks(request).await, // Re-enabled Databricks path
        "mock" => crate::wasm::wasm_complete_with_mock_async(request).await,
        _ => {
            let error_msg = format!("[WASM Provider Router] Unsupported provider: {}", request.provider_name());
            #[cfg(feature = "wasm")]
            web_sys::console::error_1(&JsValue::from_str(&error_msg));
            Err(JsValue::from_str(&error_msg))
        }
    }
}


/// Helper function to create a provider configuration for OpenAI
#[cfg(all(feature = "wasm", feature = "http"))]
#[wasm_bindgen]
pub fn wasm_create_openai_config(api_key: String, base_url: Option<String>) -> Result<String, JsValue> {
    let config = serde_json::json!({
        "api_key": api_key,
        "host": base_url.unwrap_or_else(|| "https://api.openai.com".to_string()),
        "base_path": "v1/chat/completions",
        "timeout": 60,
    });
    
    serde_json::to_string(&config)
        .map_err(|e| JsValue::from_str(&format!("[OpenAI WASM] Failed to serialize OpenAI config: {}", e)))
}

/// Helper function to create a provider configuration for Databricks
#[cfg(all(feature = "wasm", feature = "http"))]
#[wasm_bindgen]
pub fn wasm_create_databricks_config(api_key: String, host: String, _endpoint_name: String) -> Result<String, JsValue> { 
    let config = serde_json::json!({
        "token": api_key, 
        "host": host,
        "image_format": "OpenAi", // Corrected to PascalCase
        "timeout": 60,
    });
    
    serde_json::to_string(&config)
        .map_err(|e| JsValue::from_str(&format!("[Databricks WASM] Failed to serialize Databricks config: {}", e)))
}