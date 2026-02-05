//! Axum HTTP server handling OpenAI Chat Completions requests.

use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use bytes::Bytes;
use futures::StreamExt;
use serde_json::Value;
use std::sync::Arc;
use tracing::{error, info};

use crate::auth::Auth;
use crate::converter::{build_custom_request, build_openai_request_with_tools, custom_response_to_openai};
use crate::models::{CustomLlmResponse, OpenAiChatRequest, ProxyConfig, ProxyMode};
use crate::stream::{
    build_streaming_error_response, collect_sse_chunks_from_bytes,
    generate_buffered_structured_sse, generate_buffered_tools_sse, generate_plain_sse,
    generate_openai_buffered_tools_sse,
};

// Note: collect_sse_chunks_from_bytes and some others may be unused in OpenAI mode
// They are kept for Fabrix mode compatibility

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ProxyConfig>,
    pub http_client: reqwest::Client,
}

/// POST /v1/chat/completions handler.
pub async fn chat_completions_handler(
    State(state): State<AppState>,
    Json(request): Json<OpenAiChatRequest>,
) -> Result<Response, StatusCode> {
    let has_tools = request
        .tools
        .as_ref()
        .map(|t| !t.is_empty())
        .unwrap_or(false);
    let is_stream = request.stream.unwrap_or(false);

    let response_format = request.response_format.clone();
    let has_structured_output = response_format
        .as_ref()
        .and_then(|rf| rf.get("type"))
        .and_then(|t| t.as_str())
        .map(|t| t == "json_schema" || t == "json_object")
        .unwrap_or(false);

    let config = &state.config;

    // If force_non_stream is enabled, treat as non-streaming but wrap response in SSE format
    let client_wants_stream = is_stream;
    let is_stream = if config.force_non_stream { false } else { is_stream };

    info!(
        "Incoming request: model={}, messages={}, tools={}, stream={} (client_wants={}, force_non_stream={})",
        request.model.as_deref().unwrap_or("unknown"),
        request.messages.len(),
        has_tools,
        is_stream,
        client_wants_stream,
        config.force_non_stream,
    );

    // Parse auth (auto-detects packed vs simple format)
    let auth = match Auth::from_api_key(&config.api_key) {
        Ok(auth) => auth,
        Err(e) => {
            error!("Failed to parse API key: {}", e);
            return Ok(error_json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Invalid API key configuration: {}", e),
            ));
        }
    };

    let mut headers = HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());
    for (name, value) in auth.to_headers() {
        if let (Ok(hn), Ok(hv)) = (name.parse::<HeaderName>(), value.parse::<HeaderValue>()) {
            headers.insert(hn, hv);
        }
    }

    // Route based on proxy mode
    match config.mode {
        ProxyMode::OpenAi => {
            // OpenAI proxy mode: Keep OpenAI format, only inject tools into messages
            // Note: force_non_stream is handled in build_openai_request_with_tools
            let openai_body = build_openai_request_with_tools(&request, config);

            info!(
                "Forwarding to OpenAI-compatible LLM: url={}, model={}, messages={}, stream={}",
                config.llm_url,
                config.llm_id,
                request.messages.len(),
                is_stream,
            );

            if is_stream {
                handle_openai_streaming_request(&state, &headers, &openai_body, &request, has_tools)
                    .await
            } else {
                // Non-streaming request
                let response = handle_openai_non_streaming_request(&state, &headers, &openai_body, &request, has_tools)
                    .await?;

                // If client wanted streaming but we forced non-stream, wrap response in SSE
                if client_wants_stream {
                    Ok(wrap_json_response_as_sse(response).await)
                } else {
                    Ok(response)
                }
            }
        }
        ProxyMode::Fabrix => {
            // Fabrix mode: Convert OpenAI → Custom format
            // Note: force_non_stream is handled in build_custom_request
            let custom_body = build_custom_request(&request, config);

            info!(
                "Forwarding to Fabrix LLM: url={}, contents={}, llmId={}, isStream={}",
                config.llm_url,
                custom_body.contents.len(),
                custom_body.llm_id,
                custom_body.is_stream,
            );

            if is_stream {
                handle_streaming_request(
                    &state,
                    &headers,
                    &custom_body,
                    &request,
                    has_tools,
                    has_structured_output,
                )
                .await
            } else {
                // Non-streaming request
                let response = handle_non_streaming_request(&state, &headers, &custom_body, &request).await?;

                // If client wanted streaming but we forced non-stream, wrap response in SSE
                if client_wants_stream {
                    Ok(wrap_json_response_as_sse(response).await)
                } else {
                    Ok(response)
                }
            }
        }
    }
}

