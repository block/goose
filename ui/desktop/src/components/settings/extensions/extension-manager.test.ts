import { describe, it, expect, vi, beforeEach } from 'vitest';
import { addToAgentOnStartup, updateExtension, toggleExtension } from './extension-manager';
import * as agentApi from './agent-api';
import * as toasts from '../../../toasts';

// Mock dependencies
vi.mock('./agent-api');
vi.mock('../../../toasts');

const mockAddToAgent = vi.mocked(agentApi.addToAgent);
const mockRemoveFromAgent = vi.mocked(agentApi.removeFromAgent);
const mockSanitizeName = vi.mocked(agentApi.sanitizeName);
const mockToastService = vi.mocked(toasts.toastService);

describe('Extension Manager', () => {
  const mockAddToConfig = vi.fn();
  const mockRemoveFromConfig = vi.fn();

  const mockExtensionConfig = {
    type: 'stdio' as const,
    name: 'test-extension',
    description: 'test-extension',
    cmd: 'python',
    args: ['script.py'],
    timeout: 300,
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockSanitizeName.mockImplementation((name: string) => name.toLowerCase());
    mockAddToConfig.mockResolvedValue(undefined);
    mockRemoveFromConfig.mockResolvedValue(undefined);
  });

  describe('addToAgentOnStartup', () => {
    it('should successfully add extension on startup', async () => {
      mockAddToAgent.mockResolvedValue(undefined);

      await addToAgentOnStartup({
        sessionId: 'test-session',
        extensionConfig: mockExtensionConfig,
      });

      expect(mockAddToAgent).toHaveBeenCalledWith(mockExtensionConfig, 'test-session', true);
    });

    it('should successfully add extension on startup with custom toast options', async () => {
      mockAddToAgent.mockResolvedValue(undefined);

      await addToAgentOnStartup({
        sessionId: 'test-session',
        extensionConfig: mockExtensionConfig,
      });

      expect(mockAddToAgent).toHaveBeenCalledWith(mockExtensionConfig, 'test-session', true);
    });

    it('should retry on 428 errors', async () => {
      const error428 = new Error('428 Precondition Required');
      mockAddToAgent
        .mockRejectedValueOnce(error428)
        .mockRejectedValueOnce(error428)
        .mockResolvedValue(undefined);

      await addToAgentOnStartup({
        sessionId: 'test-session',
        extensionConfig: mockExtensionConfig,
      });

      expect(mockAddToAgent).toHaveBeenCalledTimes(3);
    });

    it('should throw error after max retries', async () => {
      const error428 = new Error('428 Precondition Required');
      mockAddToAgent.mockRejectedValue(error428);

      await expect(
        addToAgentOnStartup({
          sessionId: 'test-session',
          extensionConfig: mockExtensionConfig,
        })
      ).rejects.toThrow('428 Precondition Required');

      expect(mockAddToAgent).toHaveBeenCalledTimes(4); // Initial + 3 retries
    });
  });

  describe('updateExtension', () => {
    it('should update extension without name change', async () => {
      mockAddToAgent.mockResolvedValue(undefined);
      mockAddToConfig.mockResolvedValue(undefined);
      mockToastService.success = vi.fn();

      await updateExtension({
        enabled: true,
        addToConfig: mockAddToConfig,
        sessionId: 'test-session',
        removeFromConfig: mockRemoveFromConfig,
        extensionConfig: mockExtensionConfig,
        originalName: 'test-extension',
      });

      expect(mockAddToConfig).toHaveBeenCalledWith(
        'test-extension',
        { ...mockExtensionConfig, name: 'test-extension' },
        true
      );
      expect(mockToastService.success).toHaveBeenCalledWith({
        title: 'Update extension',
        msg: 'Successfully updated test-extension extension',
      });
    });

    it('should handle name change by removing old and adding new', async () => {
      mockAddToAgent.mockResolvedValue(undefined);
      mockRemoveFromAgent.mockResolvedValue(undefined);
      mockRemoveFromConfig.mockResolvedValue(undefined);
      mockAddToConfig.mockResolvedValue(undefined);
      mockToastService.success = vi.fn();

      await updateExtension({
        enabled: true,
        addToConfig: mockAddToConfig,
        sessionId: 'test-session',
        removeFromConfig: mockRemoveFromConfig,
        extensionConfig: { ...mockExtensionConfig, name: 'new-extension' },
        originalName: 'old-extension',
      });

      expect(mockRemoveFromConfig).toHaveBeenCalledWith('old-extension');
      expect(mockAddToAgent).toHaveBeenCalledWith(
        { ...mockExtensionConfig, name: 'new-extension' },
        'test-session',
        false
      );
      expect(mockAddToConfig).toHaveBeenCalledWith(
        'new-extension',
        { ...mockExtensionConfig, name: 'new-extension' },
        true
      );
    });

    it('should update disabled extension without calling agent', async () => {
      mockAddToConfig.mockResolvedValue(undefined);
      mockToastService.success = vi.fn();

      await updateExtension({
        enabled: false,
        addToConfig: mockAddToConfig,
        sessionId: 'test-session',
        removeFromConfig: mockRemoveFromConfig,
        extensionConfig: mockExtensionConfig,
        originalName: 'test-extension',
      });

      expect(mockAddToAgent).not.toHaveBeenCalled();
      expect(mockAddToConfig).toHaveBeenCalledWith(
        'test-extension',
        { ...mockExtensionConfig, name: 'test-extension' },
        false
      );
      expect(mockToastService.success).toHaveBeenCalledWith({
        title: 'Update extension',
        msg: 'Successfully updated test-extension extension',
      });
    });
  });

  describe('toggleExtension', () => {
    it('should toggle extension on successfully', async () => {
      mockAddToAgent.mockResolvedValue(undefined);
      mockAddToConfig.mockResolvedValue(undefined);

      await toggleExtension({
        toggle: 'toggleOn',
        extensionConfig: mockExtensionConfig,
        addToConfig: mockAddToConfig,
        sessionId: 'test-session',
      });

      expect(mockAddToAgent).toHaveBeenCalledWith(mockExtensionConfig, 'test-session', true);
      expect(mockAddToConfig).toHaveBeenCalledWith('test-extension', mockExtensionConfig, true);
    });

    it('should toggle extension off successfully', async () => {
      mockRemoveFromAgent.mockResolvedValue(undefined);
      mockAddToConfig.mockResolvedValue(undefined);

      await toggleExtension({
        toggle: 'toggleOff',
        extensionConfig: mockExtensionConfig,
        addToConfig: mockAddToConfig,
        sessionId: 'test-session',
      });

      expect(mockRemoveFromAgent).toHaveBeenCalledWith('test-extension', 'test-session', true);
      expect(mockAddToConfig).toHaveBeenCalledWith('test-extension', mockExtensionConfig, false);
    });

    it('should rollback on agent failure when toggling on', async () => {
      const agentError = new Error('Agent failed');
      mockAddToAgent.mockRejectedValue(agentError);
      mockAddToConfig.mockResolvedValue(undefined);

      await expect(
        toggleExtension({
          toggle: 'toggleOn',
          extensionConfig: mockExtensionConfig,
          addToConfig: mockAddToConfig,
          sessionId: 'test-session',
        })
      ).rejects.toThrow('Agent failed');

      expect(mockAddToAgent).toHaveBeenCalledWith(mockExtensionConfig, 'test-session', true);
      // addToConfig is called during the rollback (toggleOff)
      expect(mockAddToConfig).toHaveBeenCalledWith('test-extension', mockExtensionConfig, false);
    });

    it('should remove from agent if config update fails when toggling on', async () => {
      const configError = new Error('Config failed');
      mockAddToAgent.mockResolvedValue(undefined);
      mockAddToConfig.mockRejectedValue(configError);

      await expect(
        toggleExtension({
          toggle: 'toggleOn',
          extensionConfig: mockExtensionConfig,
          addToConfig: mockAddToConfig,
          sessionId: 'test-session',
        })
      ).rejects.toThrow('Config failed');

      expect(mockAddToAgent).toHaveBeenCalledWith(mockExtensionConfig, 'test-session', true);
      expect(mockAddToConfig).toHaveBeenCalledWith('test-extension', mockExtensionConfig, true);
      expect(mockRemoveFromAgent).toHaveBeenCalledWith('test-extension', 'test-session', true);
    });

    it('should update config even if agent removal fails when toggling off', async () => {
      const agentError = new Error('Agent removal failed');
      mockRemoveFromAgent.mockRejectedValue(agentError);
      mockAddToConfig.mockResolvedValue(undefined);

      await expect(
        toggleExtension({
          toggle: 'toggleOff',
          extensionConfig: mockExtensionConfig,
          addToConfig: mockAddToConfig,
          sessionId: 'test-session',
        })
      ).rejects.toThrow('Agent removal failed');

      expect(mockAddToConfig).toHaveBeenCalledWith('test-extension', mockExtensionConfig, false);
    });
  });
});
