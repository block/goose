/**
 * macOS Seatbelt sandbox for goosed.
 *
 * GOOSE_SANDBOX=true              — enable sandbox
 * LAUNCHDARKLY_CLIENT_ID=sdk-xxx  — optional LD egress control
 *
 * Seatbelt profile options (all default to enabled):
 *   GOOSE_SANDBOX_PROTECT_FILES=false      — disable SSH/shell config protection
 *   GOOSE_SANDBOX_BLOCK_RAW_SOCKETS=false  — disable raw socket blocking
 *   GOOSE_SANDBOX_BLOCK_TUNNELING=false    — disable tunneling tool blocking
 *
 * Proxy options:
 *   GOOSE_SANDBOX_ALLOW_IP=true            — allow raw IP address connections
 *   GOOSE_SANDBOX_BLOCK_LOOPBACK=true      — block loopback via proxy (default: off)
 *   GOOSE_SANDBOX_ALLOW_SSH=false           — block SSH ports (22/2222/7999) via proxy
 *   GOOSE_SANDBOX_GIT_HOSTS=host1,host2    — custom git host allowlist for SSH
 *   GOOSE_SANDBOX_SSH_ALL_HOSTS=true        — allow SSH to all hosts (default: git hosts only)
 *
 *   SSH git operations (git clone git@...) are routed through the proxy via
 *   a bundled connect-proxy.pl script used as SSH ProxyCommand. This avoids
 *   needing nc (which is blocked by the seatbelt profile).
 *   GOOSE_SANDBOX_LD_FAILOVER=allow|deny|blocklist — LD failover mode
 */

import path from 'node:path';
import fs from 'node:fs';
import os from 'node:os';
import { startProxy, ProxyInstance } from './proxy';

export { startProxy } from './proxy';
export type { ProxyInstance } from './proxy';

const homeDir = os.homedir();
const sandboxDir = path.join(homeDir, '.config', 'goose', 'sandbox');

// ---------------------------------------------------------------------------
// Sandbox profile builder
// ---------------------------------------------------------------------------

export interface SandboxProfileOptions {
  homeDir: string;
  protectSensitiveFiles: boolean;
  blockRawSockets: boolean;
  blockTunnelingTools: boolean;
}