/// Handle a non-streaming request.
async fn handle_non_streaming_request(
    state: &AppState,
    headers: &HeaderMap,
    custom_body: &crate::models::CustomLlmRequest,
    openai_request: &OpenAiChatRequest,
) -> Result<Response, StatusCode> {
    let response = match state
        .http_client
        .post(&state.config.llm_url)
        .headers(headers.clone())
        .json(custom_body)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            if e.is_connect() {
                error!("Failed to connect to custom LLM at {}", state.config.llm_url);
                return Ok(error_json_response(
                    StatusCode::BAD_GATEWAY,
                    &format!("Failed to connect to custom LLM at {}", state.config.llm_url),
                ));
            }
            if e.is_timeout() {
                error!("Timeout connecting to custom LLM");
                return Ok(error_json_response(
                    StatusCode::GATEWAY_TIMEOUT,
                    "Custom LLM request timed out",
                ));
            }
            error!("HTTP request error: {}", e);
            return Ok(error_json_response(
                StatusCode::BAD_GATEWAY,
                &format!("HTTP request error: {}", e),
            ));
        }
    };

    let status = response.status();
    if !status.is_success() {
        let body_text = response.text().await.unwrap_or_default();
        error!("Custom LLM returned {}: {}", status, &body_text[..body_text.len().min(500)]);
        return Ok(error_json_response(
            StatusCode::BAD_GATEWAY,
            &format!("Custom LLM returned HTTP {}", status),
        ));
    }

    let custom_response: CustomLlmResponse = match response.json().await {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to parse custom LLM response: {}", e);
            return Ok(error_json_response(
                StatusCode::BAD_GATEWAY,
                &format!("Failed to parse custom LLM response: {}", e),
            ));
        }
    };

    info!(
        "Custom LLM response: status={}, content_length={}",
        custom_response.status.as_deref().unwrap_or("unknown"),
        custom_response.content.as_ref().map(|c| c.len()).unwrap_or(0),
    );

    let openai_response = custom_response_to_openai(&custom_response, openai_request);

    if openai_response.get("error").is_some() {
        return Ok((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(openai_response),
        )
            .into_response());
    }

    Ok(Json(openai_response).into_response())
}

/// Handle a streaming request.
async fn handle_streaming_request(
    state: &AppState,
    headers: &HeaderMap,
    custom_body: &crate::models::CustomLlmRequest,
    openai_request: &OpenAiChatRequest,
    has_tools: bool,
    has_structured_output: bool,
) -> Result<Response, StatusCode> {
    let chunk_id = format!("chatcmpl-{}", &uuid::Uuid::new_v4().as_simple().to_string()[..12]);
    let model = openai_request
        .model
        .clone()
        .unwrap_or_else(|| "custom-llm".to_string());

    // Make the streaming request
    let response = match state
        .http_client
        .post(&state.config.llm_url)
        .headers(headers.clone())
        .json(custom_body)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            let error_msg = if e.is_connect() {
                format!("Failed to connect to custom LLM at {}", state.config.llm_url)
            } else if e.is_timeout() {
                "Custom LLM request timed out".to_string()
            } else {
                format!("HTTP request error: {}", e)
            };
            error!("{}", error_msg);
            let sse_chunks = build_streaming_error_response(&error_msg);
            return Ok(sse_response(sse_chunks));
        }
    };

    let status = response.status();
    if !status.is_success() {
        error!("Custom LLM streaming returned {}", status);
        let sse_chunks =
            build_streaming_error_response(&format!("Custom LLM returned HTTP {}", status));
        return Ok(sse_response(sse_chunks));
    }

    // For buffered strategies (tools or structured output), we need to collect all chunks first
    if has_tools || has_structured_output {
        let mut all_bytes: Vec<Bytes> = Vec::new();
        let mut byte_stream = response.bytes_stream();
        while let Some(chunk_result) = byte_stream.next().await {
            match chunk_result {
                Ok(bytes) => all_bytes.push(bytes),
                Err(e) => {
                    error!("Error reading stream: {}", e);
                    let sse_chunks =
                        build_streaming_error_response(&format!("Stream read error: {}", e));
                    return Ok(sse_response(sse_chunks));
                }
            }
        }

        let parsed_chunks = collect_sse_chunks_from_bytes(all_bytes).await;

        let sse_output = if has_tools {
            generate_buffered_tools_sse(&parsed_chunks, &chunk_id, &model)
        } else {
            generate_buffered_structured_sse(&parsed_chunks, &chunk_id, &model)
        };

        Ok(sse_response(sse_output))
    } else {
        // Plain streaming: parse and forward incrementally
        let mut all_bytes: Vec<Bytes> = Vec::new();
        let mut byte_stream = response.bytes_stream();
        while let Some(chunk_result) = byte_stream.next().await {
            match chunk_result {
                Ok(bytes) => all_bytes.push(bytes),
                Err(e) => {
                    error!("Error reading stream: {}", e);
                    let sse_chunks =
                        build_streaming_error_response(&format!("Stream read error: {}", e));
                    return Ok(sse_response(sse_chunks));
                }
            }
        }

        let parsed_chunks = collect_sse_chunks_from_bytes(all_bytes).await;
        let sse_output = generate_plain_sse(&parsed_chunks, &chunk_id, &model);
        Ok(sse_response(sse_output))
    }
}

