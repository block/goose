# Local Development Setup Guide

This guide covers setting up Watchflow for complete end-to-end local development, including GitHub App configuration, webhook setup, and API integration. For direction and supported logic, see [README](README.md) and [docs](docs/).

## Why This Setup?

**Personal GitHub App vs. Marketplace App**: For local development, creating your own GitHub App instead of using the production Watchflow app from the marketplace provides several critical advantages:

- **Isolated development environment**: Your local testing won't interfere with production Watchflow instances
- **Full control over webhooks**: You can point webhooks to your local ngrok tunnel instead of production servers
- **Custom configuration**: You can modify permissions and settings without affecting the production app
- **Safe experimentation**: Test new features, rule changes, and integrations without risk to live systems
- **Independent debugging**: Monitor webhook deliveries and debug issues in isolation
- **No rate limiting conflicts**: Avoid hitting GitHub API rate limits that might affect production usage

**ngrok for Local Development**: Since GitHub webhooks need to reach your local development server, and your localhost isn't accessible from the internet, ngrok creates a secure tunnel that:

- **Exposes your local server** to GitHub's webhook delivery system
- **Provides HTTPS endpoints** required by GitHub for webhook URLs
- **Offers request inspection** tools to debug webhook payloads
- **Eliminates complex firewall configuration** or port forwarding

## Prerequisites

