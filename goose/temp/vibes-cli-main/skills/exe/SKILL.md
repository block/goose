---
name: exe
description: Deploy a Vibes app to exe.dev VM hosting. Uses nginx on persistent VMs with SSH automation. Supports client-side multi-tenancy via subdomain-based Fireproof database isolation.
allowed-tools: Bash, Read, Glob, AskUserQuestion
---

## Deploy to exe.dev

Deploy your Vibes app to exe.dev, a VM hosting platform with persistent storage and HTTPS by default.

### Prerequisites

1. **SSH key** in `~/.ssh/` (id_ed25519, id_rsa, or id_ecdsa)
2. **exe.dev account** - run `ssh exe.dev` once to create your account and verify email
3. **Generated Vibes app** - an `index.html` file ready to deploy

### Gather Config Upfront

**Use AskUserQuestion to collect deployment config before running the deploy script.**

Use the AskUserQuestion tool with these questions:

```
Question 1: "What VM name should we use? (becomes yourname.exe.xyz)"
Header: "VM Name"
Options: Suggest based on app name from context + user enters via "Other"

Question 2: "Which file should we deploy?"
Header: "File"
Options: ["index.html (default)", "Other path"]

Question 3: "Does this app need AI features?"
Header: "AI"
Options: ["No", "Yes - I have an OpenRouter key"]

Question 4: "Is this a SaaS app with subdomain claiming?"
Header: "Registry"
Options: ["No - simple static deploy", "Yes - need Clerk keys for registry"]
```

#### After Receiving Answers

1. If AI enabled, ask for the OpenRouter API key
2. If Registry enabled, ask for Clerk PEM public key and webhook secret
3. **Proceed immediately to deploy** - no more questions

### Quick Deploy

```bash
cd "${CLAUDE_PLUGIN_ROOT}/scripts" && [ -d node_modules ] || npm install
node "${CLAUDE_PLUGIN_ROOT}/scripts/deploy-exe.js" --name myapp --file index.html
```

### What It Does

1. **Creates VM** on exe.dev via SSH CLI
2. **Starts nginx** (pre-installed on exeuntu image)
3. **Uploads** your index.html to `/var/www/html/`
4. **Generates HANDOFF.md** - context document for remote Claude
   - For Connect-enabled deployments, includes full Docker setup documentation (commands, troubleshooting, service endpoints)
5. **Makes VM public** via `ssh exe.dev share set-public <vmname>`
6. **Verifies** public access at `https://myapp.exe.xyz`

### AI-Enabled Apps

For apps using the `useAI` hook, deploy with the `--ai-key` flag:

```bash
node "${CLAUDE_PLUGIN_ROOT}/scripts/deploy-exe.js" --name myapp --file index.html --ai-key "sk-or-v1-..."
```

This sets up a **secure AI proxy**:
1. Installs Bun runtime
2. Creates `/home/exedev/proxy.js` - proxies `/api/ai/*` to OpenRouter
3. Configures systemd service for the proxy
4. Adds nginx reverse proxy from port 80/443 to Bun (port 3001)

**IMPORTANT:** Do not manually set up AI proxying. Manual nginx config changes can overwrite SSL settings and miss the Bun/proxy.js service. Always use the deploy script with `--ai-key`.

#### Multi-Tenant AI Apps

For SaaS apps with per-tenant AI:

```bash
node "${CLAUDE_PLUGIN_ROOT}/scripts/deploy-exe.js" --name myapp --file index.html --ai-key "sk-or-v1-..." --multi-tenant
```

### Registry Server

For SaaS apps using subdomain claiming (from `/vibes:sell`), deploy with Clerk credentials:

```bash
node "${CLAUDE_PLUGIN_ROOT}/scripts/deploy-exe.js" --name myapp --file index.html \
  --clerk-key "$(cat clerk-public-key.pem)" \
  --clerk-webhook-secret "whsec_xxx" \
  --reserved "admin,api,billing"
```

This sets up a **subdomain registry server**:
1. Installs Bun runtime to `/usr/local/bin/bun`
2. Creates `/var/www/registry-server.ts` with Clerk JWT verification
3. Configures systemd service (port 3002)
4. Adds nginx proxy for `/registry.json`, `/check/*`, `/claim`, `/webhook`

