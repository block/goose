# Local Inference Integration Plan

## Goal
Integrate local LLM inference into the desktop app following the whisper dictation pattern. Users can download and manage local models through the UI, then use them for inference without requiring API keys.

## MVP Scope

### Performance
- Current speed: ~230 tokens/sec on Metal GPU, ~357 tokens/sec prefill
- Context limits vary by model (1B = 4K, larger models support more)
- llama.cpp integration deferred for future optimization

### Model Tier System
Hardcode 4 models optimized for different hardware profiles:

| Tier   | Model                | Size   | Context | Use Case                    |
|--------|---------------------|--------|---------|----------------------------|
| Tiny   | Llama 3.2 1B       | ~0.7GB | 4K      | CPU-only, quick responses  |
| Small  | Llama 3.2 3B       | ~2GB   | 8K      | Laptops, balanced          |
| Medium | Hermes 2 Pro 7B    | ~4.5GB | 8K      | Desktops with GPU          |
| Large  | Mistral Small 22B  | ~13GB  | 32K     | High-end, long context     |

All models use Q4_K_M quantization for optimal size/quality balance.

## Architecture Pattern

### Follow Whisper Integration
The implementation mirrors `crates/goose/src/dictation/`:
- **Model definitions** → `local_inference.rs` (like `whisper.rs`)
- **Provider interface** → Already exists in `providers/local_inference.rs`
- **Download manager** → Reuse existing `dictation/download_manager.rs`
- **API routes** → New `routes/local_inference.rs` (like `routes/dictation.rs`)
- **OpenAPI schema** → Add to `openapi.rs`

## Implementation Plan

### Phase 1: Model Definitions & Management

#### 1.1 Add Model Constants
**File:** `crates/goose/src/providers/local_inference.rs`

Add model definitions similar to whisper:
```rust
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LocalLlmModel {
    pub id: &'static str,           // "llama-3.2-1b"
    pub name: &'static str,          // "Llama 3.2 1B Instruct"
    pub size_mb: u32,                // 700
    pub context_limit: usize,        // 4096
    pub url: &'static str,           // HuggingFace download URL
    pub tokenizer_url: &'static str, // Tokenizer JSON URL
    pub description: &'static str,   // "Tiny: CPU-only, quick responses"
    pub tier: ModelTier,             // Tiny/Small/Medium/Large
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum ModelTier {
    Tiny,
    Small,
    Medium,
    Large,
}

pub const LOCAL_LLM_MODELS: &[LocalLlmModel] = &[
    LocalLlmModel {
        id: "llama-3.2-1b",
        name: "Llama 3.2 1B Instruct",
        size_mb: 700,
        context_limit: 4096,
        url: "https://huggingface.co/.../*.gguf",
        tokenizer_url: "https://huggingface.co/.../tokenizer.json",
        description: "Fastest, CPU-optimized for quick responses",
        tier: ModelTier::Tiny,
    },
    // ... 3 more models
];
```

#### 1.2 Add Model Helper Functions
```rust
pub fn available_local_models() -> &'static [LocalLlmModel] {
    LOCAL_LLM_MODELS
}

pub fn get_local_model(id: &str) -> Option<&'static LocalLlmModel> {
    LOCAL_LLM_MODELS.iter().find(|m| m.id == id)
}

pub fn recommend_local_model() -> &'static str {
    let has_gpu = Device::new_cuda(0).is_ok() || Device::new_metal(0).is_ok();
    let cpu_count = sys_info::cpu_num().unwrap_or(1) as u64;
    let mem_mb = sys_info::mem_info().map(|m| m.avail).unwrap_or(0) / 1024;

    if has_gpu && mem_mb >= 16_000 {
        "hermes-2-pro-7b"  // Medium tier
    } else if mem_mb >= 4_000 {
        "llama-3.2-3b"     // Small tier
    } else {
        "llama-3.2-1b"     // Tiny tier
    }
}

impl LocalLlmModel {
    pub fn local_path(&self) -> PathBuf {
        Paths::in_data_dir("models").join(format!("{}.gguf", self.id))
    }

    pub fn tokenizer_path(&self) -> PathBuf {
        Paths::in_data_dir("models")
            .join(format!("{}_tokenizer.json", self.id))
    }

    pub fn is_downloaded(&self) -> bool {
        self.local_path().exists() && self.tokenizer_path().exists()
    }
}
```

