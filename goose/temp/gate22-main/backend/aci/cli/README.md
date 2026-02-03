# ACI CLI

## Example Usage

### Upsert an MCP Server

```bash
docker compose exec runner python -m aci.cli mcp upsert-server --server-file ./mcp_servers/notion/server.json

docker compose exec runner python -m aci.cli mcp upsert-server --server-file ./mcp_servers/notion/server.json --skip-dry-run
```

### Upsert MCP Tools

```bash
docker compose exec runner python -m aci.cli mcp upsert-tools --tools-file ./mcp_servers/notion/tools.json

docker compose exec runner python -m aci.cli mcp upsert-tools --tools-file ./mcp_servers/notion/tools.json --skip-dry-run
```

#### Create a mock organization, teams and users setting

```bash
docker compose exec runner python -m aci.cli create-mock-org-teams-users
```
