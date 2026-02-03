---
name: connect
description: Deploy Fireproof Connect to a dedicated Studio VM on exe.dev. Sets up cloud sync backend for all your Vibes apps.
allowed-tools: Bash, Read, Glob, AskUserQuestion
---

## Deploy Connect Studio

Deploy Fireproof Connect to exe.dev as your personal sync backend. Your Studio VM runs the full Fireproof sync stack and can be used by multiple apps.

### Prerequisites

1. **SSH key** in `~/.ssh/` (id_ed25519, id_rsa, or id_ecdsa)
2. **exe.dev account** - run `ssh exe.dev` once to create your account
3. **Clerk credentials** from [clerk.com](https://clerk.com):
   - Publishable Key (pk_test_... or pk_live_...)
   - Secret Key (sk_test_... or sk_live_...)

### Gather Config

**Use AskUserQuestion to collect deployment config before running the deploy script.**

```
Question 1: "What codename for your Studio? (becomes <codename>.exe.xyz)"
Header: "Studio"
Options: Suggest "${username}-studio" + user enters via "Other"

Question 2: "Do you have your Clerk keys ready?"
Header: "Clerk"
Options: ["Yes, I have them", "No, I need to get them first"]
```

If user needs Clerk keys, provide these instructions:
1. Go to [clerk.com](https://clerk.com) and sign in
2. Select your application (or create one)
3. Go to API Keys in the sidebar
4. Copy both the Publishable Key and Secret Key

**After receiving the codename**, ask for the keys:

```
Question: "Paste your Clerk Publishable Key (starts with pk_test_ or pk_live_)"
Header: "Publishable"
Options: [User enters via "Other"]

Question: "Paste your Clerk Secret Key (starts with sk_test_ or sk_live_)"
Header: "Secret"
Options: [User enters via "Other"]
```

### Deploy Command

```bash
cd "${CLAUDE_PLUGIN_ROOT}/scripts" && [ -d node_modules ] || npm install
node "${CLAUDE_PLUGIN_ROOT}/scripts/deploy-connect.js" \
  --studio <codename> \
  --clerk-publishable-key "pk_test_..." \
  --clerk-secret-key "sk_test_..."
```

### What It Does

1. **SSH to `<studio>.exe.xyz`** - Creates VM if needed
2. **Clone fireproof repo** - `selem/docker-for-all` branch to `/opt/fireproof`
3. **Generate security tokens** - Session tokens and device CA keys
4. **Create `.env`** - All credentials for Docker services
5. **Run `./docker/start.sh`** - Starts the full Fireproof stack
6. **Wait for services** - Confirms port 8080 is responding
7. **Write local `.connect`** - Saves studio info for future reference

### Architecture

```
Studio VM (<codename>.exe.xyz)
├── /opt/fireproof/
│   ├── docker-compose.yaml (from repo)
│   ├── docker/
│   │   ├── nginx.conf (routes all traffic)
│   │   └── start.sh (orchestrates services)
│   └── .env (generated credentials)
└── Docker services (port 8080 exposed)
    ├── nginx proxy
    ├── dashboard (internal 7370)
    └── cloud-backend (internal 8909)
```

### Public URLs

After deployment, your Studio exposes:

| Endpoint | URL | Purpose |
|----------|-----|---------|
| Token API | `https://<studio>.exe.xyz/api` | Token issuance for auth |
| Cloud Sync | `fpcloud://<studio>.exe.xyz?protocol=wss` | Real-time sync |

### Local `.connect` File

The deploy script creates a `.connect` file in your project:

```
studio: <codename>
api_url: https://<codename>.exe.xyz/api
cloud_url: fpcloud://<codename>.exe.xyz?protocol=wss
clerk_publishable_key: pk_test_...
```

This file is gitignored and used to auto-configure app deployments.

### Update Your App's Environment

After deploying Connect, update your app's `.env`:

```bash
VITE_CLERK_PUBLISHABLE_KEY=pk_test_...
VITE_API_URL=https://<studio>.exe.xyz/api
VITE_CLOUD_URL=fpcloud://<studio>.exe.xyz?protocol=wss
```

### Troubleshooting

**Check Docker status:**
```bash
ssh <studio>.exe.xyz "cd /opt/fireproof && sudo docker compose ps"
```

**View logs:**
```bash
ssh <studio>.exe.xyz "cd /opt/fireproof && sudo docker compose logs -f"
```

**Restart services:**
```bash
ssh <studio>.exe.xyz "cd /opt/fireproof && sudo docker compose restart"
```

### CLI Options

| Option | Description |
|--------|-------------|
| `--studio <name>` | Studio VM name (required) |
| `--clerk-publishable-key <key>` | Clerk publishable key (required) |
| `--clerk-secret-key <key>` | Clerk secret key (required) |
| `--dry-run` | Show what would be done without executing |

---

### What's Next?

After successful deployment, present these options using AskUserQuestion:

```
Question: "Your Connect Studio is live at https://${studio}.exe.xyz! What's next?"
Header: "Next"
Options:
- Label: "Deploy an app that uses this Studio"
  Description: "Generate and deploy a Vibes app configured to sync through your Studio. I'll set up the environment automatically."

- Label: "Update an existing app to use Connect"
  Description: "Configure an existing app's .env to point to your new Studio for cloud sync."

- Label: "I'm done for now"
  Description: "Your Studio is running 24/7 on exe.dev. Any app configured with these URLs will sync through it."
```