### Phase 2: Provider Integration

#### 2.1 Update Provider to Use Model Definitions
**File:** `crates/goose/src/providers/local_inference.rs`

Current implementation uses `find_model_by_name()` with prefix matching. Update to:
```rust
async fn load_model(&self, model_id: &str) -> Result<LoadedModel, ProviderError> {
    let model = get_local_model(model_id)
        .ok_or_else(|| ProviderError::ExecutionError(
            format!("Unknown model: {}", model_id)
        ))?;

    let model_path = model.local_path();
    let tokenizer_path = model.tokenizer_path();

    if !model_path.exists() {
        return Err(ProviderError::ExecutionError(
            format!("Model not downloaded: {}. Download it from Settings.", model.name)
        ));
    }

    tracing::info!("Loading {} from: {}", model.name, model_path.display());

    // ... existing loading code using model_path and tokenizer_path
}
```

#### 2.2 Update ProviderMetadata
```rust
impl ProviderDef for LocalInferenceProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "local",
            "Local Inference",
            "Local inference using quantized GGUF models (Candle)",
            "llama-3.2-1b",  // Default to tiny model
            vec![
                "llama-3.2-1b",
                "llama-3.2-3b",
                "hermes-2-pro-7b",
                "mistral-small-22b",
            ],
            "https://github.com/huggingface/candle",
            vec![], // No API keys required
        )
    }
}
```

### Phase 3: API Routes

#### 3.1 Create Routes File
**File:** `crates/goose-server/src/routes/local_inference.rs`

Mirror the dictation routes structure:

