import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  activateExtension,
  addToAgentOnStartup,
  updateExtension,
  toggleExtension,
  deleteExtension,
} from './extension-manager';
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

  describe('activateExtension', () => {
    it('should successfully activate extension', async () => {
      mockAddToAgent.mockResolvedValue({} as Response);

      await activateExtension({
        addToConfig: mockAddToConfig,
        extensionConfig: mockExtensionConfig,
      });

      expect(mockAddToAgent).toHaveBeenCalledWith(mockExtensionConfig, { silent: false });
      expect(mockAddToConfig).toHaveBeenCalledWith('test-extension', mockExtensionConfig, true);
    });

    it('should add to config as disabled if agent fails', async () => {
      const agentError = new Error('Agent failed');
      mockAddToAgent.mockRejectedValue(agentError);

      await expect(
        activateExtension({
          addToConfig: mockAddToConfig,
          extensionConfig: mockExtensionConfig,
        })
      ).rejects.toThrow('Agent failed');

      expect(mockAddToAgent).toHaveBeenCalledWith(mockExtensionConfig, { silent: false });
      expect(mockAddToConfig).toHaveBeenCalledWith('test-extension', mockExtensionConfig, false);
    });

    it('should remove from agent if config fails', async () => {
      const configError = new Error('Config failed');
      mockAddToAgent.mockResolvedValue({} as Response);
      mockAddToConfig.mockRejectedValue(configError);

      await expect(
        activateExtension({
          addToConfig: mockAddToConfig,
          extensionConfig: mockExtensionConfig,
        })
      ).rejects.toThrow('Config failed');

      expect(mockAddToAgent).toHaveBeenCalledWith(mockExtensionConfig, { silent: false });
      expect(mockAddToConfig).toHaveBeenCalledWith('test-extension', mockExtensionConfig, true);
      expect(mockRemoveFromAgent).toHaveBeenCalledWith('test-extension');
    });
  });

  describe('addToAgentOnStartup', () => {
    it('should successfully add extension on startup', async () => {
      mockAddToAgent.mockResolvedValue({} as Response);

      await addToAgentOnStartup({
        addToConfig: mockAddToConfig,
        extensionConfig: mockExtensionConfig,
      });

      expect(mockAddToAgent).toHaveBeenCalledWith(mockExtensionConfig, { silent: true });
      expect(mockAddToConfig).not.toHaveBeenCalled();
    });

    it('should retry on 428 errors', async () => {
      const error428 = new Error('428 Precondition Required');
      mockAddToAgent
        .mockRejectedValueOnce(error428)
        .mockRejectedValueOnce(error428)
        .mockResolvedValue({} as Response);

      await addToAgentOnStartup({
        addToConfig: mockAddToConfig,
        extensionConfig: mockExtensionConfig,
      });

      expect(mockAddToAgent).toHaveBeenCalledTimes(3);
    });

    it('should disable extension after max retries', async () => {
      const error428 = new Error('428 Precondition Required');
      mockAddToAgent.mockRejectedValue(error428);
      mockToastService.configure = vi.fn();
      mockToastService.error = vi.fn();

      await addToAgentOnStartup({
        addToConfig: mockAddToConfig,
        extensionConfig: mockExtensionConfig,
      });

      expect(mockAddToAgent).toHaveBeenCalledTimes(4); // Initial + 3 retries
      expect(mockToastService.error).toHaveBeenCalledWith({
        title: 'test-extension',
        msg: 'Extension failed to start and will be disabled.',
        traceback: '428 Precondition Required',
      });
    });
  });

  describe('updateExtension', () => {
    it('should update extension without name change', async () => {
      mockAddToAgent.mockResolvedValue({} as Response);
      mockAddToConfig.mockResolvedValue(undefined);
      mockToastService.success = vi.fn();

      await updateExtension({
        enabled: true,
        addToConfig: mockAddToConfig,
        removeFromConfig: mockRemoveFromConfig,
        extensionConfig: mockExtensionConfig,
        originalName: 'test-extension',
      });

      expect(mockAddToAgent).toHaveBeenCalledWith(
        { ...mockExtensionConfig, name: 'test-extension' },
        { silent: true }
      );
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
      mockAddToAgent.mockResolvedValue({} as Response);
      mockRemoveFromAgent.mockResolvedValue({} as Response);
      mockRemoveFromConfig.mockResolvedValue(undefined);
      mockAddToConfig.mockResolvedValue(undefined);
      mockToastService.success = vi.fn();

      await updateExtension({
        enabled: true,
        addToConfig: mockAddToConfig,
        removeFromConfig: mockRemoveFromConfig,
        extensionConfig: { ...mockExtensionConfig, name: 'new-extension' },
        originalName: 'old-extension',
      });

      expect(mockRemoveFromAgent).toHaveBeenCalledWith('old-extension', { silent: true });
      expect(mockRemoveFromConfig).toHaveBeenCalledWith('old-extension');
      expect(mockAddToAgent).toHaveBeenCalledWith(
        { ...mockExtensionConfig, name: 'new-extension' },
        { silent: true }
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
      mockAddToAgent.mockResolvedValue({} as Response);
      mockAddToConfig.mockResolvedValue(undefined);

      await toggleExtension({
        toggle: 'toggleOn',
        extensionConfig: mockExtensionConfig,
        addToConfig: mockAddToConfig,
      });

      expect(mockAddToAgent).toHaveBeenCalledWith(mockExtensionConfig, {});
      expect(mockAddToConfig).toHaveBeenCalledWith('test-extension', mockExtensionConfig, true);
    });

    it('should toggle extension off successfully', async () => {
      mockRemoveFromAgent.mockResolvedValue({} as Response);
      mockAddToConfig.mockResolvedValue(undefined);

      await toggleExtension({
        toggle: 'toggleOff',
        extensionConfig: mockExtensionConfig,
        addToConfig: mockAddToConfig,
      });

      expect(mockRemoveFromAgent).toHaveBeenCalledWith('test-extension', {});
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
        })
      ).rejects.toThrow('Agent failed');

      expect(mockAddToAgent).toHaveBeenCalledWith(mockExtensionConfig, {});
      // addToConfig is called during the rollback (toggleOff)
      expect(mockAddToConfig).toHaveBeenCalledWith('test-extension', mockExtensionConfig, false);
    });

    it('should remove from agent if config update fails when toggling on', async () => {
      const configError = new Error('Config failed');
      mockAddToAgent.mockResolvedValue({} as Response);
      mockAddToConfig.mockRejectedValue(configError);

      await expect(
        toggleExtension({
          toggle: 'toggleOn',
          extensionConfig: mockExtensionConfig,
          addToConfig: mockAddToConfig,
        })
      ).rejects.toThrow('Config failed');

      expect(mockAddToAgent).toHaveBeenCalledWith(mockExtensionConfig, {});
      expect(mockAddToConfig).toHaveBeenCalledWith('test-extension', mockExtensionConfig, true);
      expect(mockRemoveFromAgent).toHaveBeenCalledWith('test-extension', {});
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
        })
      ).rejects.toThrow('Agent removal failed');

      expect(mockRemoveFromAgent).toHaveBeenCalledWith('test-extension', {});
      expect(mockAddToConfig).toHaveBeenCalledWith('test-extension', mockExtensionConfig, false);
    });
  });

  describe('deleteExtension', () => {
    it('should delete extension successfully', async () => {
      mockRemoveFromAgent.mockResolvedValue({} as Response);
      mockRemoveFromConfig.mockResolvedValue(undefined);

      await deleteExtension({
        name: 'test-extension',
        removeFromConfig: mockRemoveFromConfig,
      });

      expect(mockRemoveFromAgent).toHaveBeenCalledWith('test-extension', { isDelete: true });
      expect(mockRemoveFromConfig).toHaveBeenCalledWith('test-extension');
    });

    it('should remove from config even if agent removal fails', async () => {
      const agentError = new Error('Agent removal failed');
      mockRemoveFromAgent.mockRejectedValue(agentError);
      mockRemoveFromConfig.mockResolvedValue(undefined);

      await expect(
        deleteExtension({
          name: 'test-extension',
          removeFromConfig: mockRemoveFromConfig,
        })
      ).rejects.toThrow('Agent removal failed');

      expect(mockRemoveFromAgent).toHaveBeenCalledWith('test-extension', { isDelete: true });
      expect(mockRemoveFromConfig).toHaveBeenCalledWith('test-extension');
    });

    it('should throw config error if both agent and config fail', async () => {
      const agentError = new Error('Agent removal failed');
      const configError = new Error('Config removal failed');
      mockRemoveFromAgent.mockRejectedValue(agentError);
      mockRemoveFromConfig.mockRejectedValue(configError);

      await expect(
        deleteExtension({
          name: 'test-extension',
          removeFromConfig: mockRemoveFromConfig,
        })
      ).rejects.toThrow('Config removal failed');

      expect(mockRemoveFromAgent).toHaveBeenCalledWith('test-extension', { isDelete: true });
      expect(mockRemoveFromConfig).toHaveBeenCalledWith('test-extension');
    });
  });
});
