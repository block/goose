# Goose ACP CLI

Terminal CLI client for goose using ACP over HTTP.

## Usage

Start the server:
```bash
cargo run -p goose-acp-server
```

Run the CLI:
```bash
cd ui/acp-cli
npm install
npm start
```

Options:
- `-s, --server <url>` - Server URL (default: http://127.0.0.1:3284)
- `-p, --prompt <text>` - One-shot mode
- `-h, --help` - Help

## Endpoints

- `POST /acp/session` - Create session
- `POST /acp/session/{id}/message` - Send message
- `GET /acp/session/{id}/stream` - SSE stream
