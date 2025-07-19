# OpenRouter PKCE Authentication Implementation Plan

## Overview
Implement PKCE (Proof Key for Code Exchange) authentication flow for OpenRouter to obtain user-controlled API keys. The implementation will be added to the main `goose` crate and initially consumed by `goose-cli`.

## Architecture

### 1. Core Components (in main `goose` crate)

#### a. PKCE Flow Manager
- **Location**: `crates/goose/src/config/signup_openrouter/mod.rs`
- **Responsibilities**:
  - Generate code verifier (random string)
  - Generate code challenge (SHA-256 hash of verifier, base64url encoded)
  - Build authorization URL
  - Handle code exchange for API key

#### b. Local Web Server
- **Location**: `crates/goose/src/config/signup_openrouter/server.rs`
- **Port**: localhost:3000 (hardcoded, keeping it simple)
- **Responsibilities**:
  - Listen for OAuth callback
  - Extract authorization code from URL parameters
  - Display success message to user
  - Signal main process that code was received

### 2. Dependencies to Add
- `sha2` - For SHA-256 hashing
- `base64` - For base64url encoding
- `rand` - For generating secure random strings
- `tokio` - For async runtime (if not already present)
- `axum` - For lightweight web server (already used in project)
- `open` - For opening browser automatically

### 3. API Design

```rust
// In crates/goose/src/config/signup_openrouter/mod.rs

pub struct PkceAuthFlow {
    code_verifier: String,
    code_challenge: String,
    server_handle: Option<JoinHandle<Result<String, Error>>>,
}

impl PkceAuthFlow {
    /// Create a new PKCE flow with generated verifier and challenge
    pub fn new() -> Result<Self, Error>;
    
    /// Get the authorization URL to open in browser
    pub fn get_auth_url(&self) -> String;
    
    /// Start local server and wait for callback
    pub async fn start_server(&mut self) -> Result<(), Error>;
    
    /// Exchange authorization code for API key
    pub async fn exchange_code(&self, code: String) -> Result<String, Error>;
    
    /// Complete flow: open browser, wait for callback, exchange code
    pub async fn complete_flow(&mut self) -> Result<String, Error>;
}
```

### 4. Implementation Steps

1. **Step 1: Core PKCE Logic**
   - Generate 128-character random code_verifier
   - Calculate SHA-256 hash of verifier
   - Base64url encode the hash for code_challenge
   - Build auth URL with parameters

2. **Step 2: Local Web Server**
   - Create minimal HTTP server on localhost:3000
   - Single route handler for OAuth callback
   - Extract `code` parameter from query string
   - Return HTML page with success message
   - Use channel to communicate code back to main flow

3. **Step 3: Browser Integration**
   - Use `open` crate to launch default browser
   - Open authorization URL automatically
   - Handle case where browser fails to open

4. **Step 4: Code Exchange**
   - Make POST request to `https://openrouter.ai/api/v1/auth/keys`
   - Include code, code_verifier, and code_challenge_method
   - Parse response to extract API key

5. **Step 5: CLI Integration**
   - Add command to goose-cli (e.g., `goose auth openrouter`)
   - Print the API key to stdout (no storage for now)
   - Provide feedback during each step

### 5. Error Handling
- Network errors
- Server bind failures (port already in use)
- Invalid/expired authorization codes
- User cancellation
- Timeout (2-3 minutes for user to complete auth)

### 6. Security Considerations
- Use cryptographically secure random generator
- Implement proper PKCE S256 method
- Don't log sensitive values (codes, keys) except final println! of key

### 7. User Experience
1. User runs `goose auth openrouter`
2. Terminal shows: "Opening browser for authentication..."
3. Browser opens to OpenRouter auth page
4. User logs in and authorizes
5. Redirected to localhost:3000
6. Page shows: "Authentication successful! You can close this tab."
7. Terminal shows: "Authentication complete! Your API key is: sk-or-v1-..."

### 8. Testing Strategy
- Unit tests for PKCE challenge generation
- Integration test with mock server
- Manual testing with real OpenRouter flow

## Next Steps
1. Set up the module structure in the main goose crate
2. Implement core PKCE logic
3. Add web server functionality
4. Integrate with goose-cli
5. Test end-to-end flow

## Implementation Status

