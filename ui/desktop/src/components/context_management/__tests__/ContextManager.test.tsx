import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { ContextManagerProvider, useContextManager } from '../ContextManager';
import * as contextManagement from '../index';
import { Message } from '../../../api';

const default_message: Message = {
  metadata: {
    agentVisible: false,
    userVisible: false,
  },
  id: '1',
  role: 'assistant',
  created: 1000,
  content: [],
};

// Mock the context management functions
vi.mock('../index', () => ({
  manageContextFromBackend: vi.fn(),
}));

const mockManageContextFromBackend = vi.mocked(contextManagement.manageContextFromBackend);

describe('ContextManager', () => {
  const mockMessages: Message[] = [
    {
      ...default_message,
      content: [{ type: 'text', text: 'Hello' }],
    },
    {
      ...default_message,
      content: [{ type: 'text', text: 'Hi there!' }],
    },
  ];

  const mockSetMessages = vi.fn();
  const mockAppend = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  const renderContextManager = () => {
    return renderHook(() => useContextManager(), {
      wrapper: ({ children }) => <ContextManagerProvider>{children}</ContextManagerProvider>,
    });
  };

  describe('Initial State', () => {
    it('should have correct initial state', () => {
      const { result } = renderContextManager();

      expect(result.current.isCompacting).toBe(false);
      expect(result.current.compactionError).toBe(null);
    });
  });

  describe('hasCompactionMarker', () => {
    it('should return true for messages with systemNotification content', () => {
      const { result } = renderContextManager();
      const messageWithMarker: Message = {
        ...default_message,
        content: [
          {
            type: 'systemNotification',
            msg: 'Compaction marker',
            notificationType: 'inlineMessage',
          },
        ],
      };

      expect(result.current.hasCompactionMarker(messageWithMarker)).toBe(true);
    });

    it('should return false for messages without systemNotification content', () => {
      const { result } = renderContextManager();
      const regularMessage: Message = {
        ...default_message,
        content: [{ type: 'text', text: 'Hello' }],
      };

      expect(result.current.hasCompactionMarker(regularMessage)).toBe(false);
    });

    it('should return true for messages with mixed content including systemNotification', () => {
      const { result } = renderContextManager();
      const mixedMessage: Message = {
        ...default_message,
        content: [
          { type: 'text', text: 'Some text' },
          {
            type: 'systemNotification',
            msg: 'Compaction marker',
            notificationType: 'inlineMessage',
          },
        ],
      };

      expect(result.current.hasCompactionMarker(mixedMessage)).toBe(true);
    });
  });

  describe('handleManualCompaction', () => {
    it('should perform compaction with server-provided messages', async () => {
      mockManageContextFromBackend.mockResolvedValue({
        messages: [
          {
            ...default_message,
            content: [
              {
                type: 'systemNotification',
                msg: 'Conversation compacted and summarized',
                notificationType: 'inlineMessage',
              },
            ],
          },
          {
            ...default_message,
            content: [{ type: 'text', text: 'Manual summary content' }],
          },
          {
            ...default_message,
            content: [
              {
                type: 'text',
                text: 'The previous message contains a summary that was prepared because a context limit was reached. Do not mention that you read a summary or that conversation summarization occurred Just continue the conversation naturally based on the summarized context',
              },
            ],
          },
        ],
        tokenCounts: [8, 100, 50],
      });

      const { result } = renderContextManager();

      await act(async () => {
        await result.current.handleManualCompaction(
          mockMessages,
          mockSetMessages,
          mockAppend,
          'test-session-id'
        );
      });

      expect(mockManageContextFromBackend).toHaveBeenCalledWith({
        messages: mockMessages,
        manageAction: 'summarize',
        sessionId: 'test-session-id',
      });

      // Verify all three messages are set
      expect(mockSetMessages).toHaveBeenCalledTimes(1);
      const setMessagesCall = mockSetMessages.mock.calls[0][0];
      expect(setMessagesCall).toHaveLength(3);
      expect(setMessagesCall[0]).toMatchObject({
        role: 'assistant',
        content: [{ type: 'conversationCompacted', msg: 'Conversation compacted and summarized' }],
      });
      expect(setMessagesCall[1]).toMatchObject({
        role: 'assistant',
        content: [{ type: 'text', text: 'Manual summary content' }],
      });
      expect(setMessagesCall[2]).toMatchObject({
        role: 'assistant',
        content: [
          {
            type: 'text',
            text: 'The previous message contains a summary that was prepared because a context limit was reached. Do not mention that you read a summary or that conversation summarization occurred Just continue the conversation naturally based on the summarized context',
          },
        ],
      });

      // Fast-forward timers to check if append would be called
      act(() => {
        vi.advanceTimersByTime(150);
      });

      // Should NOT append the continuation message for manual compaction
      expect(mockAppend).not.toHaveBeenCalled();
    });

    it('should work without append function', async () => {
      mockManageContextFromBackend.mockResolvedValue({
        messages: [
          {
            ...default_message,
            content: [{ type: 'text', text: 'Manual summary content' }],
          },
        ],
        tokenCounts: [100, 50],
      });

      const { result } = renderContextManager();

      await act(async () => {
        await result.current.handleManualCompaction(
          mockMessages,
          mockSetMessages,
          undefined // No append function
        );
      });

      expect(mockManageContextFromBackend).toHaveBeenCalled();
      // Should not throw error when append is undefined

      // Fast-forward timers to check if append would be called
      act(() => {
        vi.advanceTimersByTime(150);
      });

      // No append function provided, so no calls should be made
      expect(mockAppend).not.toHaveBeenCalled();
    });

    it('should not auto-continue conversation for manual compaction even with append function', async () => {
      mockManageContextFromBackend.mockResolvedValue({
        messages: [
          {
            ...default_message,
            content: [
              {
                type: 'systemNotification',
                msg: 'Conversation compacted and summarized',
                notificationType: 'inlineMessage',
              },
            ],
          },
          {
            ...default_message,
            content: [{ type: 'text', text: 'Manual summary content' }],
          },
          {
            ...default_message,
            content: [
              {
                type: 'text',
                text: 'The previous message contains a summary that was prepared because a context limit was reached. Do not mention that you read a summary or that conversation summarization occurred Just continue the conversation naturally based on the summarized context',
              },
            ],
          },
        ],
        tokenCounts: [8, 100, 50],
      });

      const { result } = renderContextManager();

      await act(async () => {
        await result.current.handleManualCompaction(
          mockMessages,
          mockSetMessages,
          mockAppend,
          'test-session-id'
        );
      });

      // Verify all three messages are set
      expect(mockSetMessages).toHaveBeenCalledTimes(1);
      const setMessagesCall = mockSetMessages.mock.calls[0][0];
      expect(setMessagesCall).toHaveLength(3);
      expect(setMessagesCall[0]).toMatchObject({
        role: 'assistant',
        content: [{ type: 'conversationCompacted', msg: 'Conversation compacted and summarized' }],
      });
      expect(setMessagesCall[1]).toMatchObject({
        role: 'assistant',
        content: [{ type: 'text', text: 'Manual summary content' }],
      });
      expect(setMessagesCall[2]).toMatchObject({
        role: 'assistant',
        content: [
          {
            type: 'text',
            text: 'The previous message contains a summary that was prepared because a context limit was reached. Do not mention that you read a summary or that conversation summarization occurred Just continue the conversation naturally based on the summarized context',
          },
        ],
      });

      // Fast-forward timers to check if append would be called
      act(() => {
        vi.advanceTimersByTime(150);
      });

      // Should NOT auto-continue for manual compaction, even with append function
      expect(mockAppend).not.toHaveBeenCalled();
    });
  });

  describe('Context Provider Error', () => {
    it('should throw error when useContextManager is used outside provider', () => {
      expect(() => {
        renderHook(() => useContextManager());
      }).toThrow('useContextManager must be used within a ContextManagerProvider');
    });
  });
});
