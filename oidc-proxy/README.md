# OIDC Proxy

A Cloudflare Worker that authenticates GitHub Actions OIDC tokens and proxies requests to an upstream API with an injected API key. This lets CI workflows call APIs without storing long-lived secrets in GitHub.

## How it works

```
GitHub Actions (OIDC token) → Worker (validate JWT, inject API key) → Upstream API
```

1. A GitHub Actions workflow mints an OIDC token with a configured audience
2. The workflow sends requests to this proxy, passing the OIDC token as the API key
3. The worker validates the JWT against GitHub's JWKS, checks issuer/audience/age/repo
4. If valid, the request is forwarded to the upstream API with the real API key injected

## Setup

```bash
cd oidc-proxy
npm install
```

## Configuration

Edit `wrangler.toml` for your upstream:

| Variable | Description |
|---|---|
| `OIDC_ISSUER` | `https://token.actions.githubusercontent.com` |
| `OIDC_AUDIENCE` | The audience your workflow requests (e.g. `goose-oidc-proxy`) |
| `MAX_TOKEN_AGE_SECONDS` | Max age of OIDC token in seconds (default: `1200` = 20 min) |
| `ALLOWED_REPOS` | *(optional)* Comma-separated `owner/repo` list |
| `ALLOWED_REFS` | *(optional)* Comma-separated allowed refs |
| `UPSTREAM_URL` | The upstream API base URL |
| `UPSTREAM_AUTH_HEADER` | Header name for the API key (e.g. `x-api-key`, `Authorization`) |
| `UPSTREAM_AUTH_PREFIX` | *(optional)* Prefix before the key (e.g. `Bearer `) — omit for raw value |
| `CORS_ORIGIN` | *(optional)* Allowed CORS origin |
| `CORS_EXTRA_HEADERS` | *(optional)* Additional CORS allowed headers |

Set your upstream API key as a secret:

```bash
npx wrangler secret put UPSTREAM_API_KEY
```

### Example: Anthropic

```toml
UPSTREAM_URL = "https://api.anthropic.com"
UPSTREAM_AUTH_HEADER = "x-api-key"
CORS_EXTRA_HEADERS = "anthropic-version"
```

### Example: OpenAI-compatible

```toml
UPSTREAM_URL = "https://api.openai.com"
UPSTREAM_AUTH_HEADER = "Authorization"
UPSTREAM_AUTH_PREFIX = "Bearer "
```

## Usage in GitHub Actions

```yaml
permissions:
  id-token: write

steps:
  - name: Get OIDC token
    id: oidc
    uses: actions/github-script@v7
    with:
      script: |
        const token = await core.getIDToken('goose-oidc-proxy');
        core.setOutput('token', token);
        core.setSecret(token);

  - name: Call API through proxy
    env:
      ANTHROPIC_BASE_URL: https://oidc-proxy.your-subdomain.workers.dev
      ANTHROPIC_API_KEY: ${{ steps.oidc.outputs.token }}
    run: goose run --recipe my-recipe.yaml
```

## Local development

Local testing uses a real OIDC flow — a local issuer serves discovery/JWKS endpoints
and a companion script mints signed JWTs against the same keypair.

### 1. Create `.dev.vars`

```bash
cat > .dev.vars <<'EOF'
UPSTREAM_API_KEY=sk-ant-your-real-key
OIDC_ISSUER=http://localhost:8788
EOF
```

This overrides the production `OIDC_ISSUER` (GitHub) with the local issuer.
All other settings come from `wrangler.toml` as normal.

### 2. Start the local OIDC issuer

In one terminal:

```bash
npm run issuer
# → OIDC issuer running on http://localhost:8788
```

On first run this generates an RSA keypair in `dev/.keys/` (gitignored).
The keypair persists across restarts so previously minted tokens remain valid.

### 3. Start the proxy

In a second terminal:

```bash
npm run dev
# → Ready on http://localhost:8787
```

The proxy reads `OIDC_ISSUER=http://localhost:8788` from `.dev.vars`,
so it validates tokens against the local issuer instead of GitHub.

### 4. Mint a token and use it

```bash
# With goose
ANTHROPIC_BASE_URL=http://localhost:8787 \
ANTHROPIC_API_KEY=$(npm run --silent mint-token) \
goose session

# With curl
TOKEN=$(npm run --silent mint-token)
curl --compressed http://localhost:8787/v1/messages \
  -H "x-api-key: $TOKEN" \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d '{"model":"claude-sonnet-4-20250514","max_tokens":128,"messages":[{"role":"user","content":"ping"}]}'
```

The token defaults to 20 minutes TTL. Options:

```bash
npm run --silent mint-token -- --ttl 3600         # 1 hour
npm run --silent mint-token -- --repo other/repo  # different repo claim
npm run --silent mint-token -- --audience custom   # different audience
```

## Testing

```bash
npm test
```

## Deploy

```bash
npx wrangler secret put UPSTREAM_API_KEY
npm run deploy
```

## Token age vs expiry

GitHub OIDC tokens expire after ~5 minutes. For longer-running jobs, set `MAX_TOKEN_AGE_SECONDS` to allow recently-expired tokens. When set, the proxy checks the token's `iat` (issued-at) claim instead of `exp`.