```rust
use goose::providers::local_inference::{
    available_local_models, get_local_model, recommend_local_model, LocalLlmModel
};
use goose::dictation::download_manager::{get_download_manager, DownloadProgress};

#[derive(Debug, Serialize, ToSchema)]
pub struct LocalModelResponse {
    #[serde(flatten)]
    model: &'static LocalLlmModel,
    downloaded: bool,
    recommended: bool,
}

// GET /local-inference/models
#[utoipa::path(
    get,
    path = "/local-inference/models",
    responses(
        (status = 200, description = "List of available local LLM models",
         body = Vec<LocalModelResponse>)
    )
)]
pub async fn list_local_models() -> Result<Json<Vec<LocalModelResponse>>, ErrorResponse> {
    let recommended_id = recommend_local_model();
    let models = available_local_models()
        .iter()
        .map(|m| LocalModelResponse {
            model: m,
            downloaded: m.is_downloaded(),
            recommended: m.id == recommended_id,
        })
        .collect();
    Ok(Json(models))
}

// POST /local-inference/models/{model_id}/download
#[utoipa::path(
    post,
    path = "/local-inference/models/{model_id}/download",
    responses(
        (status = 202, description = "Download started"),
        (status = 400, description = "Model not found or download already in progress"),
    )
)]
pub async fn download_local_model(
    Path(model_id): Path<String>
) -> Result<StatusCode, ErrorResponse> {
    let model = get_local_model(&model_id)
        .ok_or_else(|| ErrorResponse::bad_request("Model not found"))?;

    let manager = get_download_manager();

    // Download model file
    manager.download_model(
        format!("{}-model", model.id),
        model.url.to_string(),
        model.local_path(),
    ).await.map_err(convert_error)?;

    // Download tokenizer file
    manager.download_model(
        format!("{}-tokenizer", model.id),
        model.tokenizer_url.to_string(),
        model.tokenizer_path(),
    ).await.map_err(convert_error)?;

    Ok(StatusCode::ACCEPTED)
}

// GET /local-inference/models/{model_id}/download
pub async fn get_local_model_download_progress(
    Path(model_id): Path<String>,
) -> Result<Json<DownloadProgress>, ErrorResponse> {
    // Return progress for the model file (primary progress indicator)
    let manager = get_download_manager();
    let progress = manager
        .get_progress(&format!("{}-model", model_id))
        .ok_or_else(|| ErrorResponse::bad_request("Download not found"))?;
    Ok(Json(progress))
}

// DELETE /local-inference/models/{model_id}/download
pub async fn cancel_local_model_download(
    Path(model_id): Path<String>
) -> Result<StatusCode, ErrorResponse> {
    let manager = get_download_manager();
    manager.cancel_download(&format!("{}-model", model_id))
        .map_err(convert_error)?;
    manager.cancel_download(&format!("{}-tokenizer", model_id))
        .map_err(convert_error)?;
    Ok(StatusCode::OK)
}

// DELETE /local-inference/models/{model_id}
pub async fn delete_local_model(
    Path(model_id): Path<String>
) -> Result<StatusCode, ErrorResponse> {
    let model = get_local_model(&model_id)
        .ok_or_else(|| ErrorResponse::bad_request("Model not found"))?;

    let model_path = model.local_path();
    let tokenizer_path = model.tokenizer_path();

    if !model_path.exists() && !tokenizer_path.exists() {
        return Err(ErrorResponse::bad_request("Model not downloaded"));
    }

    // Delete both files
    if model_path.exists() {
        tokio::fs::remove_file(&model_path).await
            .map_err(|e| ErrorResponse::internal(format!("Failed to delete model: {}", e)))?;
    }
    if tokenizer_path.exists() {
        tokio::fs::remove_file(&tokenizer_path).await
            .map_err(|e| ErrorResponse::internal(format!("Failed to delete tokenizer: {}", e)))?;
    }

    Ok(StatusCode::OK)
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/local-inference/models", get(list_local_models))
        .route("/local-inference/models/{model_id}/download", post(download_local_model))
        .route("/local-inference/models/{model_id}/download", get(get_local_model_download_progress))
        .route("/local-inference/models/{model_id}/download", delete(cancel_local_model_download))
        .route("/local-inference/models/{model_id}", delete(delete_local_model))
        .with_state(state)
}
```

#### 3.2 Register Routes
**File:** `crates/goose-server/src/lib.rs`

Add to router:
```rust
mod routes {
    pub mod local_inference;  // Add this
    // ... existing modules
}

// In build_router():
.merge(routes::local_inference::routes(state.clone()))
```

### Phase 4: OpenAPI Integration

#### 4.1 Update OpenAPI Schema
**File:** `crates/goose-server/src/openapi.rs`

Add to the `#[openapi(paths(...))]` macro:
```rust
super::routes::local_inference::list_local_models,
super::routes::local_inference::download_local_model,
super::routes::local_inference::get_local_model_download_progress,
super::routes::local_inference::cancel_local_model_download,
super::routes::local_inference::delete_local_model,
```

Add to `components(schemas(...))`:
```rust
super::routes::local_inference::LocalModelResponse,
goose::providers::local_inference::LocalLlmModel,
goose::providers::local_inference::ModelTier,
```

#### 4.2 Generate Schema
Run the command to regenerate OpenAPI schema:
```bash
just generate-openapi
```

This will:
1. Build and run `cargo run -p goose-server --bin generate_schema`
2. Generate `ui/desktop/openapi.json`
3. Run `npx @hey-api/openapi-ts` to generate TypeScript client

### Phase 5: Configuration Integration

#### 5.1 Add Config Key
**File:** `crates/goose/src/providers/local_inference.rs`

```rust
pub const LOCAL_LLM_MODEL_CONFIG_KEY: &str = "LOCAL_LLM_MODEL";
```

#### 5.2 Provider Detection
The local provider should appear in provider lists and be detected as configured if a model is downloaded:

```rust
// In provider initialization
pub fn is_local_provider_configured() -> bool {
    let config = Config::global();
    config
        .get(LOCAL_LLM_MODEL_CONFIG_KEY, false)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .and_then(|id| get_local_model(&id))
        .is_some_and(|m| m.is_downloaded())
}
```

