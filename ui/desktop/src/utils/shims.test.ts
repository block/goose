import { afterEach, beforeAll, beforeEach, describe, expect, it, jest } from '@jest/globals';
import { replaceWithShim, SHIM_COMMANDS, SUPPORTED_EXTENSION_RUNNERS } from './shims';
import { join as joinPath } from 'path';
import * as fs from 'fs';

const assertExectuableFile = (path: string) => {
  if (!fs.existsSync(path)) {
    throw new Error(`Expected executable at ${path} does not exist`);
  } else if (!fs.statSync(path).isFile()) {
    throw new Error(`Expected executable at ${path} is not a file`);
  } else {
    fs.accessSync(path, fs.constants.X_OK); // Raises an error if the file is not executable
  }
};

describe('SHIM_COMMANDS', () => {
  const BINARY_DIR = joinPath(__dirname, '..', '..', 'src', 'bin');
  it('has a shim for `goosed`', () => {
    expect(SHIM_COMMANDS).toContain('goosed');
  });

  it('has a shim for `jbang`', () => {
    expect(SHIM_COMMANDS).toContain('jbang');
  });

  it('has a shim for `npx`', () => {
    expect(SHIM_COMMANDS).toContain('npx');
  });

  it('has a shim for `uvx`', () => {
    expect(SHIM_COMMANDS).toContain('uvx');
  });

  // The `goosed` shim is an artifact of the rust build process and is not checked into the
  // git repository like the other shimmed commands.
  SUPPORTED_EXTENSION_RUNNERS.forEach((cmd) => {
    it(`has an executable in the binary path for ${cmd}`, async () => {
      const expectedBinaryPath = joinPath(BINARY_DIR, cmd);
      assertExectuableFile(expectedBinaryPath);
    });
  });
});

describe('replaceWithShim', () => {
  const originalElectron = window.electron;
  const originalLog = console.log;
  const getBinaryPath = jest
    .fn()
    .mockImplementation((cmd: string) => Promise.resolve(`/path/to/${cmd}`));

  beforeEach(() => {
    window.electron = {
      ...window.electron,
      getBinaryPath,
    };
    console.log = jest.fn();
  });

  afterEach(() => {
    window.electron = originalElectron;
    console.log = originalLog;
  });

  it('returns the input command string if it is not a known shim', async () => {
    const cmd = 'unknown-command';
    const result = await replaceWithShim(cmd);
    expect(result).toBe(cmd);
  });

  SHIM_COMMANDS.forEach((cmd) => {
    it(`returns the exectuable path to the shim for ${cmd}`, async () => {
      const result = await replaceWithShim(cmd);
      expect(result).toBe(`/path/to/${cmd}`);
    });
  });
});