#### Registry Endpoints

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `/registry.json` | GET | None | Public read of all claims |
| `/check/{subdomain}` | GET | None | Check availability |
| `/claim` | POST | Bearer JWT | Claim subdomain for user |
| `/webhook` | POST | Svix sig | Clerk subscription events |

#### Getting the Clerk Public Key

The registry server needs Clerk's PEM public key to verify JWTs for the `/claim` endpoint.

**Option 1: From Clerk Dashboard**
1. Go to Clerk Dashboard → API Keys
2. Scroll to "PEM Public Key" or click "Show JWT Public Key"
3. Copy the full key (starts with `-----BEGIN PUBLIC KEY-----`)

**Option 2: From JWKS endpoint**
```bash
# Get your Clerk frontend API domain from dashboard
curl https://YOUR_CLERK_DOMAIN/.well-known/jwks.json
```
Then convert the JWK to PEM format using an online tool or `jose` CLI.

**Passing to deploy script:**
```bash
# From a file
node deploy-exe.js --clerk-key "$(cat clerk-public-key.pem)" ...

# Inline (escape newlines)
node deploy-exe.js --clerk-key "-----BEGIN PUBLIC KEY-----\nMIIB...\n-----END PUBLIC KEY-----" ...
```

**Manual configuration on server:**
```bash
ssh myapp.exe.dev
sudo nano /etc/registry.env
# Add: CLERK_PEM_PUBLIC_KEY="-----BEGIN PUBLIC KEY-----\n...\n-----END PUBLIC KEY-----"
sudo systemctl restart vibes-registry
```

### Continue Development on the VM

Claude is pre-installed on exe.dev VMs. After deployment, you can continue development remotely:

```bash
ssh myapp.exe.dev -t "cd /var/www/html && claude"
```

The HANDOFF.md file provides context about what was built, so Claude can continue meaningfully.

#### Manual Public Access

If the deploy script doesn't make the VM public automatically, run:

```bash
ssh exe.dev share set-public myapp
```

### Multi-Tenant Apps

For apps that need tenant isolation (e.g., `alice.myapp.com`, `bob.myapp.com`):

#### Client-Side Isolation

The same `index.html` serves all subdomains. JavaScript reads the hostname and uses the subdomain as a Fireproof database prefix:

```javascript
// In your app:
const hostname = window.location.hostname;
const subdomain = hostname.split('.')[0];
const dbName = `myapp-${subdomain}`;

// Each subdomain gets its own Fireproof database
const { database } = useFireproof(dbName);
```

#### Custom Domain Setup

1. **Add `--domain` flag:**
   ```bash
   node "${CLAUDE_PLUGIN_ROOT}/scripts/deploy-exe.js" --name myapp --domain myapp.com
   ```

2. **Configure wildcard DNS** at your DNS provider:
   ```
   *.myapp.com  CNAME  myapp.exe.xyz
   myapp.com    ALIAS  exe.xyz
   ```

3. **Set up wildcard SSL** on the VM:
   ```bash
   ssh myapp.exe.dev
   sudo apt install certbot
   sudo certbot certonly --manual --preferred-challenges dns \
     -d "myapp.com" -d "*.myapp.com"
   ```

### CLI Options

| Option | Description |
|--------|-------------|
| `--name <vm>` | VM name (required) |
| `--file <path>` | HTML file to deploy (default: index.html) |
| `--domain <domain>` | Custom domain for wildcard setup |
| `--ai-key <key>` | OpenRouter API key for AI features |
| `--multi-tenant` | Enable subdomain-based multi-tenancy |
| `--tenant-limit <$>` | Credit limit per tenant in dollars (default: 5) |
| `--clerk-key <pem>` | Clerk PEM public key for JWT verification |
| `--clerk-webhook-secret <secret>` | Clerk webhook signing secret |
| `--reserved <list>` | Comma-separated reserved subdomain names |
| `--preallocated <list>` | Pre-claimed subdomains (format: `sub:user_id`) |
| `--dry-run` | Show commands without executing |
| `--skip-verify` | Skip deployment verification |

