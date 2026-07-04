import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import { createGoslingServeStartupDiagnostics, createStartupDiagnostics } from './startupDiagnostics';

const tempDirs: string[] = [];

function makeTempDir(): string {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'startup-diagnostics-test-'));
  tempDirs.push(tempDir);
  return tempDir;
}

describe('startup diagnostics', () => {
  afterEach(() => {
    while (tempDirs.length > 0) {
      const tempDir = tempDirs.pop();
      if (tempDir) {
        fs.rmSync(tempDir, { recursive: true, force: true });
      }
    }
  });

  it('keeps goslingd startup diagnostics shape and file prefix', () => {
    const diagnosticsDir = makeTempDir();
    const trace = createStartupDiagnostics(diagnosticsDir, '/tmp/project');
    const expectedKeys = [
      'attemptId',
      'startedAt',
      'goslingdPath',
      'workingDir',
      'baseUrl',
      'pid',
      'certFingerprintSeen',
      'healthCheckSucceeded',
      'childExitCode',
      'childExitSignal',
      'stderrTail',
      'events',
    ];

    expect(trace).not.toBeNull();
    expect(path.basename(trace!.diagnosticsPath)).toMatch(/^goslingd-startup-.*\.json$/);
    expect(Object.keys(trace!.diagnostics)).toEqual(expectedKeys);
    expect(trace!.diagnostics).toMatchObject({
      goslingdPath: null,
      baseUrl: null,
      certFingerprintSeen: false,
    });
    const saved = JSON.parse(fs.readFileSync(trace!.diagnosticsPath, 'utf8'));
    expect(Object.keys(saved)).toEqual(expectedKeys);
  });

  it('writes serve startup diagnostics with serve-specific fields', () => {
    const diagnosticsDir = makeTempDir();
    const trace = createGoslingServeStartupDiagnostics(diagnosticsDir, '/tmp/project');

    expect(trace).not.toBeNull();
    trace!.diagnostics.binaryPath = '/bin/gosling';
    trace!.diagnostics.httpBaseUrl = 'http://127.0.0.1:3000';
    trace!.diagnostics.readinessUrl = 'http://127.0.0.1:3000/status';
    trace!.diagnostics.statusUrl = 'http://127.0.0.1:3000/status';
    trace!.diagnostics.healthUrl = 'http://127.0.0.1:3000/health';
    trace!.diagnostics.acpUrl = 'ws://127.0.0.1:3000/acp?token=REDACTED';
    trace!.record('healthcheck_start', {
      transport: 'plain-http',
      method: 'GET',
      path: '/status',
    });
    trace!.record('healthcheck_success', { attempt: 1 });

    expect(path.basename(trace!.diagnosticsPath)).toMatch(/^gosling-serve-startup-.*\.json$/);
    const saved = JSON.parse(fs.readFileSync(trace!.diagnosticsPath, 'utf8'));
    expect(saved).toMatchObject({
      binaryPath: '/bin/gosling',
      httpBaseUrl: 'http://127.0.0.1:3000',
      readinessUrl: 'http://127.0.0.1:3000/status',
      statusUrl: 'http://127.0.0.1:3000/status',
      healthUrl: 'http://127.0.0.1:3000/health',
      acpUrl: 'ws://127.0.0.1:3000/acp?token=REDACTED',
      healthCheckSucceeded: true,
    });
    expect(saved).not.toHaveProperty('goslingdPath');
    expect(saved).not.toHaveProperty('certFingerprintSeen');
    expect(saved.events.map((event: { name: string }) => event.name)).toEqual([
      'healthcheck_start',
      'healthcheck_success',
    ]);
  });
});