/// GET /v1/models handler.
pub async fn list_models_handler(State(state): State<AppState>) -> Json<Value> {
    Json(serde_json::json!({
        "object": "list",
        "data": [{
            "id": state.config.llm_id,
            "object": "model",
            "created": 0,
            "owned_by": "custom",
        }]
    }))
}

// ---------------------------------------------------------------------------
// OpenAI Proxy Mode Handlers
// ---------------------------------------------------------------------------

/// Handle a non-streaming request in OpenAI proxy mode.
/// Forwards request as OpenAI format with tool injection.
async fn handle_openai_non_streaming_request(
    state: &AppState,
    headers: &HeaderMap,
    openai_body: &Value,
    _openai_request: &OpenAiChatRequest,
    has_tools: bool,
) -> Result<Response, StatusCode> {
    let response = match state
        .http_client
        .post(&state.config.llm_url)
        .headers(headers.clone())
        .json(openai_body)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            if e.is_connect() {
                error!("Failed to connect to OpenAI-compatible LLM at {}", state.config.llm_url);
                return Ok(error_json_response(
                    StatusCode::BAD_GATEWAY,
                    &format!("Failed to connect to LLM at {}", state.config.llm_url),
                ));
            }
            if e.is_timeout() {
                error!("Timeout connecting to OpenAI-compatible LLM");
                return Ok(error_json_response(
                    StatusCode::GATEWAY_TIMEOUT,
                    "LLM request timed out",
                ));
            }
            error!("HTTP request error: {}", e);
            return Ok(error_json_response(
                StatusCode::BAD_GATEWAY,
                &format!("HTTP request error: {}", e),
            ));
        }
    };

    let status = response.status();
    if !status.is_success() {
        let body_text = response.text().await.unwrap_or_default();
        error!("OpenAI-compatible LLM returned {}: {}", status, &body_text[..body_text.len().min(500)]);
        return Ok(error_json_response(
            StatusCode::BAD_GATEWAY,
            &format!("LLM returned HTTP {}", status),
        ));
    }

    let mut openai_response: Value = match response.json().await {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to parse OpenAI-compatible response: {}", e);
            return Ok(error_json_response(
                StatusCode::BAD_GATEWAY,
                &format!("Failed to parse LLM response: {}", e),
            ));
        }
    };

    // Always try to parse tool calls from response (no has_tools hint from LLM)
    // Try content first, then reasoning field
    if let Some(choices) = openai_response.get_mut("choices").and_then(|c| c.as_array_mut()) {
        for choice in choices {
            if let Some(message) = choice.get_mut("message") {
                let content = message.get("content").and_then(|c| c.as_str()).unwrap_or("");

                // Try parsing <tool_call> from content first
                let (tool_calls_opt, cleaned_content) = crate::tool_injection::parse_tool_calls(content);

                if let Some(ref tool_calls) = tool_calls_opt {
                    if !tool_calls.is_empty() {
                        // Found tool_calls in content
                        message["tool_calls"] = serde_json::json!(tool_calls);
                        message["content"] = serde_json::json!(cleaned_content.trim());
                    }
                } else {
                    // No tool_calls in content, try reasoning field (some models like gpt-oss put tool_calls there)
                    if let Some(reasoning) = message.get("reasoning").and_then(|r| r.as_str()) {
                        let (reasoning_tool_calls_opt, _) = crate::tool_injection::parse_tool_calls(reasoning);
                        if let Some(tool_calls) = reasoning_tool_calls_opt {
                            if !tool_calls.is_empty() {
                                // Found tool_calls in reasoning
                                message["tool_calls"] = serde_json::json!(tool_calls);
                                // Keep original content as-is, remove reasoning field
                                message.as_object_mut().map(|m| m.remove("reasoning"));
                            }
                        }
                    }
                }
            }
        }
    }

    info!(
        "OpenAI-compatible LLM response received, choices={}",
        openai_response.get("choices").and_then(|c| c.as_array()).map(|a| a.len()).unwrap_or(0),
    );

    Ok(Json(openai_response).into_response())
}

