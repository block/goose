# Goose Secure Store

A secure, cross-platform secret management system for Goose MCP extensions. This crate provides secure storage and retrieval of sensitive information like API keys, tokens, and other credentials needed by MCP servers.

## Features

- **Cross-platform secure storage**: Uses the operating system's native keychain/credential store
  - macOS: Keychain Services
  - Windows: Windows Credential Manager  
  - Linux: Secret Service API (libsecret)
- **Interactive secret acquisition**: Prompts users securely for credentials with consent flow
- **Environment variable fallback**: Gracefully falls back to environment variables when secure storage is unavailable
- **Hierarchical namespacing**: Isolates secrets by extension and secret name
- **Comprehensive error handling**: Clear, actionable error messages for troubleshooting

## Architecture

The crate consists of three main components:

### 1. SecureStore Trait (`src/store.rs`)

Provides an abstraction over platform-specific secure storage:

```rust
pub trait SecureStore {
    fn set_secret(&self, service: &str, username: &str, secret: &str) -> Result<(), SecretError>;
    fn get_secret(&self, service: &str, username: &str) -> Result<String, SecretError>;
    fn delete_secret(&self, service: &str, username: &str) -> Result<(), SecretError>;
    fn has_secret(&self, service: &str, username: &str) -> bool;
}
```

### 2. SecretAcquisition (`src/acquisition.rs`)

Handles the user interaction flow for acquiring secrets:

```rust
pub struct SecretAcquisition {
    store: Box<dyn SecureStore>,
}

impl SecretAcquisition {
    pub fn acquire_prompt_secret(
        &self,
        server_name: &str,
        secret_name: &str,
        description: &str,
        prompt_message: Option<&str>,
    ) -> Result<String, SecretError>;
}
```

### 3. Error Types (`src/error.rs`)

Comprehensive error handling with user-friendly messages:

```rust
#[derive(Error, Debug)]
pub enum SecretError {
    #[error("Secret not found: {0}")]
    NotFound(String),
    
    #[error("Storage operation failed: {0}")]
    StorageFailure(String),
    
    #[error("User cancelled the operation")]
    UserCancelled,
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}
```

## Configuration

Secrets are configured in MCP extension configurations using the `secrets` array:

```json
{
  "name": "my-extension",
  "command": "npx",
  "args": ["my-mcp-server"],
  "secrets": [
    {
      "name": "API_KEY",
      "description": "Your service API key",
      "acquisition": {
        "type": "prompt",
        "prompt_message": "Please enter your API key from https://example.com/settings"
      }
    },
    {
      "name": "ACCESS_TOKEN", 
      "description": "OAuth access token",
      "acquisition": {
        "type": "prompt"
      }
    }
  ]
}
```

### Secret Configuration Schema

Each secret in the `secrets` array supports:

- **`name`** (required): Environment variable name that will be set for the MCP server
- **`description`** (required): Human-readable description of what this secret is for
- **`acquisition`** (required): How to acquire the secret

#### Acquisition Methods

**Prompt** (Currently implemented):
```yaml
acquisition:
  method: prompt
  prompt_message: "Custom prompt message (optional)"
```

**OAuth2** (Planned for Milestone 3):
```json
{
  "type": "oauth2",
  "authorization_url": "https://example.com/oauth/authorize",
  "token_url": "https://example.com/oauth/token",
  "client_id": "your-client-id",
  "scopes": ["read", "write"]
}
```

**Command** (Planned for Milestone 2):
```json
{
  "type": "command",
  "command": "aws",
  "args": ["sts", "get-session-token", "--output", "text", "--query", "Credentials.SessionToken"]
}
```

## Storage Format

Secrets are stored using hierarchical namespacing:

- **Service**: `goose.mcp.{server_name}`
- **Username**: `{secret_name}`

For example, a secret named `API_KEY` for extension `github-mcp` would be stored as:
- Service: `goose.mcp.github-mcp`
- Username: `API_KEY`

This ensures proper isolation between different extensions and their secrets.

## Usage Examples

### Basic Usage

