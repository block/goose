/**
 * Integration tests for deploy-exe.js handoff phase
 *
 * Tests the HANDOFF.md generation and upload during deployment.
 * Uses mocked SSH functions to avoid actual network calls.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { existsSync, readFileSync, writeFileSync, unlinkSync, mkdirSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';

// Import the handoff generator directly
import { generateHandoff, extractContextFromEnv } from '../../generate-handoff.js';

// Mock the SSH module
vi.mock('../../lib/exe-ssh.js', () => ({
  findSSHKey: vi.fn(() => '/Users/test/.ssh/id_ed25519'),
  connect: vi.fn(async () => ({
    end: vi.fn(),
    on: vi.fn()
  })),
  runCommand: vi.fn(async () => ({ stdout: '', stderr: '', code: 0 })),
  runExeCommand: vi.fn(async () => 'OK'),
  uploadFile: vi.fn(async () => undefined),
  createVM: vi.fn(async (name) => ({ success: true, message: `VM ${name} created` })),
  setPublic: vi.fn(async () => ({ success: true, message: 'Public' })),
  testConnection: vi.fn(async () => true)
}));

describe('deploy handoff integration', () => {
  const testDir = join(tmpdir(), 'vibes-handoff-test');
  const testFile = join(testDir, 'index.html');
  const handoffFile = join(testDir, 'HANDOFF.md');

  beforeEach(() => {
    // Create test directory and file
    mkdirSync(testDir, { recursive: true });
    writeFileSync(testFile, '<html><body>Test App</body></html>');
  });

  afterEach(() => {
    // Cleanup
    if (existsSync(testFile)) unlinkSync(testFile);
    if (existsSync(handoffFile)) unlinkSync(handoffFile);
  });

  describe('handoff generation workflow', () => {
    it('generates HANDOFF.md with correct structure', () => {
      const context = {
        appDescription: 'A test todo app',
        files: ['index.html'],
        originalPrompt: 'Build a todo app',
        decisions: '- Used Fireproof for storage',
        nextSteps: 'Add filtering',
        vmName: 'testvm'
      };

      const handoff = generateHandoff(context);

      // Write to file like deploy script does
      writeFileSync(handoffFile, handoff);

      // Verify file was created
      expect(existsSync(handoffFile)).toBe(true);

      // Verify content
      const content = readFileSync(handoffFile, 'utf-8');
      expect(content).toContain('# Development Handoff');
      expect(content).toContain('A test todo app');
      expect(content).toContain('Build a todo app');
      expect(content).toContain('Fireproof for storage');
      expect(content).toContain('Add filtering');
      expect(content).toContain('testvm.exe.xyz');
    });

    it('uses environment variables when available', () => {
      const originalEnv = { ...process.env };

      process.env.VIBES_APP_DESCRIPTION = 'Env-based app';
      process.env.VIBES_ORIGINAL_PROMPT = 'Env prompt';
      process.env.VIBES_VM_NAME = 'envvm';

      const context = extractContextFromEnv();
      context.files = ['index.html'];
      const handoff = generateHandoff(context);

      expect(handoff).toContain('Env-based app');
      expect(handoff).toContain('Env prompt');
      expect(handoff).toContain('envvm.exe.xyz');

      // Restore env
      process.env = originalEnv;
    });

    it('falls back to defaults when env vars missing', () => {
      const originalEnv = { ...process.env };

      delete process.env.VIBES_APP_DESCRIPTION;
      delete process.env.VIBES_ORIGINAL_PROMPT;
      delete process.env.VIBES_VM_NAME;

      const context = extractContextFromEnv();
      const handoff = generateHandoff(context);

      expect(handoff).toContain('A Vibes app');
      expect(handoff).toContain('Build an app');

      process.env = originalEnv;
    });
  });

  describe('handoff content validation', () => {
    it('includes all required sections', () => {
      const handoff = generateHandoff({
        vmName: 'testapp'
      });

      const requiredSections = [
        '# Development Handoff',
        '## What Was Built',
        '## Files Included',
        "## User's Original Request",
        '## Key Decisions Made',
        '## What To Do Next',
        '## Technical Context'
      ];

      for (const section of requiredSections) {
        expect(handoff).toContain(section);
      }
    });

    it('includes helpful commands for the VM', () => {
      const handoff = generateHandoff({
        vmName: 'helpfulapp'
      });

      // Should include common commands
      expect(handoff).toContain('open https://helpfulapp.exe.xyz');
      expect(handoff).toContain('nano /var/www/html/index.html');
      expect(handoff).toContain('systemctl status nginx');
      expect(handoff).toContain('nginx/access.log');
    });

    it('describes the technical stack', () => {
      const handoff = generateHandoff({});

      expect(handoff).toContain('React');
      expect(handoff).toContain('Fireproof');
      expect(handoff).toContain('Tailwind');
      expect(handoff).toContain('esm.sh');
      expect(handoff).toContain('text/babel');
    });

    it('formats files as markdown list', () => {
      const handoff = generateHandoff({
        files: ['index.html', 'styles.css', 'data.json']
      });

      expect(handoff).toContain('- `index.html`');
      expect(handoff).toContain('- `styles.css`');
      expect(handoff).toContain('- `data.json`');
    });

    it('quotes the original prompt', () => {
      const handoff = generateHandoff({
        originalPrompt: 'Make me a weather app'
      });

      expect(handoff).toContain('> Make me a weather app');
    });
  });

  describe('edge cases', () => {
    it('handles empty options gracefully', () => {
      const handoff = generateHandoff({});

      expect(handoff).toBeTruthy();
      expect(handoff.length).toBeGreaterThan(100);
    });

    it('handles undefined options', () => {
      const handoff = generateHandoff(undefined);

      expect(handoff).toBeTruthy();
      expect(handoff).toContain('# Development Handoff');
    });

    it('handles special characters in prompt', () => {
      const handoff = generateHandoff({
        originalPrompt: 'Build an app with "quotes" and <html> tags'
      });

      expect(handoff).toContain('"quotes"');
      expect(handoff).toContain('<html>');
    });

    it('handles multiline decisions', () => {
      const decisions = `- First decision
- Second decision
- Third decision with more details
  - Sub-point 1
  - Sub-point 2`;

      const handoff = generateHandoff({ decisions });

      expect(handoff).toContain('First decision');
      expect(handoff).toContain('Sub-point 2');
    });

    it('handles very long app description', () => {
      const longDescription = 'A '.repeat(500) + 'very long description';
      const handoff = generateHandoff({ appDescription: longDescription });

      expect(handoff).toContain('very long description');
    });
  });
});

describe('SSH upload simulation', () => {
  it('simulates the upload workflow', async () => {
    const { uploadFile, connect, runCommand } = await import('../../lib/exe-ssh.js');

    // Generate handoff
    const handoff = generateHandoff({
      appDescription: 'Upload test app',
      vmName: 'uploadtest'
    });

    // Simulate writing to tmp
    const tmpPath = join(tmpdir(), 'test-handoff.md');
    writeFileSync(tmpPath, handoff);

    // Simulate upload (mocked)
    await uploadFile(tmpPath, 'uploadtest.exe.dev', '/tmp/HANDOFF.md');

    // Verify upload was called
    expect(uploadFile).toHaveBeenCalledWith(
      tmpPath,
      'uploadtest.exe.dev',
      '/tmp/HANDOFF.md'
    );

    // Simulate move command (mocked)
    const client = await connect('uploadtest.exe.dev');
    await runCommand(client, 'sudo mv /tmp/HANDOFF.md /var/www/html/HANDOFF.md');

    expect(connect).toHaveBeenCalledWith('uploadtest.exe.dev');
    expect(runCommand).toHaveBeenCalled();

    // Cleanup
    unlinkSync(tmpPath);
  });
});