/// Handle a streaming request in OpenAI proxy mode.
/// Forwards request as OpenAI format with tool injection.
async fn handle_openai_streaming_request(
    state: &AppState,
    headers: &HeaderMap,
    openai_body: &Value,
    openai_request: &OpenAiChatRequest,
    has_tools: bool,
) -> Result<Response, StatusCode> {
    let chunk_id = format!("chatcmpl-{}", &uuid::Uuid::new_v4().as_simple().to_string()[..12]);
    let model = openai_request
        .model
        .clone()
        .unwrap_or_else(|| "custom-llm".to_string());

    // Make the streaming request
    let response = match state
        .http_client
        .post(&state.config.llm_url)
        .headers(headers.clone())
        .json(openai_body)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            let error_msg = if e.is_connect() {
                format!("Failed to connect to LLM at {}", state.config.llm_url)
            } else if e.is_timeout() {
                "LLM request timed out".to_string()
            } else {
                format!("HTTP request error: {}", e)
            };
            error!("{}", error_msg);
            let sse_chunks = build_streaming_error_response(&error_msg);
            return Ok(sse_response(sse_chunks));
        }
    };

    let status = response.status();
    if !status.is_success() {
        let body_text = response.text().await.unwrap_or_default();
        let error_msg = format!("LLM returned HTTP {}", status);
        error!("{}: {}", error_msg, &body_text[..body_text.len().min(500)]);
        let sse_chunks = build_streaming_error_response(&error_msg);
        return Ok(sse_response(sse_chunks));
    }

    // If tools are present, buffer the stream and parse tool calls
    if has_tools {
        let mut all_bytes: Vec<Bytes> = Vec::new();
        let mut byte_stream = response.bytes_stream();
        while let Some(chunk_result) = byte_stream.next().await {
            match chunk_result {
                Ok(bytes) => {
                    all_bytes.push(bytes);
                }
                Err(e) => {
                    error!("Error reading stream: {}", e);
                    let sse_chunks =
                        build_streaming_error_response(&format!("Stream read error: {}", e));
                    return Ok(sse_response(sse_chunks));
                }
            }
        }

        let sse_output = generate_openai_buffered_tools_sse(all_bytes, &chunk_id, &model);
        return Ok(sse_response(sse_output));
    }

    // No tools - pass through the SSE stream directly
    let stream = response
        .bytes_stream()
        .map(|result| result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)));

    let body = Body::from_stream(stream);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/event-stream")
        .header("cache-control", "no-cache")
        .header("connection", "keep-alive")
        .header("x-accel-buffering", "no")
        .body(body)
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Internal error"))
                .unwrap()
        }))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Wrap a non-streaming JSON response as SSE format for clients expecting streaming.
