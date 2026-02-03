# Development Guide

This guide covers setting up the Watchflow development environment for local development and testing.

**Direction (for contributors):** Watchflow is a **rule engine** for GitHub—rules in YAML, enforcement on PR and push. The hot path is **condition-based** (no LLM for “did this PR violate the rule?”). Optional AI is used for repo analysis and feasibility suggestions. We aim for maintainer-first docs and code: tech-forward, slightly retro, no marketing fluff. See [README](README.md) and [docs](docs/) for the supported logic and architecture.

## Quick Start

🚀 **New to Watchflow?** Start with our [Local Development Setup Guide](./LOCAL_SETUP.md) for a complete end-to-end setup including GitHub App configuration and webhook testing.

This document covers advanced development topics and workflow.

## Prerequisites

- Python 3.12 or higher
- [uv](https://docs.astral.sh/uv/) package manager (recommended) or pip
- [ngrok](https://ngrok.com/) for local webhook testing
- [LangSmith](https://langsmith.com/) account for AI agent debugging
- GitHub App credentials

## Development Setup

### 1. Clone and Setup

```bash
git clone https://github.com/watchflow/watchflow.git
cd watchflow
```

### 2. Create Virtual Environment

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

### 3. Environment Configuration

Create and configure your environment file:

```bash
cp .env.example .env
```

Required environment variables:

```bash
# GitHub Configuration (required)
APP_NAME_GITHUB=your_app_name
APP_CLIENT_ID_GITHUB=your_client_id
APP_CLIENT_SECRET_GITHUB=your_client_secret
PRIVATE_KEY_BASE64_GITHUB=your_private_key_base64
WEBHOOK_SECRET_GITHUB=your_webhook_secret

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

### 4. GitHub App Setup

Create a GitHub App for development:

1. Go to [GitHub Developer Settings](https://github.com/settings/apps)
2. Click "New GitHub App"
3. Configure with these settings:
   - **App name**: `watchflow-dev`
   - **Homepage URL**: `http://localhost:8000`
   - **Webhook URL**: `https://your-ngrok-url.ngrok.io/webhooks/github`
   - **Webhook secret**: Generate a secure random string

4. **Permissions**:
   - Repository permissions:
     - Checks: Read & write
     - Contents: Read-only
     - Deployments: Read & write
     - Issues: Read & write
     - Metadata: Read-only
     - Pull requests: Read & write
     - Commit statuses: Read & write

5. **Subscribe to events**:
   - Check run
   - Deployment
   - Deployment protection rule
   - Deployment review
   - Deployment status
   - Issue comment
   - Pull request
   - Push
   - Status

6. **Generate private key** and encode it:

   ```bash
   cat /path/to/private-key.pem | base64 | tr -d '\n'
   ```

### 5. Local Webhook Testing with ngrok

Install ngrok and start a tunnel:

```bash
# Install ngrok
npm install -g ngrok

# Start tunnel
ngrok http 8000
```

Update your GitHub App webhook URL with the ngrok URL.

### 6. LangSmith Integration

For debugging AI agents:

1. Create a [LangSmith](https://langsmith.com/) account
2. Get your API key
3. Add to environment:

   ```bash
   LANGCHAIN_TRACING_V2=true
   LANGCHAIN_API_KEY=your-langsmith-api-key
   LANGCHAIN_PROJECT=watchflow-dev
   ```

## Running the Application

### Development Server

```bash
# Using uv
uv run uvicorn src.main:app --reload --host 0.0.0.0 --port 8000

# Using pip
uvicorn src.main:app --reload --host 0.0.0.0 --port 8000
```

The API will be available at `http://localhost:8000`

### Docker Development

```bash
# Build and run with Docker Compose
docker-compose up --build

# Or build manually
docker build -t watchflow-dev .
docker run -p 8000:8000 watchflow-dev
```

## Development Workflow

### Code Quality

```bash
# Format code
uv run black src/
uv run isort src/

# Lint code
uv run ruff check src/
uv run ruff format src/

# Type checking
uv run mypy src/
```

### Pre-commit Hooks

This project uses pre-commit hooks to ensure code quality and consistency. The hooks automatically run on every commit and include:

- **Trailing whitespace removal** - Cleans up extra whitespace
- **End of file fixer** - Ensures files end with newlines
- **YAML/JSON validation** - Checks syntax
- **Ruff formatting and linting** - Formats code and sorts imports
- **Conventional commit validation** - Ensures commit messages follow conventional format

#### Setup Pre-commit Hooks

```bash
# Install pre-commit hooks (run once after cloning)
uv run pre-commit install

# Also install commit message validation
uv run pre-commit install --hook-type commit-msg
```

#### Using Pre-commit Hooks

```bash
# Hooks run automatically on commit, but you can run them manually:
uv run pre-commit run --all-files

# Run on specific files
uv run pre-commit run --files src/main.py

# Skip hooks for a commit (not recommended)
git commit --no-verify -m "commit message"
```

The hooks will prevent commits if any issues are found. Most formatting issues are automatically fixed, so you just need to stage the changes and commit again.

### Testing

The project includes comprehensive tests that run **without making real API calls** by default:

### Running Tests

CI runs tests the same way (see [.github/workflows/tests.yaml](.github/workflows/tests.yaml)). To run tests locally **like CI** (and avoid the wrong interpreter):

1. **Use this repo's environment only.** If you have another project's venv activated (e.g. PyCharm's watchflow), **deactivate it first** so `uv` uses this project's `.venv`:
   ```powershell
   deactivate
   ```
2. **From this repo root** (`D:\watchflow-env\watchflow` or `watchflow/`):
   ```bash
   uv sync --all-extras
   uv run pytest tests/unit/ tests/integration/ -v
   ```

If you skip step 1 and another venv is activated, `uv run pytest` can still use that interpreter and you may see `ModuleNotFoundError: No module named 'structlog'` or `respx`. Deactivating ensures `uv` creates/uses the venv in **this** repo.

**Alternative (always use this repo's venv):** From this repo root, run pytest with the project's Python explicitly so the interpreter is unambiguous:
```powershell
# Windows
.\.venv\Scripts\python.exe -m pytest tests/unit/ tests/integration/ -v
```

```bash
# Install deps (matches CI)
uv sync --all-extras

# Run all tests (same as GitHub Action)
uv run pytest tests/unit/ tests/integration/ -v

# Run only unit tests
uv run pytest tests/unit/ -v

# Run only integration tests
uv run pytest tests/integration/ -v
```

### Test Structure

```txt
tests/
├── unit/                     # ⚡ Fast unit tests (mocked OpenAI)
│   └── test_feasibility_agent.py
└── integration/              # Full HTTP stack tests (mocked OpenAI)
    └── test_rules_api.py
```

### Real API Testing (Local Development Only)

If you want to test with **real OpenAI API calls** locally:

```bash
# Set environment variables
export OPENAI_API_KEY="your-api-key"
export INTEGRATION_TEST_REAL_API=true

# Run integration tests with real API calls (costs money!)
pytest tests/integration/ -m integration
```

_Note: Real API tests make actual OpenAI calls and will cost money. They're disabled by default in CI/CD._

## Testing AI Agents

### Rule Evaluation Testing

Test rule evaluation with the API:

```bash
curl -X POST "http://localhost:8000/api/v1/rules/evaluate" \
  -H "Content-Type: application/json" \
  -d '{
    "rule_text": "All pull requests must have at least 2 approvals"
  }'
```

### LangSmith Debugging

With LangSmith configured, you can:

1. View agent execution traces in the LangSmith dashboard
2. Debug prompt engineering and agent logic
3. Monitor performance and token usage
4. Iterate on agent behavior

### Local Rule Testing

**💡 Tip**: Test your natural language rules at [watchflow.dev](https://watchflow.dev) to verify they're supported and get the generated YAML. Copy the output directly into your `rules.yaml` file.

Create a test repository with `.watchflow/rules.yaml`:

```yaml
rules:
  - description: Test rule for development
    enabled: true
    severity: medium
    event_types: [pull_request]
    parameters:
      test_param: "test_value"

  - description: All PRs must pass required status checks
    enabled: true
    severity: high
    event_types: [pull_request]
    parameters:
      required_checks: ["ci/test", "lint"]
```

## Debugging

### Logging

Configure logging level in `.env`:

```bash
LOG_LEVEL=DEBUG
```

### Agent Debugging

Enable detailed agent logging:

```bash
# Add to .env
AGENT_DEBUG=true
LANGCHAIN_TRACING_V2=true
LANGCHAIN_API_KEY=your-langsmith-key
```

### Webhook Debugging

Test webhook delivery:

```bash
# Use ngrok webhook inspection
# Visit https://your-ngrok-url.ngrok.io/inspect/http
```

## Common Issues

### GitHub App Permissions

If webhooks aren't being received:

1. Verify webhook URL is accessible
2. Check GitHub App permissions
3. Verify webhook secret matches

### AI Agent Issues

If agents aren't working:

1. Verify OpenAI API key
2. Check LangSmith configuration
3. Review agent logs for errors

### Development Environment

If dependencies aren't working:

1. Ensure Python 3.12+
2. Try recreating virtual environment
3. Check uv/pip installation

## Performance Testing

### Load Testing

```bash
# Install locust
pip install locust

# Run load test
locust -f load_test.py --host=http://localhost:8000
```

### Agent Performance

Monitor agent performance with LangSmith:

- Token usage per request
- Response times
- Error rates
- Cost analysis

## Documentation

- [API Documentation](http://localhost:8000/docs) - Auto-generated FastAPI docs
- [LangSmith Dashboard](https://smith.langchain.com/) - AI agent debugging
- [GitHub App Documentation](https://docs.github.com/en/apps) - GitHub App setup