### ✅ Completed Phase 1
1. **Module Structure**: Created `src/config/signup_openrouter/` with:
   - `mod.rs` - Main PKCE flow implementation
   - `server.rs` - Axum web server for OAuth callback
   - `tests.rs` - Unit tests for PKCE challenge generation

2. **Core PKCE Logic**: 
   - Generates 128-character random code verifier
   - Creates SHA-256 hash and base64url encodes for challenge
   - Builds proper auth URL with PKCE parameters

3. **Web Server**:
   - Simple Axum server on localhost:3000
   - Handles OAuth callback with code extraction
   - Shows success/error pages to user
   - Graceful shutdown after receiving code

4. **CLI Integration**:
   - Added `goose auth openrouter` command
   - Opens browser automatically
   - Displays progress messages
   - Prints API key on success

5. **Tests**:
   - Verifies PKCE challenge generation
   - Ensures unique verifiers for each flow
   - Tests URL formatting

### ✅ Completed Phase 2
1. **Automated Setup**: When user runs `goose auth openrouter`:
   - Gets API key via PKCE flow
   - Stores key securely in keyring
   - Sets OpenRouter as provider
   - Configures models:
     - Main: `moonshotai/kimi-k2`
     - Lead: `anthropic/claude-3.5-sonnet`
     - Editor: `anthropic/claude-3.5-sonnet`
   - Tests configuration with a simple request
   - Provides clear success/failure feedback

2. **No User Interaction**: Fully automated configuration after authentication

### 📋 Usage

```bash
# Authenticate and configure OpenRouter
goose auth openrouter

# Output:
# Opening browser for authentication...
# Waiting for authentication callback...
# Authorization code received. Exchanging for API key...
# 
# Authentication complete! Your API key is:
# sk-or-v1-...
# 
# Configuring OpenRouter...
# ✓ API key stored securely
# ✓ Provider set to OpenRouter
# ✓ Model set to moonshotai/kimi-k2
# ✓ Lead model configured (anthropic/claude-3.5-sonnet)
# ✓ Editor model configured (anthropic/claude-3.5-sonnet)
# 
# Testing configuration...
# ✓ Configuration test passed!
# 
# OpenRouter setup complete! You can now use Goose.
```

### 🎯 What Gets Configured

After running `goose auth openrouter`, the following settings are automatically saved:

1. **API Key**: `OPENROUTER_API_KEY` (stored in system keyring)
2. **Provider**: `GOOSE_PROVIDER` = "openrouter"
3. **Models**:
   - `GOOSE_MODEL` = "moonshotai/kimi-k2"
   - `GOOSE_LEAD_MODEL` = "anthropic/claude-3.5-sonnet"
   - `GOOSE_LEAD_PROVIDER` = "openrouter"
   - `GOOSE_EDITOR_MODEL` = "anthropic/claude-3.5-sonnet"
   - `GOOSE_EDITOR_PROVIDER` = "openrouter"

### 🔄 Future Enhancements
- Add model switching commands
- Support for other OAuth providers
- Usage tracking and cost monitoring
- Model performance comparisons

---

# Configuration System Analysis & OpenRouter Integration Plan

## Understanding Goose's Configuration System

### 1. **Configuration Storage**
Goose uses a multi-layered configuration system:

- **Config Files**: 
  - Location: `~/.config/goose/config.yaml` (macOS/Linux)
  - Format: YAML
  - Used for: Non-sensitive settings (provider, model, etc.)
  
- **Secrets Storage**:
  - **Keyring** (default): System keychain/credential manager
  - **File** (fallback): `~/.config/goose/secrets.yaml` if keyring disabled
  - Used for: API keys and sensitive data

### 2. **Config Priority**
Configuration values are loaded with this precedence:
1. Environment variables (highest)
2. Config file / keyring
3. Default values (lowest)

### 3. **Key APIs**
```rust
// Get/set regular config values
config.get_param::<T>(key) -> Result<T>
config.set_param(key, value) -> Result<()>

// Get/set secrets
config.get_secret::<T>(key) -> Result<T>
config.set_secret(key, value) -> Result<()>

// Generic get/set (auto-determines secret vs param)
config.get(key, is_secret) -> Result<Value>
config.set(key, value, is_secret) -> Result<()>
```

### 4. **Provider Configuration**
Each provider has metadata defining:
- Name, display name, description
- Default model
- Known/supported models
- Required config keys (with secret flag)

