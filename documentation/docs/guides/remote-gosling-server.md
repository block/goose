---
sidebar_position: 90
title: Running a Remote gosling Server
sidebar_label: Remote Server
---

# Running a Remote gosling Server

gosling Desktop normally runs its own `goslingd` server process in the background on the same machine. You can also run `goslingd` separately — for example, on a remote VM or a different machine on your network — and point gosling Desktop at it.

This is useful when you want gosling to run somewhere with more compute, a stable IP, or shared access, while still driving it from a local Desktop UI.

This guide covers:

1. [Starting a `goslingd` server on a remote machine](#1-start-the-goslingd-server)
2. [Verifying it is reachable](#2-verify-the-server-is-up)
3. [Locating the certificate fingerprint](#3-find-the-certificate-fingerprint)
4. [Configuring gosling Desktop to connect to it](#4-configure-gosling-desktop)
5. [Running `goslingd` as a background service on macOS](#running-goslingd-as-a-background-service-macos)
6. [Troubleshooting](#troubleshooting)

:::warning TLS is required
gosling Desktop will refuse to connect to a remote `goslingd` server over plain HTTP. TLS is enabled by default (`GOSLING_TLS=true`), so make sure you have not disabled it.
:::

## Initial Setup

### 1. Start the `goslingd` server

On the remote machine, launch `goslingd` with the host, port, TLS, and a shared secret key:

```bash
GOSLING_HOST=0.0.0.0 \
GOSLING_PORT=3000 \
GOSLING_TLS=true \
GOSLING_SERVER__SECRET_KEY='YOUR_SECRET' \
/Applications/Gosling.app/Contents/Resources/bin/goslingd agent
```

On Linux or Windows the path to the `goslingd` binary will differ — use the one bundled with your gosling installation, or a standalone `goslingd` build.

| Variable | Purpose |
|----------|---------|
| `GOSLING_HOST` | Interface to bind to. Use `0.0.0.0` to accept connections from other machines. Binding to `localhost` or `127.0.0.1` will only accept local connections. |
| `GOSLING_PORT` | TCP port to listen on. |
| `GOSLING_TLS` | Must be `true`. gosling Desktop will not connect to a plain HTTP server. |
| `GOSLING_SERVER__SECRET_KEY` | Shared secret. The client must send this in the `X-Secret-Key` header. Treat it like a password. |

:::tip
Pick a long, random value for `GOSLING_SERVER__SECRET_KEY` and store it in a password manager — the same value goes into gosling Desktop later.
:::

### 2. Verify the server is up

First, confirm `goslingd` is actually listening on the port you expect:

```bash
lsof -nP -iTCP:3000 -sTCP:LISTEN
```

Then test the endpoints from the server itself. The `-k` flag tells `curl` to accept the self-signed TLS certificate that `goslingd` generates:

```bash
# Connectivity only
curl -i https://127.0.0.1:3000/status -k

# Authenticated endpoint (real test)
curl -i https://127.0.0.1:3000/config/read -k \
  -H 'Content-Type: application/json' \
  -H 'X-Secret-Key: YOUR_SECRET' \
  --data '{"key":"GOSLING_PROVIDER","is_secret":false}'
```

A `200` response from the second call confirms that TLS is up, the secret key is being accepted, and the server is ready to receive client requests.

If you intend to reach the server from another machine, also test from there using the server's hostname or VPN address — not `127.0.0.1`.

### 3. Find the certificate fingerprint

Because `goslingd` generates a self-signed TLS certificate, gosling Desktop pins it by SHA-256 fingerprint rather than relying on a public certificate authority.

When TLS is enabled, `goslingd` logs the fingerprint on startup. It looks like:

```text
GOSLINGD_CERT_FINGERPRINT=AA:BB:CC:DD:EE:FF:...
```

To capture it, either:

- Run `goslingd` interactively and read it from the terminal output, or
- Tail the log file you redirect to when running as a service (see [Running `goslingd` as a background service](#running-goslingd-as-a-background-service-macos)):

```bash
grep GOSLINGD_CERT_FINGERPRINT ~/Library/Logs/GoslingExternal/goslingd.out.log
```

Make a note of the fingerprint — you will paste it into gosling Desktop in the next step.

:::note
The fingerprint changes whenever `goslingd` regenerates its certificate (for example, if you delete the cert file). If gosling Desktop suddenly refuses to connect after a server restart, re-check the fingerprint.
:::

### 4. Configure gosling Desktop

On the client machine, open gosling Desktop and navigate to **Settings → gosling Server**:

| Setting | Value |
|---------|-------|
| **Use external server** | Enabled |
| **URL** | `https://your-server-host:3000` (use the hostname or IP that the client can reach — for example a VPN/tailnet address) |
| **Secret Key** | The same value you used for `GOSLING_SERVER__SECRET_KEY` |
| **Certificate Fingerprint** | The `GOSLINGD_CERT_FINGERPRINT` value from the server logs |

After saving, gosling Desktop will route all backend requests to the remote `goslingd`. If the connection fails, see [Troubleshooting](#troubleshooting).

## Running `goslingd` as a Background Service (macOS)

Running `goslingd` in a terminal session is fine for testing, but for everyday use you probably want it managed as a background service so it starts at login and restarts on failure. On macOS, this is done with `launchd`.

Create a LaunchAgent plist at `~/Library/LaunchAgents/com.gosling.goslingd.external.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
  <dict>
    <key>Label</key>
    <string>com.gosling.goslingd.external</string>

    <key>ProgramArguments</key>
    <array>
      <string>/Applications/Gosling.app/Contents/Resources/bin/goslingd</string>
      <string>agent</string>
    </array>

    <key>EnvironmentVariables</key>
    <dict>
      <key>GOSLING_HOST</key><string>0.0.0.0</string>
      <key>GOSLING_PORT</key><string>3000</string>
      <key>GOSLING_TLS</key><string>true</string>
      <key>GOSLING_SERVER__SECRET_KEY</key><string>YOUR_SECRET</string>
    </dict>

    <key>RunAtLoad</key><true/>
    <key>KeepAlive</key><true/>

    <key>StandardOutPath</key>
    <string>/Users/YOUR_USERNAME/Library/Logs/GoslingExternal/goslingd.out.log</string>
    <key>StandardErrorPath</key>
    <string>/Users/YOUR_USERNAME/Library/Logs/GoslingExternal/goslingd.err.log</string>
  </dict>
</plist>
```

Replace `YOUR_SECRET` and `YOUR_USERNAME` with appropriate values, and make sure the log directory exists:

```bash
mkdir -p ~/Library/Logs/GoslingExternal
```

Then load and start the service:

```bash
launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/com.gosling.goslingd.external.plist
launchctl kickstart -k gui/$(id -u)/com.gosling.goslingd.external
```

To stop or remove it later:

```bash
launchctl bootout gui/$(id -u)/com.gosling.goslingd.external
```

:::tip
Because the secret key is stored in plain text in the plist, the file should be readable only by your user. macOS LaunchAgents under `~/Library/LaunchAgents/` are already user-scoped, but you can tighten further with `chmod 600 ~/Library/LaunchAgents/com.gosling.goslingd.external.plist`.
:::

## Troubleshooting

### Server only accepts local connections

If `curl` works from the server but the client machine times out or gets "connection refused", check what interface `goslingd` is bound to. If `GOSLING_HOST` is `localhost` or `127.0.0.1`, only loopback connections are accepted.

Set `GOSLING_HOST=0.0.0.0` to accept connections on all interfaces, then restart `goslingd`. You can verify with:

```bash
lsof -nP -iTCP:3000 -sTCP:LISTEN
```

The output should show the address as `*:3000` or the specific external IP, not `127.0.0.1:3000`.

### TLS is not enabled

In the server's startup logs:

- If you see `listening on http://...`, TLS is **not** enabled. gosling Desktop will not connect. Set `GOSLING_TLS=true` and restart `goslingd`.
- If you see `listening on https://...`, TLS is enabled and you are good to go.

The startup logs also contain the `GOSLINGD_CERT_FINGERPRINT=...` line you need for the gosling Desktop configuration. Search the server's stdout (or log file, if running under `launchd`) for `GOSLINGD_CERT_FINGERPRINT` to find it.

### Client cannot authenticate (401 / Unauthorized)

A `401` from the server, or a gosling Desktop error indicating that the secret was rejected, almost always means that `GOSLING_SERVER__SECRET_KEY` on the server does not match the **Secret Key** in gosling Desktop's settings.

To check the secret end-to-end without involving gosling Desktop, run the authenticated `curl` from [step 2](#2-verify-the-server-is-up) using exactly the value you have configured on the client. If that returns `200`, the secret is correct and the problem is in the client configuration; if it returns `401`, the secret on the server is different from what you are sending.

If you rotate the secret on the server, you must also update it in gosling Desktop's settings — they are not synchronized automatically.

### Certificate fingerprint mismatch

If gosling Desktop refuses to connect with a certificate or fingerprint error, the most common causes are:

- The server regenerated its certificate (for example, after deleting the cert file). Look at the latest startup logs for the current `GOSLINGD_CERT_FINGERPRINT` and update gosling Desktop.
- You copied the fingerprint with extra whitespace or pasted the wrong value.

## Related

- [Environment Variables](/docs/guides/environment-variables) — full reference for all `GOSLING_*` variables
- [Configuration Files](/docs/guides/config-files) — persistent client-side configuration
