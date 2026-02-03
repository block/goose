/**
 * SSH automation library for exe.dev deployment
 *
 * Provides helpers for connecting to exe.dev VMs, running commands,
 * and uploading files via SCP.
 *
 * IMPORTANT: exe.dev VMs
 * - Home directory is /home/exedev (not ~ expansion)
 * - Use explicit paths or /tmp for staging files
 * - Default user is 'exedev'
 */

import { spawn } from 'child_process';

// exe.dev VM constants
export const EXE_HOME_DIR = '/home/exedev';
export const EXE_DEFAULT_USER = 'exedev';
import { readFileSync, createReadStream, statSync } from 'fs';
import { homedir } from 'os';
import { join } from 'path';

/**
 * Find the user's SSH private key
 * @returns {string|null} Path to private key or null if not found
 */
export function findSSHKey() {
  const sshDir = join(homedir(), '.ssh');
  const keyNames = ['id_ed25519', 'id_rsa', 'id_ecdsa'];

  for (const name of keyNames) {
    const keyPath = join(sshDir, name);
    try {
      statSync(keyPath);
      return keyPath;
    } catch {
      continue;
    }
  }
  return null;
}

/**
 * Create a virtual SSH client that uses system ssh commands.
 * This is a drop-in replacement for ssh2 Client that works with passphrase-protected keys
 * via ssh-agent.
 * @param {string} host - Hostname
 * @param {string} username - SSH username
 * @returns {object} Virtual client with host/username info and end() method
 */
function createVirtualClient(host, username) {
  return {
    host,
    username,
    end: () => {} // No-op since system ssh handles connections per-command
  };
}

/**
 * Create an SSH connection to a host
 * Returns a virtual client that stores connection info for use with runCommand.
 * Uses system ssh via ssh-agent for passphrase-protected keys.
 * @param {string} host - Hostname to connect to
 * @param {object} [options] - Connection options
 * @param {string} [options.username] - SSH username (default: exedev for exe.dev VMs)
 * @param {string} [options.privateKeyPath] - Path to private key (not used, ssh-agent handles this)
 * @returns {Promise<object>} Virtual SSH client
 */
export async function connect(host, options = {}) {
  const privateKeyPath = findSSHKey();
  if (!privateKeyPath) {
    throw new Error('No SSH key found. Please ensure you have an SSH key in ~/.ssh/');
  }

  // Determine username based on host
  const username = options.username || (host.endsWith('.exe.xyz') ? 'exedev' : process.env.USER || 'user');

  // Test connection with a quick command
  const testClient = createVirtualClient(host, username);
  try {
    await runCommand(testClient, 'echo connected');
  } catch (err) {
    throw new Error(`Failed to connect to ${host}: ${err.message}`);
  }

  return testClient;
}

/**
 * Run a command on an SSH connection (virtual client)
 * Uses system ssh to leverage ssh-agent for passphrase-protected keys.
 * @param {object} client - Virtual SSH client from connect()
 * @param {string} command - Command to execute
 * @returns {Promise<{stdout: string, stderr: string, code: number}>}
 */
export function runCommand(client, command) {
  return new Promise((resolve, reject) => {
    const ssh = spawn('ssh', [
      '-o', 'ConnectTimeout=10',
      '-o', 'StrictHostKeyChecking=accept-new',
      `${client.username}@${client.host}`,
      command
    ], { stdio: ['ignore', 'pipe', 'pipe'] });

    let stdout = '';
    let stderr = '';

    ssh.stdout.on('data', (data) => { stdout += data.toString(); });
    ssh.stderr.on('data', (data) => { stderr += data.toString(); });

    ssh.on('close', (code) => {
      resolve({ stdout, stderr, code });
    });

    ssh.on('error', reject);
  });
}

/**
 * Create an error with a code for programmatic handling
 * @param {string} message - Error message
 * @param {string} code - Error code
 * @returns {Error}
 */
function createError(message, code) {
  const err = new Error(message);
  err.code = code;
  return err;
}

/**
 * Run an interactive session with the exe.dev CLI
 * This handles the interactive prompts from `ssh exe.dev`
 * @param {string} command - Command to run (e.g., 'new myvm')
 * @param {object} [options] - Options
 * @param {number} [options.timeout] - Timeout in ms (default: 30000)
 * @returns {Promise<string>} Command output
 * @throws {Error} With code property: NO_SSH_KEY, CONNECTION_REFUSED, TIMEOUT, AUTH_FAILED, SSH_ERROR
 */