///
/// Converts an OpenAI chat completion response into SSE chunks:
/// 1. Role chunk (assistant)
/// 2. Content chunk(s)
/// 3. Tool call chunks (if any)
/// 4. Finish chunk with usage
/// 5. [DONE] signal
async fn wrap_json_response_as_sse(response: Response) -> Response {
    use axum::body::to_bytes;

    // Extract body from response
    let (parts, body) = response.into_parts();

    // If the original response was an error, return as-is
    if !parts.status.is_success() {
        return Response::from_parts(parts, body);
    }

    // Read body bytes
    let bytes = match to_bytes(body, 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => {
            return error_json_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to read response body");
        }
    };

    // Parse as JSON
    let json_response: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(j) => j,
        Err(_) => {
            return error_json_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to parse response JSON");
        }
    };

    // Check for error response
    if json_response.get("error").is_some() {
        // Return error as SSE
        let error_chunk = format!("data: {}\n\ndata: [DONE]\n\n", serde_json::to_string(&json_response).unwrap_or_default());
        return sse_response(vec![error_chunk]);
    }

    // Extract data from OpenAI response
    let chunk_id = json_response.get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("chatcmpl-unknown")
        .to_string();
    let model = json_response.get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let created = json_response.get("created")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let choice = json_response.get("choices")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .cloned()
        .unwrap_or(serde_json::json!({}));

    let message = choice.get("message").cloned().unwrap_or(serde_json::json!({}));
    let content = message.get("content").and_then(|c| c.as_str()).unwrap_or("");
    let tool_calls = message.get("tool_calls").and_then(|t| t.as_array());
    let finish_reason = choice.get("finish_reason")
        .and_then(|f| f.as_str())
        .unwrap_or("stop");
    let usage = json_response.get("usage").cloned();

    let mut chunks = Vec::new();

    // 1. Role chunk
    chunks.push(format!(
        "data: {}\n\n",
        serde_json::json!({
            "id": chunk_id,
            "object": "chat.completion.chunk",
            "created": created,
            "model": model,
            "choices": [{
                "index": 0,
                "delta": {"role": "assistant"},
                "finish_reason": null
            }]
        })
    ));

    // 2. Content chunk (if any)
    if !content.is_empty() {
        chunks.push(format!(
            "data: {}\n\n",
            serde_json::json!({
                "id": chunk_id,
                "object": "chat.completion.chunk",
                "created": created,
                "model": model,
                "choices": [{
                    "index": 0,
                    "delta": {"content": content},
                    "finish_reason": null
                }]
            })
        ));
    }

    // 3. Tool call chunks (if any)
    if let Some(calls) = tool_calls {
        for (idx, call) in calls.iter().enumerate() {
            let func = call.get("function").unwrap_or(call);
            let name = func.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let arguments = func.get("arguments").and_then(|a| a.as_str()).unwrap_or("{}");
            let call_id = call.get("id").and_then(|i| i.as_str()).unwrap_or("");

            // First chunk: function name
            chunks.push(format!(
                "data: {}\n\n",
                serde_json::json!({
                    "id": chunk_id,
                    "object": "chat.completion.chunk",
                    "created": created,
                    "model": model,
                    "choices": [{
                        "index": 0,
                        "delta": {
                            "tool_calls": [{
                                "index": idx,
                                "id": call_id,
                                "type": "function",
                                "function": {"name": name, "arguments": ""}
                            }]
                        },
                        "finish_reason": null
                    }]
                })
            ));

            // Second chunk: arguments
            chunks.push(format!(
                "data: {}\n\n",
                serde_json::json!({
                    "id": chunk_id,
                    "object": "chat.completion.chunk",
                    "created": created,
                    "model": model,
                    "choices": [{
                        "index": 0,
                        "delta": {
                            "tool_calls": [{
                                "index": idx,
                                "function": {"arguments": arguments}
                            }]
                        },
                        "finish_reason": null
                    }]
                })
            ));
        }
    }

    // 4. Finish chunk with usage
    let mut finish_chunk = serde_json::json!({
        "id": chunk_id,
        "object": "chat.completion.chunk",
        "created": created,
        "model": model,
        "choices": [{
            "index": 0,
            "delta": {},
            "finish_reason": finish_reason
        }]
    });
    if let Some(u) = usage {
        finish_chunk["usage"] = u;
    }
    chunks.push(format!("data: {}\n\n", finish_chunk));

    // 5. [DONE] signal
    chunks.push("data: [DONE]\n\n".to_string());

    sse_response(chunks)
}

fn error_json_response(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(serde_json::json!({
            "error": {
                "message": message,
                "type": "server_error",
                "code": "proxy_error",
            }
        })),
    )
        .into_response()
}

fn sse_response(chunks: Vec<String>) -> Response {
    let body = chunks.join("");
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/event-stream")
        .header("cache-control", "no-cache")
        .header("connection", "keep-alive")
        .header("x-accel-buffering", "no")
        .body(Body::from(body))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Internal error"))
                .unwrap()
        })
}
