/**
 * Mock for exe-ssh.js
 *
 * Provides mock implementations of SSH functions for testing
 * the deploy-exe.js script without actual network calls.
 */

import { vi } from 'vitest';

// Track all calls for assertions
export const mockCalls = {
  connect: [],
  runCommand: [],
  uploadFile: [],
  createVM: [],
  setPublic: [],
  testConnection: []
};

// Reset all mock call tracking
export function resetMocks() {
  mockCalls.connect = [];
  mockCalls.runCommand = [];
  mockCalls.uploadFile = [];
  mockCalls.createVM = [];
  mockCalls.setPublic = [];
  mockCalls.testConnection = [];
}

// Mock SSH client
export function createMockClient() {
  return {
    end: vi.fn(),
    on: vi.fn(),
    exec: vi.fn()
  };
}

// Mock implementations
export const findSSHKey = vi.fn(() => '/Users/test/.ssh/id_ed25519');

export const connect = vi.fn(async (host) => {
  mockCalls.connect.push({ host });
  return createMockClient();
});

export const runCommand = vi.fn(async (client, command) => {
  mockCalls.runCommand.push({ command });

  // Simulate different command responses
  if (command.includes('systemctl is-active nginx')) {
    return { stdout: 'active', stderr: '', code: 0 };
  }
  if (command.includes('systemctl enable')) {
    return { stdout: '', stderr: '', code: 0 };
  }
  if (command.includes('mv ') || command.includes('chown ')) {
    return { stdout: '', stderr: '', code: 0 };
  }

  return { stdout: '', stderr: '', code: 0 };
});

export const runExeCommand = vi.fn(async (command) => {
  return 'exe> ' + command + '\nOK';
});

export const uploadFile = vi.fn(async (localPath, host, remotePath) => {
  mockCalls.uploadFile.push({ localPath, host, remotePath });
  return undefined;
});

export const createVM = vi.fn(async (vmName) => {
  mockCalls.createVM.push({ vmName });
  return { success: true, message: `VM ${vmName} created successfully` };
});

export const setPublic = vi.fn(async (vmName) => {
  mockCalls.setPublic.push({ vmName });
  return { success: true, message: 'Public access enabled' };
});

export const testConnection = vi.fn(async () => {
  mockCalls.testConnection.push({});
  return true;
});

// Create a module mock object for vi.mock
export const exeSshMock = {
  findSSHKey,
  connect,
  runCommand,
  runExeCommand,
  uploadFile,
  createVM,
  setPublic,
  testConnection
};