```rust
use goose_secure_store::SecretAcquisition;

let acquisition = SecretAcquisition::new();

// Acquire a secret with user prompt
let api_key = acquisition.acquire_prompt_secret(
    "github-mcp",           // server name
    "GITHUB_TOKEN",         // secret name  
    "GitHub personal access token", // description
    Some("Get your token from https://github.com/settings/tokens") // custom prompt
)?;

// Check if a secret exists
if acquisition.has_secret("github-mcp", "GITHUB_TOKEN") {
    let token = acquisition.get_secret("github-mcp", "GITHUB_TOKEN")?;
    println!("Token retrieved: {}", token);
}
```

### Testing with Mock Store

```rust
use goose_secure_store::{SecretAcquisition, MockSecureStore};

let mock_store = MockSecureStore::new();
let acquisition = SecretAcquisition::with_store(Box::new(mock_store));

// Mock store allows testing without user interaction
```

## Testing Locally

### 1. Unit Tests

Run the comprehensive test suite:

```bash
cargo test -p goose-secure-store
```

This runs:
- 6 unit tests for SecureStore implementations
- 3 environment fallback tests  
- 2 integration tests
- All tests should pass

### 2. Manual Testing with Goose CLI

The secret management feature is integrated into the extension loading process. Here's how to test it:

#### Step 1: Add an Extension with Secrets

Use `goose configure` to add a new stdio extension:

```bash
./target/debug/goose configure
# Select "Add Extension" 
# Select "Command-line Extension"
# Name: test-secrets
# Command: echo "Hello from test extension"
# Add environment variables: Yes
# Environment variable name: TEST_API_KEY
# Environment variable value: test-value-123
```

#### Step 2: Manually Edit the Configuration

After adding the extension, edit your config file to add the secrets configuration:

```bash
# Edit the config file
nano ~/.config/goose/config.yaml
```

Find the extension you just created and modify it to include secrets:

```yaml
extensions:
  test-secrets:
    enabled: true
    config:
      name: test-secrets
      command: echo
      args: ["Hello from test extension"]
      envs: {}
      env_keys: []
      secrets:
        - name: TEST_API_KEY
          description: "A test API key for demonstration"
          acquisition:
            type: prompt
            prompt_message: "Enter any test value (this is just for testing)"
      description: "Test extension for secret management"
      timeout: 30
```

#### Step 3: Test Secret Acquisition

Now when you start a goose session, it will try to load the extension and prompt for the secret:

```bash
./target/debug/goose session
```

You should see a prompt like:
```
Extension 'test-secrets' requires a secret: TEST_API_KEY
A test API key for demonstration
Enter any test value (this is just for testing)
Do you consent to storing this secret securely? (y/N): y
TEST_API_KEY: [enter your test value]
```

#### Alternative: Using the Provided Test Files

The project includes ready-to-use test files with both Node.js and Python implementations:

##### Option 1: Node.js Version

1. **Use the Node.js test MCP server**:
   ```bash
   # Make sure Node.js is installed
   node --version
   
   # Backup your existing config
   cp ~/.config/goose/config.yaml ~/.config/goose/config.yaml.backup
   
   # Use the test configuration
   cp test-secrets-config.yaml ~/.config/goose/config.yaml
   
   # Run from the goose project directory
   ./target/debug/goose session
   ```

##### Option 2: Python Version (Recommended)

1. **Use the Python test MCP server**:
   ```bash
   # Make sure Python 3 is installed
   python3 --version
   
   # Backup your existing config
   cp ~/.config/goose/config.yaml ~/.config/goose/config.yaml.backup
   
   # Use the Python test configuration
   cp test-secrets-config-python.yaml ~/.config/goose/config.yaml
   
   # Run from the goose project directory
   ./target/debug/goose session
   ```

##### Testing Steps (Same for Both Versions)

2. **You should see a prompt like**:
   ```
   🔐 Secret Storage Consent
   ─────────────────────────
   Server: test-secrets
   Secret: TEST_API_KEY (A test API key for demonstration)
   
   Goose would like to securely store this secret in your system's keychain.
   This will allow automatic retrieval for future MCP server connections.
   
   Do you consent to storing this secret? [y/N]: y
   
   🔑 Secret Input Required
   ─────────────────────────
   Enter any test value (this is just for testing the secret management system)
   
   TEST_API_KEY: [enter your test value]
   ```