## Testing Plan

### 1. API Endpoint Testing
```bash
# List models
curl http://localhost:3000/local-inference/models

# Start download
curl -X POST http://localhost:3000/local-inference/models/llama-3.2-1b/download

# Check progress
curl http://localhost:3000/local-inference/models/llama-3.2-1b/download

# Cancel download
curl -X DELETE http://localhost:3000/local-inference/models/llama-3.2-1b/download

# Delete model
curl -X DELETE http://localhost:3000/local-inference/models/llama-3.2-1b
```

### 2. Provider Testing
```bash
# After downloading a model, test inference
GOOSE_PROVIDER=local GOOSE_MODEL=llama-3.2-1b cargo run --release -- run --text "Hello"
```

### 3. Desktop App Testing
1. Start desktop app: `just ui-desktop`
2. Navigate to Settings > Local Inference
3. Verify model list shows all 4 models with correct metadata
4. Download tiny model (700MB)
5. Verify progress bar updates
6. Cancel and restart download
7. Delete downloaded model
8. Select local provider for a session
9. Send messages and verify responses

## File Changes Summary

### New Files
- `crates/goose-server/src/routes/local_inference.rs` (~300 lines)

### Modified Files
- `crates/goose/src/providers/local_inference.rs` (add model definitions, ~150 lines)
- `crates/goose-server/src/lib.rs` (register routes, ~5 lines)
- `crates/goose-server/src/openapi.rs` (add schemas/paths, ~10 lines)
- `crates/goose/src/providers/mod.rs` (export constants, ~2 lines)

### Generated Files (auto-generated)
- `ui/desktop/openapi.json`
- `ui/desktop/src/client/...` (TypeScript types)

## Known Limitations

### Context Windows
- Llama 3.2 1B: 4K tokens (not suitable for large system prompts)
- Llama 3.2 3B: 8K tokens
- Hermes 2 Pro 7B: 8K tokens
- Mistral Small 22B: 32K tokens

For Goose's typical system prompt (~700 tokens), recommend 3B or larger.

### Prompt Formatting
Current implementation uses simple text concatenation:
```rust
fn build_prompt(&self, _system: &str, messages: &[Message]) -> String {
    if let Some(last_message) = messages.last() {
        last_message.as_concat_text()
    } else {
        String::new()
    }
}
```

**Future improvement:** Implement proper Llama 3 chat templates:
```
<|begin_of_text|><|start_header_id|>system<|end_header_id|>
{system}<|eot_id|><|start_header_id|>user<|end_header_id|>
{user}<|eot_id|><|start_header_id|>assistant<|end_header_id|>
```

This would enable multi-turn conversations and system prompts.

### Performance
- Prefill: ~350-550 tokens/sec (varies by model size)
- Generation: ~230 tokens/sec on Metal GPU
- 10-20x slower than API providers
- llama.cpp would be ~3-4x faster but requires C++ integration

## Success Criteria

- ✅ Desktop app shows 4 local models in settings
- ✅ Can download models with progress indication
- ✅ Can cancel downloads mid-flight
- ✅ Can delete downloaded models
- ✅ Local provider appears in provider list when model downloaded
- ✅ Can create session with local provider
- ✅ Can send messages and receive responses
- ✅ Generate OpenAPI schema includes new endpoints
- ✅ TypeScript types auto-generated for frontend

## Future Enhancements (Post-MVP)

1. **Llama.cpp Integration** - 3-4x faster inference
2. **Proper Chat Templates** - Support system prompts and multi-turn
3. **Streaming Responses** - Real-time token generation
4. **Tool Calling** - Function calling support for local models
5. **Fine-tuned Models** - Add code-specific models
6. **LoRA Adapters** - Task-specific model adaptations
7. **Automatic Model Selection** - Based on query complexity
8. **Model Quantization Options** - Q8, Q6, Q4 variants
9. **GPU Memory Management** - Offload layers to GPU strategically
10. **Context Window Expansion** - RoPE scaling for longer contexts