- Python 3.12 or higher
- [uv](https://docs.astral.sh/uv/) package manager (recommended) or pip
- [ngrok](https://ngrok.com/) installed for webhook tunneling
- GitHub organization or user account with admin access
- [OpenAI API key](https://platform.openai.com/api-keys) for AI agent functionality
- [LangSmith account](https://langsmith.com/) for AI agent debugging (optional)

## Step 1: Create a GitHub App

1. Navigate to your GitHub organization settings or [personal settings](https://github.com/settings/apps)
2. Go to **"Developer settings"** → **"GitHub Apps"** → **"New GitHub App"**
3. Fill in the basic app information:
   - **App name**: `watchflow-dev` (or your preferred name)
   - **Homepage URL**: `http://localhost:8000`
   - **Webhook URL**: `https://placeholder.ngrok.io/webhooks/github` (we'll update this in Step 3)
   - **Webhook secret**: Generate a secure random string and save it
   - **Description**: "Local development instance of Watchflow"

## Step 2: Configure GitHub App Permissions

### Repository Permissions

Set the following permissions for your GitHub App:

- **Actions**: Read-only
- **Checks**: Read and write
- **Contents**: Read-only
- **Deployments**: Read and write
- **Environments**: Read-only
- **Issues**: Read and write
- **Metadata**: Read-only (mandatory)
- **Pull requests**: Read and write
- **Commit statuses**: Read and write

### Organization Permissions

- **Members**: Read-only

### Subscribe to Events

Check the following webhook events:

- ✅ **Check run**
- ✅ **Commit comment**
- ✅ **Deployment**
- ✅ **Deployment protection rule**
- ✅ **Deployment review**
- ✅ **Deployment status**
- ✅ **Issue comment**
- ✅ **Issues**
- ✅ **Pull request**
- ✅ **Pull request review**
- ✅ **Pull request review comment**
- ✅ **Pull request review thread**
- ✅ **Push**
- ✅ **Status**
- ✅ **Workflow dispatch**
- ✅ **Workflow job**
- ✅ **Workflow run**

### Generate and Download Private Key

1. After creating the app, scroll down to **"Private keys"**
2. Click **"Generate a private key"**
3. Download the `.pem` file and save it securely
4. Note down your **App ID** from the app settings page

## Step 3: Clone and Setup Watchflow

### 3.1 Clone the Repository

```bash
git clone https://github.com/watchflow/watchflow.git
cd watchflow
```

### 3.2 Create Virtual Environment

Using uv (recommended):

```bash
# Create and activate virtual environment
uv venv
source .venv/bin/activate  # On macOS/Linux
# or
.venv\Scripts\activate     # On Windows

# Install dependencies
uv sync
```

Or using pip:

```bash
# Create virtual environment
python -m venv .venv

# Activate virtual environment
source .venv/bin/activate  # On macOS/Linux
# or
.venv\Scripts\activate     # On Windows

# Install dependencies
pip install -e ".[dev]"
```

### 3.3 Install and Setup ngrok

```bash
# Install ngrok
# On macOS with Homebrew:
brew install ngrok

# On other systems, download from https://ngrok.com/download

# Start ngrok tunnel to your local API
ngrok http 8000
```

Copy the ngrok HTTPS URL from the terminal output (e.g., `https://abc123.ngrok.io`)

### 3.4 Update GitHub App Webhook URL

1. Go back to your GitHub App settings
2. Update the **Webhook URL** to: `https://your-ngrok-url.ngrok.io/webhooks/github`
3. Save the changes

## Step 4: Configure Environment Variables

### 4.1 Create Environment File

```bash
cp .env.example .env
```

### 4.2 Configure Required Variables

Edit your `.env` file with the following configuration:

```bash
# GitHub Configuration (required)
APP_NAME_GITHUB=watchflow-dev
APP_CLIENT_ID_GITHUB=your_app_id_from_github_app_settings
APP_CLIENT_SECRET_GITHUB=your_client_secret_from_github_app_settings
PRIVATE_KEY_BASE64_GITHUB=your_base64_encoded_private_key
WEBHOOK_SECRET_GITHUB=your_webhook_secret_from_step_1

# AI Provider Selection
AI_PROVIDER=openai  # Options: openai, bedrock, vertex_ai

# Common AI Settings (defaults for all agents)
AI_MAX_TOKENS=4096
AI_TEMPERATURE=0.1

# OpenAI Configuration (when AI_PROVIDER=openai)
OPENAI_API_KEY=your_openai_api_key_here
OPENAI_MODEL=gpt-4.1-mini  # Optional, defaults to gpt-4.1-mini

# Engine Agent Configuration
AI_ENGINE_MAX_TOKENS=8000  # Default: 8000
AI_ENGINE_TEMPERATURE=0.1

# Feasibility Agent Configuration
AI_FEASIBILITY_MAX_TOKENS=4096
AI_FEASIBILITY_TEMPERATURE=0.1

# Acknowledgment Agent Configuration
AI_ACKNOWLEDGMENT_MAX_TOKENS=2000
AI_ACKNOWLEDGMENT_TEMPERATURE=0.1

# LangSmith Configuration
LANGCHAIN_TRACING_V2=false
LANGCHAIN_ENDPOINT=https://api.smith.langchain.com
LANGCHAIN_API_KEY=your_langsmith_api_key
LANGCHAIN_PROJECT=watchflow-dev

# CORS Configuration
CORS_HEADERS=["*"]
CORS_ORIGINS=["http://localhost:3000", "http://127.0.0.1:3000", "http://localhost:5500", "https://warestack.github.io", "https://watchflow.dev"]

# Repository Configuration
REPO_CONFIG_BASE_PATH=.watchflow
REPO_CONFIG_RULES_FILE=rules.yaml

# Logging Configuration
LOG_LEVEL=INFO
LOG_FORMAT=%(asctime)s - %(name)s - %(levelname)s - %(message)s

# Development Settings
DEBUG=false
ENVIRONMENT=development
```

### 4.3 Encode Your Private Key

Convert your GitHub App private key to base64:

**Option 1: Using command line (recommended):**

```bash
# Encode the private key file
cat /path/to/your-private-key.pem | base64 | tr -d '\n'
```

**Option 2: Using online tools:**

If you prefer using a web interface, you can use online base64 encoding tools like [base64encode.org](https://www.base64encode.org):

1. Open the `.pem` file in a text editor
2. Copy the entire content (including the BEGIN/END lines)
3. Paste it into the online encoder
4. Copy the base64 output

Copy the base64 output and use it as the value for `PRIVATE_KEY_BASE64_GITHUB` in your `.env` file.

**Security Note**: When using online tools, ensure you're using a reputable service and understand that your private key content will be processed by their servers. For maximum security, prefer the command-line method.

## Step 5: Install GitHub App on Repositories

### 5.1 Install the App

1. In your GitHub App settings, click on **"Install App"** in the left sidebar
2. Choose to install on your **organization** or **personal account**
3. Select **"Selected repositories"** and choose the repositories you want to monitor
4. Alternatively, select **"All repositories"** for organization-wide installation
5. Click **"Install"** to complete the installation

### 5.2 Verify Installation

1. Go to the repository settings of an installed repository
2. Navigate to **"Integrations"** → **"GitHub Apps"**
3. Verify that your `watchflow-dev` app is listed and active

## Step 6: Start Local Development Environment

### 6.1 Start the Watchflow API

```bash
# Using uv (recommended)
uv run uvicorn src.main:app --reload --host 0.0.0.0 --port 8000

# Using pip
uvicorn src.main:app --reload --host 0.0.0.0 --port 8000
```

The API will be available at `http://localhost:8000`

### 6.2 Verify API is Running

Open your browser and navigate to:

- `http://localhost:8000/docs` - Interactive API documentation
- `http://localhost:8000/health` - Health check endpoint

## Step 7: Test the Setup

### 7.1 Verify Webhook Connection

1. Ensure your local API is running on port 8000
2. Verify ngrok tunnel is active and forwarding to localhost:8000
3. Check the ngrok dashboard at `http://localhost:4040` for incoming requests

### 7.2 Test with Real Events

1. Go to one of your monitored repositories
2. Create a new pull request or push a commit
3. Check your local API logs to confirm webhook events are being received
4. Monitor the ngrok dashboard for incoming webhook requests

### 7.3 Test Rule Evaluation

**💡 Tip**: You can test your natural language rules at [watchflow.dev](https://watchflow.dev) to see if they're supported and get the generated YAML configuration. Then copy and paste it into your repository's `rules.yaml` file.

Create a test rule in a monitored repository by adding `.watchflow/rules.yaml`:

```yaml
rules:
  - description: Simple rule to test local setup
    enabled: true
    severity: medium
    event_types: [pull_request]
    parameters:
      test_param: "local_test"

  - description: All pull requests must have at least 1 approval
    enabled: true
    severity: high
    event_types: [pull_request]
    parameters:
      min_approvals: 1
```

## Troubleshooting

### Common Issues

#### Webhook not receiving events

- Verify ngrok is running and the URL is correct in GitHub App settings
- Check that the webhook secret matches your environment configuration
- Ensure your local API endpoint `/webhooks/github` is properly configured
- Check GitHub App webhook delivery logs in the app settings

#### Permission errors

- Double-check that all required permissions are granted to the GitHub App
- Verify the app is installed on the correct repositories/organization
- Ensure the private key is correctly encoded and configured

#### ngrok tunnel expires

- Free ngrok tunnels expire after 8 hours
- Restart ngrok and update the webhook URL in your GitHub App settings
- Consider upgrading to ngrok Pro for persistent URLs

#### AI Agent not working

- Verify your OpenAI API key is valid and has sufficient credits
- Check the AI model name in your configuration
- Review API logs for OpenAI-related errors

### Logs and Debugging

Monitor the following for debugging:

1. **Local API server logs** - Check your terminal running uvicorn
2. **ngrok request logs** - Run `ngrok http 8000 --log=stdout`
3. **GitHub App webhook delivery logs** - Available in GitHub App settings
4. **LangSmith traces** - If configured, view at [LangSmith Dashboard](https://smith.langchain.com/)

### Additional Debugging Commands

```bash
# Check if API is responding
curl http://localhost:8000/health

# Test rule evaluation endpoint
curl -X POST "http://localhost:8000/api/v1/rules/evaluate" \
  -H "Content-Type: application/json" \
  -d '{
    "rule_text": "All pull requests must have at least 2 approvals"
  }'

# View detailed logs
tail -f logs/watchflow.log
```

## Security Notes

- **Never commit your private keys or webhook secrets to version control**
- Use environment variables or secure secret management for all credentials
- Rotate webhook secrets periodically
- Limit GitHub App installation scope to only necessary repositories during development
- Keep your ngrok tunnel URL private and don't share it publicly
- Use separate GitHub Apps for development and production environments

## Next Steps

Once your local setup is working:

1. **Explore the API documentation** at `http://localhost:8000/docs`
2. **Create custom rules** in your test repositories
3. **Set up LangSmith** for AI agent debugging and monitoring
4. **Run the test suite** to verify everything is working: `pytest`
5. **Read the main development guide** in `DEVELOPMENT.md` for advanced topics

## Development Workflow Integration

After completing this setup, you can integrate with the standard development workflow:

```bash
# Format and lint code
uv run ruff format src/
uv run ruff check src/

# Run tests
pytest

# Install pre-commit hooks for code quality
uv run pre-commit install
uv run pre-commit install --hook-type commit-msg
```

For more advanced development topics, testing strategies, and deployment options, refer to the main [DEVELOPMENT.md](./DEVELOPMENT.md) guide.