3. **Test the extension**:
   - The extension should load successfully
   - Ask goose: "use the test_secret tool"
   - It should respond with your secret value

4. **Restore your config**:
   ```bash
   mv ~/.config/goose/config.yaml.backup ~/.config/goose/config.yaml
   ```

##### Python Version Advantages

The Python version (`test-mcp-server.py`) offers several improvements:
- **Better code structure**: More readable and maintainable
- **Type hints**: Enhanced code clarity and IDE support
- **Improved error handling**: Better exception handling and logging
- **Pythonic patterns**: More idiomatic Python code
- **Easier to extend**: Simpler to add new functionality
- **Better documentation**: Comprehensive docstrings

### 3. Environment Variable Fallback Testing

Test the fallback mechanism:

```bash
# Set environment variable
export TEST_API_KEY="fallback-value"

# Run goose - it should use the environment variable if secure storage fails
goose session
```

### 4. Cross-Platform Testing

The secure store works differently on each platform:

**macOS**: 
- Secrets stored in Keychain Access
- View with: `open /Applications/Utilities/Keychain\ Access.app`
- Look for entries with service name `goose.mcp.*`

**Windows**:
- Secrets stored in Windows Credential Manager
- View with: Control Panel → Credential Manager → Windows Credentials
- Look for entries with target name `goose.mcp.*`

**Linux**:
- Secrets stored via Secret Service API
- View with: `secret-tool search service goose.mcp`

## Error Handling

The system provides comprehensive error handling with actionable messages:

### Common Error Scenarios

1. **Secret Not Found**: 
   - Tries secure storage first
   - Falls back to environment variable
   - Provides clear error if neither available

2. **Storage Unavailable**:
   - Gracefully falls back to environment variables
   - Provides guidance on system requirements

3. **User Cancellation**:
   - Handles Ctrl+C during secret input
   - Provides option to retry or use environment variables

### Example Error Messages

```
Failed to acquire secret 'API_KEY' for extension 'github-mcp': Secret not found

To resolve this issue, you can:
1. Run the command again and provide the secret when prompted
2. Set the environment variable 'API_KEY' with your secret value  
3. Check that your system's keychain/credential store is accessible
```

## Security Considerations

- **No plaintext storage**: Secrets are never stored in plaintext files
- **OS-level encryption**: Uses platform-native secure storage with OS-level encryption
- **Process isolation**: Secrets are only accessible to the Goose process and its children
- **Consent flow**: Users must explicitly consent before secrets are stored
- **Graceful fallback**: Falls back to environment variables when secure storage unavailable

## Development

### Adding New Acquisition Methods

To add new secret acquisition methods (like OAuth2 or command-based):

1. Extend the `SecretAcquisition` enum in `src/acquisition.rs`
2. Update the JSON schema in `mcp_secrets.schema.json`
3. Implement the acquisition logic in `SecretAcquisition::acquire_secret()`
4. Add comprehensive tests

### Testing

The crate includes both unit tests and integration tests:

- **Unit tests**: Test individual components with mocks
- **Integration tests**: Test end-to-end flows
- **Environment fallback tests**: Test graceful degradation

Run tests with:
```bash
cargo test -p goose-secure-store -- --nocapture
```

## Troubleshooting

### Common Issues

**"Secret not found" errors**:
- Check if the secret was previously stored
- Try setting the environment variable as fallback
- Verify system keychain/credential store is accessible

**Permission errors on Linux**:
- Ensure `libsecret` is installed: `sudo apt-get install libsecret-1-dev`
- Check that Secret Service is running: `systemctl --user status gnome-keyring`

**Keychain access issues on macOS**:
- Check Keychain Access permissions
- Try running with `security unlock-keychain` if needed

**Windows Credential Manager issues**:
- Verify Windows Credential Manager service is running
- Check user permissions for credential access

## Future Enhancements

- **Milestone 2**: Command-based secret acquisition
- **Milestone 3**: OAuth2 flow support  
- **Secret rotation**: Automatic secret refresh capabilities
- **Audit logging**: Track secret access for security monitoring
- **Backup/restore**: Export/import encrypted secret backups