Example from OpenRouter:
```rust
ConfigKey::new("OPENROUTER_API_KEY", true, true, None),  // required, secret
ConfigKey::new("OPENROUTER_HOST", false, false, Some("https://openrouter.ai")), // optional, not secret
```

## Integration Plan: OpenRouter Setup Flow

### Phase 1: Enhance Auth Command ✅
Already implemented:
- `goose auth openrouter` - Gets API key via PKCE flow
- Prints key to stdout

### Phase 2: Automated Setup (Next Implementation)
**Goal**: When user runs `goose auth openrouter`, automatically:
1. Get API key via PKCE
2. Store the key securely
3. Set OpenRouter as provider
4. Configure models:
   - Main model: `moonshotai/kimi-k2`
   - Lead model: `anthropic/claude-3.5-sonnet` (for lead/worker setup)
   - Editor model: `anthropic/claude-3.5-sonnet` (for code editing)
5. Test the configuration
6. Save everything

**No prompts, no interactive lists - fully automated.**

### Implementation Details:

```rust
// In goose-cli auth handler:
Some(Command::Auth { service }) => {
    if service == "openrouter" {
        // 1. Get API key via PKCE
        let mut auth_flow = goose::config::signup_openrouter::OpenRouterAuth::new()?;
        let api_key = auth_flow.complete_flow().await?;
        
        println!("\nAuthentication complete! Your API key is:");
        println!("{}", api_key);
        
        // 2. Automatically configure everything
        let config = Config::global();
        
        // Store API key securely
        config.set_secret("OPENROUTER_API_KEY", Value::String(api_key.clone()))?;
        println!("✓ API key stored securely");
        
        // Set provider
        config.set_param("GOOSE_PROVIDER", Value::String("openrouter".to_string()))?;
        println!("✓ Provider set to OpenRouter");
        
        // Set main model
        config.set_param("GOOSE_MODEL", Value::String("moonshotai/kimi-k2".to_string()))?;
        println!("✓ Model set to moonshotai/kimi-k2");
        
        // Set lead model for lead/worker pattern
        config.set_param("GOOSE_LEAD_MODEL", Value::String("anthropic/claude-3.5-sonnet".to_string()))?;
        config.set_param("GOOSE_LEAD_PROVIDER", Value::String("openrouter".to_string()))?;
        println!("✓ Lead model configured");
        
        // Set editor model
        config.set_param("GOOSE_EDITOR_MODEL", Value::String("anthropic/claude-3.5-sonnet".to_string()))?;
        config.set_param("GOOSE_EDITOR_PROVIDER", Value::String("openrouter".to_string()))?;
        println!("✓ Editor model configured");
        
        // Test configuration
        println!("\nTesting configuration...");
        let provider = create("openrouter", ModelConfig::new("moonshotai/kimi-k2".to_string()))?;
        // Simple test request
        let test_result = provider.complete(
            "You are Goose, an AI assistant.",
            &[Message::user().with_text("Say 'Configuration test successful!'")],
            &[]
        ).await;
        
        match test_result {
            Ok(_) => {
                println!("✓ Configuration test passed!");
                println!("\nOpenRouter setup complete! You can now use Goose.");
            }
            Err(e) => {
                eprintln!("⚠️  Configuration test failed: {}", e);
                eprintln!("Your API key has been saved, but there may be an issue with the connection.");
            }
        }
    }
}
```

### Configuration Values Set:
- `OPENROUTER_API_KEY`: User's API key (stored in keyring)
- `GOOSE_PROVIDER`: "openrouter"
- `GOOSE_MODEL`: "moonshotai/kimi-k2"
- `GOOSE_LEAD_MODEL`: "anthropic/claude-3.5-sonnet"
- `GOOSE_LEAD_PROVIDER`: "openrouter"
- `GOOSE_EDITOR_MODEL`: "anthropic/claude-3.5-sonnet"
- `GOOSE_EDITOR_PROVIDER`: "openrouter"

### Benefits:
1. **One command setup**: `goose auth openrouter` does everything
2. **No user interaction needed**: Fully automated
3. **Optimized model selection**: Kimi K2 for main work, Claude for lead/editing
4. **Immediate validation**: Tests config before finishing

## Next Steps

1. Create reusable auth function in goose crate
2. Add provider setup logic to auth command
3. Update configure command to handle OpenRouter specially
4. Add model fetching and selection
5. Implement configuration testing
6. Update documentation
