/**
 * macOS Seatbelt sandbox for goosed.
 *
 * GOOSE_SANDBOX=true              — enable sandbox
 * LAUNCHDARKLY_CLIENT_ID=sdk-xxx  — optional LD egress control
 */

import path from 'node:path';
import fs from 'node:fs';
import os from 'node:os';
import { app } from 'electron';
import log from '../utils/logger';
import { startProxy, ProxyInstance } from './proxy';

export { startProxy } from './proxy';
export type { ProxyInstance } from './proxy';

const homeDir = os.homedir();
const sandboxDir = path.join(homeDir, '.config', 'goose', 'sandbox');

export function isSandboxEnabled(): boolean {
  return process.env.GOOSE_SANDBOX === 'true' || process.env.GOOSE_SANDBOX === '1';
}

export function isSandboxAvailable(): boolean {
  return process.platform === 'darwin' && fs.existsSync('/usr/bin/sandbox-exec');
}

function bundledPath(filename: string): string {
  if (app.isPackaged) {
    return path.join(process.resourcesPath, 'sandbox', filename);
  }
  return path.join(process.cwd(), 'src', 'sandbox', filename);
}

/**
 * Copy a bundled file to the runtime sandbox dir on first use.
 * The .sb profile has __HOMEDIR__ replaced with the actual home directory
 * since seatbelt can't expand ~ or use env vars.
 */
function materialise(filename: string): string {
  const runtimePath = path.join(sandboxDir, filename);
  if (!fs.existsSync(runtimePath)) {
    fs.mkdirSync(sandboxDir, { recursive: true });
    let content = fs.readFileSync(bundledPath(filename), 'utf-8');
    content = content.split('__HOMEDIR__').join(homeDir);
    fs.writeFileSync(runtimePath, content);
    log.info(`[sandbox] Materialised ${filename}`);
  }
  return runtimePath;
}

export function buildSandboxSpawn(
  goosedPath: string,
  goosedArgs: string[],
  proxyPort: number
): { command: string; args: string[]; env: Record<string, string> } {
  const sandboxProfile = materialise('sandbox.sb');
  const proxyUrl = `http://127.0.0.1:${proxyPort}`;

  log.info(`[sandbox] Profile: ${sandboxProfile}`);
  log.info(`[sandbox] Proxy port: ${proxyPort}`);

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
    },
  };
}

let activeProxy: ProxyInstance | null = null;

export async function ensureProxy(): Promise<ProxyInstance> {
  if (activeProxy) return activeProxy;

  const ldClientId = process.env.LAUNCHDARKLY_CLIENT_ID;
  const blockedPath = materialise('blocked.txt');

  activeProxy = await startProxy({
    blockedPath,
    launchDarkly: ldClientId
      ? { clientId: ldClientId, username: os.userInfo().username }
      : undefined,
  });

  log.info(`[sandbox] Proxy started on port ${activeProxy.port}`);
  return activeProxy;
}

export async function stopProxy(): Promise<void> {
  if (activeProxy) {
    await activeProxy.close();
    log.info('[sandbox] Proxy stopped');
    activeProxy = null;
  }
}