### Redeployment

After making changes, redeploy with:

```bash
node "${CLAUDE_PLUGIN_ROOT}/scripts/deploy-exe.js" --name myapp
```

### SSH Access

Access your VM directly:

```bash
ssh myapp.exe.dev
```

### Architecture

```
exe.dev VM (exeuntu image)
├── nginx (serves all subdomains via server_name _)
├── claude (pre-installed CLI)
├── /usr/local/bin/bun  ← Bun runtime (system-wide)
├── /var/www/html/
│   ├── index.html   ← Your Vibes app
│   └── HANDOFF.md   ← Context for remote Claude
├── (with --ai-key)
│   ├── /opt/vibes/proxy.js  ← AI proxy service (port 3001)
│   └── vibes-proxy.service  ← systemd unit
└── (with --clerk-key)
    ├── /var/www/registry-server.ts  ← Registry service (port 3002)
    ├── /var/www/html/registry.json  ← Subdomain claims data
    └── vibes-registry.service       ← systemd unit
```

#### Port Assignments

| Service | Port | Purpose |
|---------|------|---------|
| AI Proxy | 3001 | OpenRouter proxy for `useAI` hook |
| Registry | 3002 | Subdomain claim/check API |

- **No server-side logic** - pure static hosting (unless using AI proxy)
- **Persistent disk** - survives restarts
- **HTTPS by default** - exe.dev handles SSL for *.exe.xyz
- **Claude pre-installed** - continue development on the VM

### Post-Deploy Debugging

After deployment, **always work with local files** - they are the source of truth. SSHing to read deployed files is slow and wastes tokens.

| Task | Use Local | Use SSH |
|------|-----------|---------|
| Editing/debugging code | ✅ Always | ❌ Never |
| Checking console errors | ✅ Local file | ❌ No need |
| Verifying deploy | ❌ | ✅ `curl https://vm.exe.xyz` |
| Server-specific issues | ❌ | ✅ Only if local works but remote doesn't |

**To redeploy after local fixes:**
```bash
node "${CLAUDE_PLUGIN_ROOT}/scripts/deploy-exe.js" --name <vmname> --file index.html
```

### SSL Configuration

The deploy script preserves existing SSL by using include files for AI proxy config. When manually editing nginx:

1. **Never replace the entire config** - only add/modify specific blocks
2. **Check for SSL first:** Look for `listen 443 ssl` in the config
3. **Use includes:** Put new configs in `/etc/nginx/conf.d/` or separate files
4. **Test before reload:** `sudo nginx -t`

#### exe.dev Home Directory

The home directory on exe.dev VMs is `/home/exedev` (not `~` expansion). For manual file operations:

```bash
# Use explicit path or /tmp for staging
scp file.html vmname.exe.xyz:/home/exedev/
# or
scp file.html vmname.exe.xyz:/tmp/
```

---

### What's Next?

After successful deployment, present these options using AskUserQuestion:

```
Question: "Your app is live at https://${name}.exe.xyz! What's next?"
Header: "Next"
Options:
- Label: "Share my URL"
  Description: "Get the shareable link for your app. I'll confirm the public URL and you can send it to anyone - they'll see your app immediately with full functionality."

- Label: "Make changes and redeploy"
  Description: "Continue iterating locally. Edit your files here, then run deploy again to push updates. The VM keeps running so there's zero downtime during updates."

- Label: "Continue development on VM"
  Description: "Work directly on the server. SSH in and use the pre-installed Claude to make changes live. Great for server-specific debugging or when you want changes to persist immediately."

- Label: "I'm done for now"
  Description: "Wrap up this session. Your app stays live at the URL - it runs 24/7 on exe.dev's persistent VMs. Come back anytime to make updates."
```

**After user responds:**
- "Share URL" → Confirm "Your app is live at https://${name}.exe.xyz - share this link!"
- "Make changes" → Acknowledge, stay ready for local edits
- "Continue on VM" → Provide: `ssh ${name}.exe.dev -t "cd /var/www/html && claude"`
- "I'm done" → Confirm app stays live, wish them well
