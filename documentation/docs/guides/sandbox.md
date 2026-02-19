# macOS Sandbox for goosed

goose includes an optional macOS sandbox that restricts the goosed process using Apple's seatbelt (`sandbox-exec`) and routes all network traffic through a local egress proxy. This limits what the agent can do on your system — blocking sensitive file writes, raw sockets, tunneling tools, and unapproved network destinations.

> **Requirements:** macOS only. The sandbox relies on `/usr/bin/sandbox-exec` which is only available on macOS.

## Quick Start

Set the environment variable before launching the goose desktop app:

```bash
GOOSE_SANDBOX=true
```

Then start the desktop app as normal. goose will:

1. Generate a seatbelt sandbox profile
2. Start a local HTTP CONNECT proxy on localhost
3. Launch goosed inside `sandbox-exec`, forcing all traffic through the proxy

If `sandbox-exec` is not available (e.g. you're on Linux), goose will fail fast with a clear error rather than running unsandboxed.

## What Gets Restricted

### File System (seatbelt)

By default, the sandbox blocks writes to:

| Path | Purpose |
|------|---------|
| `~/.ssh/` | Prevent SSH key tampering |
| `~/.bashrc`, `~/.zshrc`, `~/.bash_profile`, `~/.zprofile` | Prevent shell config injection |
| `~/.config/goose/sandbox/` | Protect sandbox config from the sandboxed process |
| `~/.config/goose/config.yaml` | Protect goose config |

### Network (seatbelt)

All direct network access is denied. The only allowed paths are:

- **Localhost** — so the process can reach the egress proxy and its own server port
- **Unix sockets** — for local IPC
- **mDNSResponder** — for DNS resolution

Everything else must go through the proxy.

### Process Restrictions (seatbelt)

- **Tunneling tools blocked:** `nc`, `ncat`, `netcat`, `socat`, `telnet` — prevents the agent from bypassing the proxy
- **Raw sockets blocked:** `SOCK_RAW` on `AF_INET`/`AF_INET6` — prevents raw packet crafting
- **Kernel extensions blocked:** `system-kext-load` denied

### Network (proxy)

The egress proxy checks connections in this order:

1. **Loopback detection** — prevents using the proxy as a relay back to localhost
2. **Raw IP blocking** — connections to bare IP addresses (no domain) are blocked
3. **Domain blocklist** — domains listed in `blocked.txt` are denied (including all subdomains)
4. **SSH/Git host restrictions** — SSH ports (22, 2222, 7999) are restricted to known git hosts
5. **LaunchDarkly allowlist** (optional) — dynamic egress control via feature flag

## Configuration

All configuration is via environment variables. Defaults are designed to be secure out of the box.

### Core

| Variable | Default | Description |
|----------|---------|-------------|
| `GOOSE_SANDBOX` | `false` | Set to `true` or `1` to enable the sandbox |

### Seatbelt Profile

| Variable | Default | Description |
|----------|---------|-------------|
| `GOOSE_SANDBOX_PROTECT_FILES` | `true` | Write-protect `~/.ssh` and shell configs. Set to `false` to disable |
| `GOOSE_SANDBOX_BLOCK_RAW_SOCKETS` | `true` | Block `SOCK_RAW`. Set to `false` to disable |
| `GOOSE_SANDBOX_BLOCK_TUNNELING` | `true` | Block `nc`/`netcat`/`socat`/`telnet`. Set to `false` to disable |

### Proxy

| Variable | Default | Description |
|----------|---------|-------------|
| `GOOSE_SANDBOX_ALLOW_IP` | `false` | Set to `true` to allow connections to raw IP addresses |
| `GOOSE_SANDBOX_BLOCK_LOOPBACK` | `false` | Set to `true` to block loopback relay through the proxy |
| `GOOSE_SANDBOX_ALLOW_SSH` | `true` | Set to `false` to block all SSH traffic |
| `GOOSE_SANDBOX_GIT_HOSTS` | built-in list | Comma-separated list of allowed SSH git hosts (e.g. `github.com,gitlab.com`) |
| `GOOSE_SANDBOX_SSH_ALL_HOSTS` | `false` | Set to `true` to allow SSH to any host (not just git hosts) |

### LaunchDarkly (optional — not required)

LaunchDarkly is **not required**. The sandbox works fully without it using the local `blocked.txt` blocklist. These settings only apply if your organization uses LaunchDarkly for dynamic egress control.

| Variable | Default | Description |
|----------|---------|-------------|
| `LAUNCHDARKLY_CLIENT_ID` | — | LD client SDK key to enable dynamic egress control |
| `GOOSE_SANDBOX_LD_FAILOVER` | — | Failover mode if LD is unreachable: `allow`, `deny`, or `blocklist` |

## Domain Blocklist

The file `~/.config/goose/sandbox/blocked.txt` controls which domains are blocked by the proxy. It's created automatically on first run from a bundled template.

```
# One domain per line. Subdomains are blocked automatically.
# Lines starting with # are comments.
evil.com          # blocks evil.com and *.evil.com
pastebin.com
transfer.sh
webhook.site
```

**Live reload:** Changes to `blocked.txt` take effect immediately — the proxy watches the file with `fs.watch` and reloads it automatically. No restart needed.

## SSH and Git

SSH git operations (`git clone git@github.com:...`) work through the sandbox via a bundled `connect-proxy.pl` script that acts as an SSH `ProxyCommand`. This routes SSH connections through the egress proxy, which then applies the same allowlist rules.

By default, SSH is only allowed to well-known git hosting domains (GitHub, GitLab, Bitbucket, etc.). To customise:

```bash
# Add custom git hosts
export GOOSE_SANDBOX_GIT_HOSTS="github.com,gitlab.com,your-gitea.internal.com"

# Or allow SSH to all hosts
export GOOSE_SANDBOX_SSH_ALL_HOSTS=true
```

## Example Configurations

### Maximum security

```bash
export GOOSE_SANDBOX=true
# All protections enabled (defaults)
```

### Allow raw IP connections (e.g. for internal APIs)

```bash
export GOOSE_SANDBOX=true
export GOOSE_SANDBOX_ALLOW_IP=true
```

### Disable SSH entirely

```bash
export GOOSE_SANDBOX=true
export GOOSE_SANDBOX_ALLOW_SSH=false
```

### Relaxed mode (sandbox on, fewer restrictions)

```bash
export GOOSE_SANDBOX=true
export GOOSE_SANDBOX_PROTECT_FILES=false
export GOOSE_SANDBOX_BLOCK_RAW_SOCKETS=false
export GOOSE_SANDBOX_BLOCK_TUNNELING=false
export GOOSE_SANDBOX_ALLOW_IP=true
export GOOSE_SANDBOX_SSH_ALL_HOSTS=true
```

### With LaunchDarkly egress control

```bash
export GOOSE_SANDBOX=true
export LAUNCHDARKLY_CLIENT_ID=sdk-your-key-here
export GOOSE_SANDBOX_LD_FAILOVER=blocklist  # fall back to local blocklist if LD is down
```

## Troubleshooting

**"GOOSE_SANDBOX=true but sandbox-exec is not available (macOS only)"**
You're not on macOS, or `/usr/bin/sandbox-exec` is missing. The sandbox only works on macOS.

**Extensions or tools can't reach the network**
Check if the destination domain is in `~/.config/goose/sandbox/blocked.txt`, or if you need to enable `GOOSE_SANDBOX_ALLOW_IP=true` for IP-based endpoints.

**Git clone over SSH fails**
The target host may not be in the default git hosts allowlist. Add it with `GOOSE_SANDBOX_GIT_HOSTS=your-host.com` or set `GOOSE_SANDBOX_SSH_ALL_HOSTS=true`.

**Want to inspect what the proxy is blocking?**
Check the Electron/goosed logs — blocked connections are logged with the reason.