export function runExeCommand(command, options = {}) {
  return new Promise((resolve, reject) => {
    const timeout = options.timeout || 30000;

    const privateKeyPath = findSSHKey();
    if (!privateKeyPath) {
      reject(createError('No SSH key found. Create one with: ssh-keygen -t ed25519', 'NO_SSH_KEY'));
      return;
    }

    let output = '';
    let stderr = '';
    let resolved = false;

    // Use system ssh to leverage ssh-agent for passphrase-protected keys
    const ssh = spawn('ssh', [
      '-o', 'ConnectTimeout=10',
      '-o', 'StrictHostKeyChecking=accept-new',
      'exe.dev',
      command
    ], { stdio: ['ignore', 'pipe', 'pipe'] });

    const timeoutId = setTimeout(() => {
      if (!resolved) {
        resolved = true;
        ssh.kill();
        reject(createError(`Command timed out after ${timeout / 1000}s. Check network or try: ssh exe.dev`, 'TIMEOUT'));
      }
    }, timeout);

    ssh.stdout.on('data', (data) => { output += data.toString(); });
    ssh.stderr.on('data', (data) => { stderr += data.toString(); });

    ssh.on('close', (code) => {
      if (!resolved) {
        resolved = true;
        clearTimeout(timeoutId);
        // exe.dev REPL may return non-zero for some commands but still succeed
        // Return output and let caller interpret
        resolve(output || stderr);
      }
    });

    ssh.on('error', (err) => {
      if (!resolved) {
        resolved = true;
        clearTimeout(timeoutId);

        let message = err.message;
        let errorCode = 'SSH_ERROR';

        if (err.message.includes('ENOENT')) {
          message = 'ssh command not found. Install OpenSSH.';
          errorCode = 'SSH_NOT_FOUND';
        }

        reject(createError(message, errorCode));
      }
    });
  });
}

/**
 * Upload a file via SCP
 * Uses system scp to leverage ssh-agent for passphrase-protected keys.
 * @param {string} localPath - Local file path
 * @param {string} host - Remote hostname
 * @param {string} remotePath - Remote file path
 * @param {object} [options] - Options
 * @returns {Promise<void>}
 */
export function uploadFile(localPath, host, remotePath, options = {}) {
  return new Promise((resolve, reject) => {
    const privateKeyPath = options.privateKeyPath || findSSHKey();
    if (!privateKeyPath) {
      reject(new Error('No SSH key found'));
      return;
    }

    const username = options.username || 'exedev';
    const destination = `${username}@${host}:${remotePath}`;

    const scp = spawn('scp', [
      '-o', 'StrictHostKeyChecking=accept-new',
      localPath,
      destination
    ], { stdio: ['ignore', 'pipe', 'pipe'] });

    let stderr = '';
    scp.stderr.on('data', (data) => { stderr += data.toString(); });

    scp.on('close', (code) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`SCP failed: ${stderr || 'Unknown error'}`));
      }
    });

    scp.on('error', reject);
  });
}

/**
 * Run a command on a remote VM via system ssh
 * Uses ssh-agent for passphrase-protected keys.
 * @param {string} host - Remote hostname
 * @param {string} command - Command to execute
 * @param {object} [options] - Options
 * @param {string} [options.username] - SSH username (default: 'exedev')
 * @returns {Promise<string>} Command output
 */
export async function runVMCommand(host, command, options = {}) {
  const username = options.username || 'exedev';
  const client = createVirtualClient(host, username);
  const result = await runCommand(client, command);

  if (result.code === 0) {
    return result.stdout;
  } else {
    throw new Error(`SSH command failed (code ${result.code}): ${result.stderr || result.stdout}`);
  }
}

/**
 * Upload a file via SCP to a privileged location (uses temp + sudo)
 * @param {string} localPath - Local file path
 * @param {string} host - Remote hostname
 * @param {string} remotePath - Remote file path (e.g., /var/www/html/index.html)
 * @param {object} [options] - Options
 * @returns {Promise<void>}
 */
export async function uploadFileWithSudo(localPath, host, remotePath, options = {}) {
  const { basename } = await import('path');
  const filename = basename(localPath);
  const tempPath = `/home/exedev/${filename}`;

  // Upload to home directory first (guaranteed writable)
  await uploadFile(localPath, host, tempPath, options);

  // Move to final location with sudo
  await runVMCommand(host, `sudo cp ${tempPath} ${remotePath} && sudo chown www-data:www-data ${remotePath} && rm ${tempPath}`, options);
}

