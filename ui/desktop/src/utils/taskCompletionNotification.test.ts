/* eslint-disable @typescript-eslint/no-explicit-any */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { notifyTaskCompletion } from './taskCompletionNotification';

/**
 * Tests for task completion notification functionality.
 *
 * When Goose finishes a task (stream finishes), a native OS notification
 * should be shown if the window is not focused. This helps users who
 * switch to other apps while waiting for Goose to complete.
 */

describe('notifyTaskCompletion', () => {
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

  it('should show notification when window is not focused', () => {
    document.hasFocus = vi.fn().mockReturnValue(false);

    notifyTaskCompletion();

    expect(mockShowNotification).toHaveBeenCalledTimes(1);
    expect(mockShowNotification).toHaveBeenCalledWith({
      title: 'Goose',
      body: 'Task completed',
    });
  });

  it('should NOT show notification when window is focused', () => {
    document.hasFocus = vi.fn().mockReturnValue(true);

    notifyTaskCompletion();

    expect(mockShowNotification).not.toHaveBeenCalled();
  });
});