export function buildSandboxProfile(opts: SandboxProfileOptions): string {
  const h = opts.homeDir;
  const lines: string[] = [
    '(version 1)',
    '(allow default)',
    '',
    `;; Protect sandbox config from the sandboxed process`,
    `(deny file-write* (subpath "${h}/.config/goose/sandbox"))`,
    `(deny file-write* (literal "${h}/.config/goose/config.yaml"))`,
  ];

  if (opts.protectSensitiveFiles) {
    lines.push(
      '',
      `(deny file-write* (subpath "${h}/.ssh"))`,
      `(deny file-write* (literal "${h}/.bashrc"))`,
      `(deny file-write* (literal "${h}/.zshrc"))`,
      `(deny file-write* (literal "${h}/.bash_profile"))`,
      `(deny file-write* (literal "${h}/.zprofile"))`
    );
  }

  lines.push(
    '',
    '(deny network*)',
    '(allow network-outbound (literal "/private/var/run/mDNSResponder"))',
    '(allow network-outbound (remote unix-socket))',
    '(allow network-outbound (remote ip "localhost:*"))',
    '(allow network-inbound (local ip "localhost:*"))'
  );

  if (opts.blockRawSockets) {
    lines.push(
      '',
      '(deny system-socket (require-all (socket-domain AF_INET) (socket-type SOCK_RAW)))',
      '(deny system-socket (require-all (socket-domain AF_INET6) (socket-type SOCK_RAW)))'
    );
  }

  if (opts.blockTunnelingTools) {
    lines.push(
      '',
      '(deny process-exec',
      '  (literal "/usr/bin/nc")',
      '  (literal "/usr/bin/ncat")',
      '  (literal "/usr/bin/netcat")',
      '  (literal "/usr/bin/socat")',
      '  (literal "/usr/bin/telnet")',
      ')'
    );
  }

  lines.push('', '(deny system-kext-load)', '');

  return lines.join('\n');
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

export function isSandboxEnabled(): boolean {
  return process.env.GOOSE_SANDBOX === 'true' || process.env.GOOSE_SANDBOX === '1';
}

export function isSandboxAvailable(): boolean {
  return process.platform === 'darwin' && fs.existsSync('/usr/bin/sandbox-exec');
}

function bundledPath(filename: string): string {
  // In packaged apps, process.resourcesPath points to the app resources directory.
  // In development, fall back to the source tree.
  const packagedPath = process.resourcesPath
    ? path.join(process.resourcesPath, 'sandbox', filename)
    : '';
  if (packagedPath && fs.existsSync(packagedPath)) {
    return packagedPath;
  }
  return path.join(process.cwd(), 'src', 'sandbox', filename);
}

function materialise(filename: string): string {
  const runtimePath = path.join(sandboxDir, filename);
  if (!fs.existsSync(runtimePath)) {
    fs.mkdirSync(sandboxDir, { recursive: true });
    const content = fs.readFileSync(bundledPath(filename), 'utf-8');
    fs.writeFileSync(runtimePath, content);
    console.log(`[sandbox] Materialised ${filename}`);
  }
  return runtimePath;
}

function writeSandboxProfile(content: string): string {
  const runtimePath = path.join(sandboxDir, 'sandbox.sb');
  fs.mkdirSync(sandboxDir, { recursive: true });
  fs.writeFileSync(runtimePath, content);
  return runtimePath;
}

function writeConnectProxy(): string {
  const runtimePath = path.join(sandboxDir, 'connect-proxy.pl');
  fs.mkdirSync(sandboxDir, { recursive: true });
  const content = fs.readFileSync(bundledPath('connect-proxy.pl'), 'utf-8');
  fs.writeFileSync(runtimePath, content, { mode: 0o755 });
  return runtimePath;
}

// ---------------------------------------------------------------------------
// Spawn
// ---------------------------------------------------------------------------

export function buildSandboxSpawn(
  goosedPath: string,
  goosedArgs: string[],
  proxyPort: number
): { command: string; args: string[]; env: Record<string, string> } {
  const profileOptions: SandboxProfileOptions = {
    homeDir,
    protectSensitiveFiles: process.env.GOOSE_SANDBOX_PROTECT_FILES !== 'false',
    blockRawSockets: process.env.GOOSE_SANDBOX_BLOCK_RAW_SOCKETS !== 'false',
    blockTunnelingTools: process.env.GOOSE_SANDBOX_BLOCK_TUNNELING !== 'false',
  };

  const profileContent = buildSandboxProfile(profileOptions);
  const sandboxProfile = writeSandboxProfile(profileContent);
  const proxyUrl = `http://127.0.0.1:${proxyPort}`;
  const connectProxy = writeConnectProxy();

  console.log(`[sandbox] Profile: ${sandboxProfile}`);
  console.log(`[sandbox] Proxy port: ${proxyPort}`);
  console.log(
    `[sandbox] Config: protectSensitiveFiles=${profileOptions.protectSensitiveFiles}, blockRawSockets=${profileOptions.blockRawSockets}, blockTunnelingTools=${profileOptions.blockTunnelingTools}`
  );

  return {
    command: '/usr/bin/sandbox-exec',
    args: ['-f', sandboxProfile, goosedPath, ...goosedArgs],
    env: {
      http_proxy: proxyUrl,
      https_proxy: proxyUrl,
      HTTP_PROXY: proxyUrl,
      HTTPS_PROXY: proxyUrl,
      no_proxy: 'localhost,127.0.0.1,::1',
      NO_PROXY: 'localhost,127.0.0.1,::1',
      GIT_SSH_COMMAND: `ssh -o ProxyCommand='/usr/bin/perl "${connectProxy}" %h %p'`,
      SANDBOX_PROXY_PORT: String(proxyPort),
    },
  };
}

// ---------------------------------------------------------------------------
// Proxy lifecycle
// ---------------------------------------------------------------------------

let activeProxy: ProxyInstance | null = null;

export async function ensureProxy(): Promise<ProxyInstance> {
  if (activeProxy) return activeProxy;

  const ldClientId = process.env.LAUNCHDARKLY_CLIENT_ID;
  const blockedPath = materialise('blocked.txt');

  activeProxy = await startProxy({
    blockedPath,
    launchDarkly: ldClientId
      ? {
          clientId: ldClientId,
          username: os.userInfo().username,
          failoverMode:
            (process.env.GOOSE_SANDBOX_LD_FAILOVER as 'allow' | 'deny' | 'blocklist') || undefined,
        }
      : undefined,
    allowIPAddresses: process.env.GOOSE_SANDBOX_ALLOW_IP === 'true',
    blockLoopback: process.env.GOOSE_SANDBOX_BLOCK_LOOPBACK === 'true',
    allowSSH: process.env.GOOSE_SANDBOX_ALLOW_SSH !== 'false',
    gitHosts:
      process.env.GOOSE_SANDBOX_GIT_HOSTS?.split(',')
        .map((h) => h.trim())
        .filter(Boolean) || undefined,
    allowSSHToAllHosts: process.env.GOOSE_SANDBOX_SSH_ALL_HOSTS === 'true',
  });

  console.log(`[sandbox] Proxy started on port ${activeProxy.port}`);
  return activeProxy;
}

export async function stopProxy(): Promise<void> {
  if (activeProxy) {
    await activeProxy.close();
    console.log('[sandbox] Proxy stopped');
    activeProxy = null;
  }
}