/**
 * Test SSH connectivity to exe.dev
 * Uses system ssh command to leverage ssh-agent for passphrase-protected keys.
 * @returns {Promise<boolean>} True if connection successful
 */
export async function testConnection() {
  return new Promise((resolve) => {
    const privateKeyPath = findSSHKey();
    if (!privateKeyPath) {
      resolve(false);
      return;
    }

    const ssh = spawn('ssh', [
      '-o', 'ConnectTimeout=10',
      '-o', 'BatchMode=yes',
      '-o', 'StrictHostKeyChecking=accept-new',
      'exe.dev',
      'whoami'
    ], { stdio: ['ignore', 'pipe', 'pipe'] });

    let stdout = '';
    ssh.stdout.on('data', (data) => { stdout += data.toString(); });

    const timeout = setTimeout(() => {
      ssh.kill();
      resolve(false);
    }, 15000);

    ssh.on('close', (code) => {
      clearTimeout(timeout);
      // exe.dev REPL returns user info on 'whoami', or error if not set up
      // A successful connection will have some output (even if command fails)
      resolve(code === 0 || stdout.length > 0);
    });

    ssh.on('error', () => {
      clearTimeout(timeout);
      resolve(false);
    });
  });
}

/**
 * Create a new VM on exe.dev
 * @param {string} vmName - Name for the new VM
 * @returns {Promise<{success: boolean, message: string, code?: string}>}
 */
export async function createVM(vmName) {
  try {
    const output = await runExeCommand(`new --name=${vmName}`, { timeout: 60000 });
    const lowerOutput = output.toLowerCase();

    // Success patterns
    if (lowerOutput.includes('creating') || lowerOutput.includes('created') || lowerOutput.includes('ready') || lowerOutput.includes('.exe.xyz')) {
      return { success: true, message: `VM ${vmName} created` };
    }

    if (lowerOutput.includes('already exists') || lowerOutput.includes('not available')) {
      return { success: true, message: `VM ${vmName} already exists`, code: 'EXISTS' };
    }

    // Error patterns - detect specific issues
    if (lowerOutput.includes('quota') || lowerOutput.includes('limit')) {
      return {
        success: false,
        message: `VM quota exceeded. Delete unused VMs with: ssh exe.dev rm <vmname>`,
        code: 'QUOTA_EXCEEDED'
      };
    }

    if (lowerOutput.includes('invalid') || lowerOutput.includes('not allowed')) {
      return {
        success: false,
        message: `Invalid VM name "${vmName}". Use lowercase letters, numbers, hyphens only.`,
        code: 'INVALID_NAME'
      };
    }

    // Unknown response
    return {
      success: false,
      message: `Unexpected response. Run manually: ssh exe.dev new ${vmName}`,
      code: 'UNKNOWN'
    };
  } catch (err) {
    return { success: false, message: err.message, code: err.code || 'SSH_ERROR' };
  }
}

/**
 * Set a VM to public access
 * @param {string} vmName - VM name
 * @returns {Promise<{success: boolean, message: string}>}
 */
export async function setPublic(vmName) {
  try {
    const output = await runExeCommand(`share set-public ${vmName}`, { timeout: 30000 });

    // Check for success indicators in output
    const lowerOutput = output.toLowerCase();
    if (lowerOutput.includes('public') || lowerOutput.includes('success') || lowerOutput.includes('ok')) {
      return { success: true, message: output };
    }

    // Check for error indicators
    if (lowerOutput.includes('error') || lowerOutput.includes('fail') || lowerOutput.includes('not found')) {
      return { success: false, message: output.trim() };
    }

    // If no clear indicator, assume success (command completed without error)
    return { success: true, message: output };
  } catch (err) {
    return { success: false, message: err.message };
  }
}

/**
 * List VMs on exe.dev
 * @returns {Promise<string[]>} List of VM names
 */
export async function listVMs() {
  try {
    const output = await runExeCommand('ls', { timeout: 15000 });
    // Parse the ls output to extract VM names
    const lines = output.split('\n').filter(line => line.trim() && !line.includes('exe>'));
    return lines;
  } catch {
    return [];
  }
}
