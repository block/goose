import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  nameToKey,
  getDefaultFormData,
  extensionToFormData,
  createExtensionConfig,
  splitCmdAndArgs,
  combineCmdAndArgs,
  extractExtensionConfig,
  replaceWithShims,
  removeShims,
  extractCommand,
  extractExtensionName,
  DEFAULT_EXTENSION_TIMEOUT,
} from './utils';
import type { FixedExtensionEntry } from '../../ConfigContext';

// Mock window.electron
const mockElectron = {
  getBinaryPath: vi.fn(),
};

Object.defineProperty(window, 'electron', {
  value: mockElectron,
  writable: true,
});

describe('Extension Utils', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('nameToKey', () => {
    it('should convert name to lowercase key format', () => {
      expect(nameToKey('My Extension')).toBe('myextension');
      expect(nameToKey('Test-Extension_Name')).toBe('test-extension_name');
      expect(nameToKey('UPPERCASE')).toBe('uppercase');
    });

    it('should remove spaces', () => {
      expect(nameToKey('Extension With Spaces')).toBe('extensionwithspaces');
      expect(nameToKey('  Multiple   Spaces  ')).toBe('multiplespaces');
    });
  });

  describe('getDefaultFormData', () => {
    it('should return default form data structure', () => {
      const defaultData = getDefaultFormData();

      expect(defaultData).toEqual({
        name: '',
        description: '',
        type: 'stdio',
        cmd: '',
        endpoint: '',
        enabled: true,
        timeout: 300,
        envVars: [],
        headers: [],
      });
    });
  });

  describe('extensionToFormData', () => {
    it('should convert stdio extension to form data', () => {
      const extension: FixedExtensionEntry = {
        type: 'stdio',
        name: 'test-extension',
        description: 'Test description',
        cmd: 'python',
        args: ['script.py', '--flag'],
        enabled: true,
        timeout: 600,
        env_keys: ['API_KEY', 'SECRET'],
      };

      const formData = extensionToFormData(extension);

      expect(formData).toEqual({
        name: 'test-extension',
        description: 'Test description',
        type: 'stdio',
        cmd: 'python script.py --flag',
        endpoint: undefined,
        enabled: true,
        timeout: 600,
        envVars: [
          { key: 'API_KEY', value: '••••••••', isEdited: false },
          { key: 'SECRET', value: '••••••••', isEdited: false },
        ],
        headers: [],
      });
    });

    it('should convert sse extension to form data', () => {
      const extension: FixedExtensionEntry = {
        type: 'sse',
        name: 'sse-extension',
        description: 'SSE description',
        uri: 'http://localhost:8080/events',
        enabled: false,
        env_keys: ['TOKEN'],
      };

      const formData = extensionToFormData(extension);

      expect(formData).toEqual({
        name: 'sse-extension',
        description: 'SSE description',
        type: 'sse',
        cmd: undefined,
        endpoint: 'http://localhost:8080/events',
        enabled: false,
        timeout: undefined,
        envVars: [{ key: 'TOKEN', value: '••••••••', isEdited: false }],
        headers: [],
      });
    });

    it('should convert streamable_http extension to form data', () => {
      const extension: FixedExtensionEntry = {
        type: 'streamable_http',
        name: 'http-extension',
        description: 'HTTP description',
        uri: 'http://api.example.com',
        enabled: true,
        headers: {
          Authorization: 'Bearer token',
          'Content-Type': 'application/json',
        },
        env_keys: ['API_KEY'],
      };

      const formData = extensionToFormData(extension);

      expect(formData).toEqual({
        name: 'http-extension',
        description: 'HTTP description',
        type: 'streamable_http',
        cmd: undefined,
        endpoint: 'http://api.example.com',
        enabled: true,
        timeout: undefined,
        envVars: [{ key: 'API_KEY', value: '••••••••', isEdited: false }],
        headers: [
          { key: 'Authorization', value: 'Bearer token', isEdited: false },
          { key: 'Content-Type', value: 'application/json', isEdited: false },
        ],
      });
    });

    it('should handle legacy envs field', () => {
      const extension: FixedExtensionEntry = {
        type: 'stdio',
        name: 'legacy-extension',
        cmd: 'node',
        args: ['app.js'],
        enabled: true,
        envs: {
          OLD_KEY: 'old_value',
          LEGACY_TOKEN: 'legacy_token',
        },
        env_keys: ['NEW_KEY'],
      };

      const formData = extensionToFormData(extension);

      expect(formData.envVars).toEqual([
        { key: 'OLD_KEY', value: 'old_value', isEdited: true },
        { key: 'LEGACY_TOKEN', value: 'legacy_token', isEdited: true },
        { key: 'NEW_KEY', value: '••••••••', isEdited: false },
      ]);
    });

    it('should handle builtin extension', () => {
      const extension: FixedExtensionEntry = {
        type: 'builtin',
        name: 'developer',
        enabled: true,
      };

      const formData = extensionToFormData(extension);

      expect(formData).toEqual({
        name: 'developer',
        description: '',
        type: 'builtin',
        cmd: undefined,
        endpoint: undefined,
        enabled: true,
        timeout: undefined,
        envVars: [],
        headers: [],
      });
    });
  });

  describe('createExtensionConfig', () => {
    it('should create stdio extension config', () => {
      const formData = {
        name: 'test-stdio',
        description: 'Test stdio extension',
        type: 'stdio' as const,
        cmd: 'python script.py --arg1 --arg2',
        endpoint: '',
        enabled: true,
        timeout: 300,
        envVars: [
          { key: 'API_KEY', value: 'secret123', isEdited: true },
          { key: '', value: '', isEdited: false }, // Should be filtered out
        ],
        headers: [],
      };

      const config = createExtensionConfig(formData);

      expect(config).toEqual({
        type: 'stdio',
        name: 'test-stdio',
        description: 'Test stdio extension',
        cmd: 'python',
        args: ['script.py', '--arg1', '--arg2'],
        timeout: 300,
        env_keys: ['API_KEY'],
      });
    });

    it('should create sse extension config', () => {
      const formData = {
        name: 'test-sse',
        description: 'Test SSE extension',
        type: 'sse' as const,
        cmd: '',
        endpoint: 'http://localhost:8080/events',
        enabled: true,
        timeout: 600,
        envVars: [{ key: 'TOKEN', value: 'abc123', isEdited: true }],
        headers: [],
      };

      const config = createExtensionConfig(formData);

      expect(config).toEqual({
        type: 'sse',
        name: 'test-sse',
        description: 'Test SSE extension',
        timeout: 600,
        uri: 'http://localhost:8080/events',
        env_keys: ['TOKEN'],
      });
    });

    it('should create streamable_http extension config', () => {
      const formData = {
        name: 'test-http',
        description: 'Test HTTP extension',
        type: 'streamable_http' as const,
        cmd: '',
        endpoint: 'http://api.example.com',
        enabled: true,
        timeout: 300,
        envVars: [{ key: 'API_KEY', value: 'key123', isEdited: true }],
        headers: [
          { key: 'Authorization', value: 'Bearer token', isEdited: true },
          { key: '', value: '', isEdited: false }, // Should be filtered out
        ],
      };

      const config = createExtensionConfig(formData);

      expect(config).toEqual({
        type: 'streamable_http',
        name: 'test-http',
        description: 'Test HTTP extension',
        timeout: 300,
        uri: 'http://api.example.com',
        env_keys: ['API_KEY'],
        headers: {
          Authorization: 'Bearer token',
        },
      });
    });

    it('should create builtin extension config', () => {
      const formData = {
        name: 'developer',
        description: '',
        type: 'builtin' as const,
        cmd: '',
        endpoint: '',
        enabled: true,
        timeout: 300,
        envVars: [],
        headers: [],
      };

      const config = createExtensionConfig(formData);

      expect(config).toEqual({
        type: 'builtin',
        name: 'developer',
        timeout: 300,
      });
    });
  });

  describe('splitCmdAndArgs', () => {
    it('should split command and arguments correctly', () => {
      expect(splitCmdAndArgs('python script.py --flag value')).toEqual({
        cmd: 'python',
        args: ['script.py', '--flag', 'value'],
      });

      expect(splitCmdAndArgs('node')).toEqual({
        cmd: 'node',
        args: [],
      });

      expect(splitCmdAndArgs('')).toEqual({
        cmd: '',
        args: [],
      });

      expect(splitCmdAndArgs('  multiple   spaces  between  args  ')).toEqual({
        cmd: 'multiple',
        args: ['spaces', 'between', 'args'],
      });
    });
  });

  describe('combineCmdAndArgs', () => {
    it('should combine command and arguments correctly', () => {
      expect(combineCmdAndArgs('python', ['script.py', '--flag', 'value'])).toBe(
        'python script.py --flag value'
      );

      expect(combineCmdAndArgs('node', [])).toBe('node');

      expect(combineCmdAndArgs('', ['arg1', 'arg2'])).toBe(' arg1 arg2');
    });
  });

  describe('extractExtensionConfig', () => {
    it('should extract extension config from fixed entry', () => {
      const fixedEntry: FixedExtensionEntry = {
        type: 'stdio',
        name: 'test-extension',
        cmd: 'python',
        args: ['script.py'],
        enabled: true,
        timeout: 300,
      };

      const config = extractExtensionConfig(fixedEntry);

      expect(config).toEqual({
        type: 'stdio',
        name: 'test-extension',
        cmd: 'python',
        args: ['script.py'],
        enabled: true,
        timeout: 300,
      });
    });
  });

  describe('replaceWithShims', () => {
    beforeEach(() => {
      mockElectron.getBinaryPath.mockImplementation((binary: string) => {
        const paths: Record<string, string> = {
          goosed: '/path/to/goosed',
          jbang: '/path/to/jbang',
          npx: '/path/to/npx',
          uvx: '/path/to/uvx',
        };
        return Promise.resolve(paths[binary] || binary);
      });
    });

    it('should replace known commands with shim paths', async () => {
      expect(await replaceWithShims('goosed')).toBe('/path/to/goosed');
      expect(await replaceWithShims('jbang')).toBe('/path/to/jbang');
      expect(await replaceWithShims('npx')).toBe('/path/to/npx');
      expect(await replaceWithShims('uvx')).toBe('/path/to/uvx');
    });

    it('should leave unknown commands unchanged', async () => {
      expect(await replaceWithShims('python')).toBe('python');
      expect(await replaceWithShims('node')).toBe('node');
    });
  });

  describe('removeShims', () => {
    it('should remove shim paths and return command name', () => {
      expect(removeShims('/path/to/goosed')).toBe('goosed');
      expect(removeShims('/usr/local/bin/jbang')).toBe('jbang');
      expect(removeShims('/Applications/Docker.app/Contents/Resources/bin/docker')).toBe('docker');
      expect(removeShims('/path/to/npx.cmd')).toBe('npx.cmd');
    });

    it('should handle paths with trailing slashes', () => {
      // The removeShims function only works if the path ends with the shim pattern
      // Trailing slashes prevent the pattern from matching
      expect(removeShims('/path/to/goosed/')).toBe('/path/to/goosed/');
      expect(removeShims('/path/to/uvx//')).toBe('/path/to/uvx//');
    });

    it('should leave non-shim commands unchanged', () => {
      expect(removeShims('python')).toBe('python');
      expect(removeShims('node')).toBe('node');
      expect(removeShims('/usr/bin/python3')).toBe('/usr/bin/python3');
    });
  });

  describe('extractCommand', () => {
    it('should extract command from extension link', () => {
      const link = 'goose://extension/add?name=Test&cmd=python&arg=script.py&arg=--flag';
      expect(extractCommand(link)).toBe('python script.py --flag');
    });

    it('should handle encoded arguments', () => {
      const link = 'goose://extension/add?cmd=echo&arg=hello%20world&arg=--test%3Dvalue';
      expect(extractCommand(link)).toBe('echo hello world --test=value');
    });

    it('should handle missing command', () => {
      const link = 'goose://extension/add?name=Test';
      expect(extractCommand(link)).toBe('Unknown Command');
    });

    it('should handle command without arguments', () => {
      const link = 'goose://extension/add?cmd=python';
      expect(extractCommand(link)).toBe('python');
    });
  });

  describe('extractExtensionName', () => {
    it('should extract extension name from link', () => {
      const link = 'goose://extension/add?name=Test%20Extension&cmd=python';
      expect(extractExtensionName(link)).toBe('Test Extension');
    });

    it('should handle missing name', () => {
      const link = 'goose://extension/add?cmd=python';
      expect(extractExtensionName(link)).toBe('Unknown Extension');
    });

    it('should decode URL encoded names', () => {
      const link = 'goose://extension/add?name=My%20Special%20Extension%21';
      expect(extractExtensionName(link)).toBe('My Special Extension!');
    });
  });

  describe('DEFAULT_EXTENSION_TIMEOUT', () => {
    it('should have correct default timeout value', () => {
      expect(DEFAULT_EXTENSION_TIMEOUT).toBe(300);
    });
  });
});
