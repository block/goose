# MCP Gateway Backend

[![Backend Checks](https://github.com/aipotheosis-labs/mcp-gateway/actions/workflows/backend-checks.yml/badge.svg)](https://github.com/aipotheosis-labs/mcp-gateway/actions/workflows/backend-checks.yml)
[![Backend Deployment](https://github.com/aipotheosis-labs/mcp-gateway/actions/workflows/backend-deployment.yml/badge.svg)](https://github.com/aipotheosis-labs/mcp-gateway/actions/workflows/backend-deployment.yml)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

## Overview

- [Code Structure](#code-structure)
- [Development Setup](#development-setup)
   - [Prerequisites](#prerequisites)
   - [Code Style](#code-style)
   - [IDE Configuration](#ide-configuration)
   - [Getting Started](#getting-started)
   - [Running Tests](#running-tests)
- [Database Management](#database-management)
   - [Working with Migrations](#working-with-migrations)
- [Contributing](#contributing)
- [License](#license)


## Code Structure

The backend consists of several main components:

- **ACI**: The root package for the MCP Gateway.
  - **Alembic**: Database migrations
  - **CLI**: Command-line interface for local development, admin operations, etc.
  - **Common**: Shared code and utilities used across components
  - **Control Plane**: The Service powering the core operations of the gateway
  - **MCP**: The Service for hosting Remote MCP Servers
  - **Virtual MCP**: The Service for hosting Virtual MCP Servers
- **MCP Server**: MCP servers indexed by the MCP Gateway
- **Virtual MCP Server**: Virtual MCP servers hosted by **Virtual MCP** service

## Development Setup

### Prerequisites

- Python 3.12+
- Docker and Docker Compose
- `uv` package manager

### Code Style

We follow strict code quality standards:

- **Formatting & Linting**: We use `ruff` for code formatting and linting
- **Type Checking**: We use `mypy` for static type checking
- **Pre-commit Hooks**: Install with `pre-commit install`

### IDE Configuration

For VS Code or VS Code compatible IDEs, use [`.vscode`](../.vscode) folder for configuration.

### Getting Started

1. Clone the repository:

   ```bash
   git clone https://github.com/aipotheosis-labs/mcp-gateway.git
   cd mcp-gateway/backend
   ```

1. Install dependencies and activate virtual environment:

   ```bash
   uv sync
   source .venv/bin/activate
   ```

1. Install `pre-commit` hooks:

   ```bash
   pre-commit install
   ```

1. Set up environment variables for **local** development:

   ```bash
   cp .env.example .env.local
   ```

   Most sensitive variables and dummy values are already defined in `.env.example`, so you only need to set the following env vars in `.env.local`:

   - `CONTROL_PLANE_OPENAI_API_KEY`
   - `MCP_OPENAI_API_KEY`
   - `CLI_OPENAI_API_KEY`

   You might need to set AWS related variables if you want to test relevant features
   - `AWS_ACCESS_KEY_ID`
   - `AWS_SECRET_ACCESS_KEY`
   - `AWS_DEFAULT_REGION`

1. Start services with Docker Compose:

   ```bash
   docker compose up --build
   # or if you want to enable OpenTelemetry: docker compose --profile otel up --build
   ```

   This will start:

   - `control-plane`: Backend API service
   - `mcp`: MCP service
   - `virtual-mcp`: Virtual MCP service
   - `runner`: Container for running commands like pytest, cli commands or scripts
   - `test-runner`: Container for running pytest
   - `db`: PostgreSQL database for local development
   - `test-db`: PostgreSQL database for running pytest

1. Seed the database with cli commands:

   - Insert mcp servers and tools

      ```bash
      for server_dir in ./mcp_servers/*/; do
        server_file="${server_dir}server.json"
        tools_file="${server_dir}tools.json"
        dotenv run python -m aci.cli mcp upsert-server --server-file "$server_file"
        dotenv run python -m aci.cli mcp upsert-tools --tools-file "$tools_file"
      done
      ```

   - Insert virtual mcp servers and tools

      ```bash
      for server_dir in ./virtual_mcp_servers/*/; do
        server_file="${server_dir}server.json"
        tools_file="${server_dir}tools.json"
        dotenv run python -m aci.cli virtual-mcp upsert-server --server-file "$server_file"
        dotenv run python -m aci.cli virtual-mcp upsert-tools --tools-file "$tools_file"
      done
      ```

1. (Optional) Connect to the database using a GUI client (e.g., `Beekeeper Studio`)

   - Parameters for the db connection can be found in the `.env.local` file you created in step 4.

1. Access the API documentation at:

   ```bash
   http://localhost:8000/v1/control-plane/docs
   ```

### Running Tests

Run the test suite in an ephemeral container (automatically applies migrations before executing tests):

```bash
docker compose exec test-runner pytest
```

## Database Management

### Working with Migrations

When making changes to database models:

1. Check for detected changes:

   ```bash
   docker compose exec runner alembic check
   ```

1. Generate a migration:

   ```bash
   docker compose exec runner alembic revision --autogenerate -m "description of changes"
   ```

1. Manually review and edit the generated file in `database/alembic/versions/` if needed to add custom changes, e.g.,:

   - pgvector library imports
   - Index creation/deletion
   - Vector extension setup
   - Other database-specific operations

1. Apply the migration (to the local db):

   ```bash
   docker compose exec runner alembic upgrade head
   ```

1. To revert the latest migration:

   ```bash
   docker compose exec runner alembic downgrade -1
   ```

## Contributing

Please refer to the [Contributing Guide](../CONTRIBUTING.md) for details on making contributions to this project.

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](../LICENSE) file for details.
