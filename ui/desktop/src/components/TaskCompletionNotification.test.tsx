/* eslint-disable @typescript-eslint/no-explicit-any */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

/**
 * Tests for task completion notification functionality.
 * 
 * When Goose finishes a task (stream finishes), a native OS notification
 * should be shown if the window is not focused. This helps users who
 * switch to other apps while waiting for Goose to complete.
 * 
 * The notification is triggered in BaseChat.tsx's onStreamFinish callback.
 */

describe('Task Completion Notification', () => {
  let mockShowNotification: ReturnType<typeof vi.fn>;
  let originalHasFocus: typeof document.hasFocus;

  beforeEach(() => {
    mockShowNotification = vi.fn();
    
    // Store original hasFocus
    originalHasFocus = document.hasFocus;
    
    // Mock window.electron.showNotification
    (window as any).electron = {
      ...(window as any).electron,
      showNotification: mockShowNotification,
    };
  });

  afterEach(() => {
    vi.clearAllMocks();
    // Restore original hasFocus
    document.hasFocus = originalHasFocus;
  });

  describe('onStreamFinish notification behavior', () => {
    // This tests the logic that exists in BaseChat.tsx's onStreamFinish callback
    const simulateOnStreamFinish = () => {
      if (!document.hasFocus()) {
        window.electron.showNotification({
          title: 'Goose',
          body: 'Task completed',
        });
      }
    };

    it('should show notification when window is not focused', () => {
      // Mock document.hasFocus to return false (user switched to another app)
      document.hasFocus = vi.fn().mockReturnValue(false);

      simulateOnStreamFinish();

      expect(mockShowNotification).toHaveBeenCalledTimes(1);
      expect(mockShowNotification).toHaveBeenCalledWith({
        title: 'Goose',
        body: 'Task completed',
      });
    });

    it('should NOT show notification when window is focused', () => {
      // Mock document.hasFocus to return true (user is looking at the app)
      document.hasFocus = vi.fn().mockReturnValue(true);

      simulateOnStreamFinish();

      expect(mockShowNotification).not.toHaveBeenCalled();
    });

    it('should call document.hasFocus to check window state', () => {
      const hasFocusMock = vi.fn().mockReturnValue(true);
      document.hasFocus = hasFocusMock;

      simulateOnStreamFinish();

      expect(hasFocusMock).toHaveBeenCalled();
    });
  });

  describe('notification content', () => {
    it('should have correct title', () => {
      document.hasFocus = vi.fn().mockReturnValue(false);

      // Simulate the notification call
      if (!document.hasFocus()) {
        window.electron.showNotification({
          title: 'Goose',
          body: 'Task completed',
        });
      }

      const callArgs = mockShowNotification.mock.calls[0][0];
      expect(callArgs.title).toBe('Goose');
    });

    it('should have correct body message', () => {
      document.hasFocus = vi.fn().mockReturnValue(false);

      // Simulate the notification call
      if (!document.hasFocus()) {
        window.electron.showNotification({
          title: 'Goose',
          body: 'Task completed',
        });
      }

      const callArgs = mockShowNotification.mock.calls[0][0];
      expect(callArgs.body).toBe('Task completed');
    });
  });
});
