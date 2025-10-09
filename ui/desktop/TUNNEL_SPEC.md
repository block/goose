# Tailscale Tunnel Integration Specification

## Overview
Add remote access functionality to the desktop app by integrating the `tailscale-tunnel.sh` script from goose-ios. This allows users to start a Tailscale tunnel for remote access to their goosed server.

## Architecture

### Script Location
- Source: `../goose-ios/tailscale-tunnel.sh`
- This script handles:
  - Starting goosed on a specified port
  - Setting up Tailscale tunnel
  - Writing connection info to a JSON output file

### Script Arguments
The script requires 4 arguments:
1. `<path_to_goosed>` - Full path to the goosed binary
2. `<port>` - Port number for goosed to listen on
3. `<secret>` - Secret key for authentication
4. `<output_json_file>` - Path to write connection info

### Output Format
The script writes a JSON file with this structure (from `out.json` example):
```json
{
  "url": "http://blkmqqxm3w7jq-2.tail66dcf.ts.net",
  "ipv4": "100.96.70.42",
  "ipv6": "fd7a:115c:a1e0::7733:462a",
  "hostname": "blkmqqxm3w7jq-2.tail66dcf.ts.net",
  "secret": "test",
  "port": 8090,
  "pids": {
    "goosed": 16942,
    "tailscale_serve": 16978
  }
}
```

## Implementation Requirements

### 1. Secret Management
- Generate a random secret on first use
- Store persistently in settings.json (same pattern as other settings)
- Reuse the same secret across app restarts
- Secret format: alphanumeric string, ~32 characters

### 2. Port Assignment
- Use dynamic port assignment (similar to existing `findAvailablePort()`)
- Or use a fixed port like 62997 (as in launch_tailscale.sh)

### 3. Goosed Binary Path
- The app already knows the goosed binary path via `getBinaryPath(app, 'goosed')` in goosed.ts
- Pass this path to the script

### 4. Process Management
- Execute `tailscale-tunnel.sh` with appropriate arguments
- Write output JSON to a temp file (e.g., `/tmp/goose-tunnel-${timestamp}.json`)
- Poll for the JSON file to exist (tunnel is ready when file is written)
- Parse the JSON to extract connection info
- Keep track of the child process for cleanup

### 5. UI Components

#### Settings Section
Add a new Card in AppSettingsSection.tsx (or new TunnelSection.tsx):
- Title: "Remote Access" or "Tailscale Tunnel"
- Description: "Enable remote access to goose via Tailscale"
- Button: "Start Tunnel for Remote Access"
- When clicked:
  - Disable button, show loading state
  - Start the tunnel script
  - Wait for out.json to be written
  - Parse connection info
  - Display QR code modal

#### QR Code Modal
When tunnel is ready, show a modal with:
- QR code for mobile app connection
- QR code format (from launch_tailscale.sh):
  ```
  goosechat://configure?data=<url_encoded_json>
  ```
  Where the JSON is: `{"url":"http://<ipv4>","secret":"<secret>"}`
- Display connection info:
  - Tunnel URL
  - IPv4 address
  - Secret key
- "Stop Tunnel" button

### 6. State Management
Track tunnel state:
- `idle` - Not running
- `starting` - Script executing, waiting for out.json
- `running` - Tunnel established, connection info available
- `error` - Failed to start

Store in component state or settings if persistence needed across restarts.

### 7. Cleanup
- When "Stop Tunnel" clicked, kill the tunnel process
- Kill tunnel process when app quits
- Handle errors gracefully

## File Structure

### New Files
- `src/components/settings/tunnel/TunnelSection.tsx` - Main tunnel UI component
- `src/utils/tunnel.ts` - Tunnel management logic (start/stop/status)

### Modified Files
- `src/components/settings/app/AppSettingsSection.tsx` - Import and include TunnelSection
- `src/utils/settings.ts` - Add `tunnelSecret` to Settings interface
- `src/preload.ts` - Add IPC methods: `startTunnel`, `stopTunnel`, `getTunnelStatus`
- `src/main.ts` - Add IPC handlers for tunnel operations

## QR Code Generation
Use the same pattern as launch_tailscale.sh (lines 272-279):
```typescript
const configJson = JSON.stringify({
  url: `http://${ipv4}`,
  secret: secret
});
const urlEncodedConfig = encodeURIComponent(configJson);
const appUrl = `goosechat://configure?data=${urlEncodedConfig}`;
// Generate QR code from appUrl
```

Use a React QR code library (check package.json for existing dependencies or add one like `qrcode.react`).

## Implementation Patterns
Follow existing patterns in the codebase:
- Settings management: Like `envToggles`, `showMenuBarIcon`, etc.
- IPC communication: Like `setMenuBarIcon` / `getMenuBarIconState`
- Child process spawning: Like goosed.ts
- UI components: Like existing settings sections with Card/CardHeader/CardContent
- Modal dialogs: Like notification instructions modal in AppSettingsSection

## Security Considerations
- Validate script path exists before execution
- Sanitize all inputs passed to shell script
- Use absolute paths
- Don't expose secret in logs
- Clean up temp files after use
