import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { ContextManagerProvider, useContextManager } from '../ContextManager';
import * as contextManagement from '../index';
import { ContextManageResponse, Message } from '../../../api';

// Mock the context management functions
vi.mock('../index', () => ({
  manageContextFromBackend: vi.fn(),
  convertApiMessageToFrontendMessage: vi.fn(),
}));

const mockManageContextFromBackend = vi.mocked(contextManagement.manageContextFromBackend);

describe('ContextManager', () => {
  const mockMessages: Message[] = [
    {
      id: '1',
      role: 'user',
      created: 1000,
      content: [{ type: 'text', text: 'Hello' }],
    },
    {
      id: '2',
      role: 'assistant',
      created: 2000,
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
      expect(typeof result.current.handleAutoCompaction).toBe('function');
      expect(typeof result.current.handleManualCompaction).toBe('function');
      expect(typeof result.current.hasCompactionMarker).toBe('function');
    });
  });

  describe('hasCompactionMarker', () => {
    it('should return true for messages with summarizationRequested content', () => {
      const { result } = renderContextManager();
      const messageWithMarker: Message = {
        id: '1',
        role: 'assistant',
        created: 1000,
        content: [{ type: 'summarizationRequested', msg: 'Compaction marker' }],
      };

      expect(result.current.hasCompactionMarker(messageWithMarker)).toBe(true);
    });

    it('should return false for messages without summarizationRequested content', () => {
      const { result } = renderContextManager();
      const regularMessage: Message = {
        id: '1',
        role: 'user',
        created: 1000,
        content: [{ type: 'text', text: 'Hello' }],
      };

      expect(result.current.hasCompactionMarker(regularMessage)).toBe(false);
    });

    it('should return true for messages with mixed content including summarizationRequested', () => {
      const { result } = renderContextManager();
      const mixedMessage: Message = {
        id: '1',
        role: 'assistant',
        created: 1000,
        content: [
          { type: 'text', text: 'Some text' },
          { type: 'summarizationRequested', msg: 'Compaction marker' },
        ],
      };

      expect(result.current.hasCompactionMarker(mixedMessage)).toBe(true);
    });
  });

  describe('handleAutoCompaction', () => {
    it('should successfully perform auto compaction with server-provided messages', async () => {
      // Mock the backend response with 3 messages: marker, summary, continuation
      mockManageContextFromBackend.mockResolvedValue({
        messages: [
          {
            role: 'assistant',
            content: [
              { type: 'summarizationRequested', msg: 'Conversation compacted and summarized' },
            ],
          },
          {
            role: 'assistant',
            content: [{ type: 'text', text: 'Summary content' }],
          },
          {
            role: 'assistant',
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
        await result.current.handleAutoCompaction(
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

      // Expect setMessages to be called with all 3 converted messages
      // Note: id and created are generated dynamically, so we check structure instead
      expect(mockSetMessages).toHaveBeenCalledTimes(1);
      const setMessagesCall = mockSetMessages.mock.calls[0][0];
      expect(setMessagesCall).toHaveLength(3);
      expect(setMessagesCall[0]).toMatchObject({
        role: 'assistant',
        content: [{ type: 'summarizationRequested', msg: 'Conversation compacted and summarized' }],
      });
      expect(setMessagesCall[0]).toHaveProperty('id');
      expect(setMessagesCall[0]).toHaveProperty('created');
      expect(setMessagesCall[1]).toMatchObject({
        role: 'assistant',
        content: [{ type: 'text', text: 'Summary content' }],
      });
      expect(setMessagesCall[1]).toHaveProperty('id');
      expect(setMessagesCall[1]).toHaveProperty('created');
      expect(setMessagesCall[2]).toMatchObject({
        role: 'assistant',
        content: [
          {
            type: 'text',
            text: 'The previous message contains a summary that was prepared because a context limit was reached. Do not mention that you read a summary or that conversation summarization occurred Just continue the conversation naturally based on the summarized context',
          },
        ],
      });
      expect(setMessagesCall[2]).toHaveProperty('id');
      expect(setMessagesCall[2]).toHaveProperty('created');

      // Fast-forward timers to trigger the append call
      act(() => {
        vi.advanceTimersByTime(150);
      });

      // Should append the continuation message (index 2) for auto-compaction
      expect(mockAppend).toHaveBeenCalledTimes(1);
      const appendedMessage = mockAppend.mock.calls[0][0];
      expect(appendedMessage).toMatchObject({
        role: 'assistant',
        content: [
          {
            type: 'text',
            text: 'The previous message contains a summary that was prepared because a context limit was reached. Do not mention that you read a summary or that conversation summarization occurred Just continue the conversation naturally based on the summarized context',
          },
        ],
      });
      expect(appendedMessage).toHaveProperty('id');
      expect(appendedMessage).toHaveProperty('created');
    });

    it('should handle compaction errors gracefully', async () => {
      const error = new Error('Backend error');
      mockManageContextFromBackend.mockRejectedValue(error);

      const { result } = renderContextManager();

      await act(async () => {
        await result.current.handleAutoCompaction(
          mockMessages,
          mockSetMessages,
          mockAppend,
          'test-session-id'
        );
      });

      expect(result.current.compactionError).toBe('Backend error');
      expect(result.current.isCompacting).toBe(false);

      expect(mockSetMessages).toHaveBeenCalledWith([
        ...mockMessages,
        expect.objectContaining({
          content: [
            {
              type: 'summarizationRequested',
              msg: 'Compaction failed. Please try again or start a new session.',
            },
          ],
        }),
      ]);
    });

    it('should set isCompacting state correctly during operation', async () => {
      let resolvePromise: (value: ContextManageResponse) => void;
      const promise = new Promise<ContextManageResponse>((resolve) => {
        resolvePromise = resolve;
      });

      mockManageContextFromBackend.mockReturnValue(promise);

      const { result } = renderContextManager();

      // Start compaction
      act(() => {
        result.current.handleAutoCompaction(
          mockMessages,
          mockSetMessages,
          mockAppend,
          'test-session-id'
        );
      });

      // Should be compacting
      expect(result.current.isCompacting).toBe(true);
      expect(result.current.compactionError).toBe(null);

      // Resolve the backend call
      resolvePromise!({
        messages: [
          {
            role: 'assistant',
            content: [{ type: 'text', text: 'Summary content' }],
          },
        ],
        tokenCounts: [100, 50],
      });

      await act(async () => {
        await promise;
      });

      // Should no longer be compacting
      expect(result.current.isCompacting).toBe(false);
    });

    it('preserves display: false for ancestor messages', async () => {
      mockManageContextFromBackend.mockResolvedValue({ messages: [], tokenCounts: [] });

      const hiddenMessage: Message = {
        id: 'hidden-1',
        role: 'user',
        created: 1500,
        content: [{ type: 'text', text: 'Secret' }],
      };

      const visibleMessage: Message = {
        id: 'visible-1',
        role: 'assistant',
        created: 1600,
        content: [{ type: 'text', text: 'Public' }],
      };

      const messages: Message[] = [hiddenMessage, visibleMessage];

      const { result } = renderContextManager();

      await act(async () => {
        await result.current.handleAutoCompaction(
          messages,
          mockSetMessages,
          mockAppend,
          'test-session-id'
        );
      });

      // No server messages -> setMessages called with empty list
      expect(mockSetMessages).toHaveBeenCalledWith([]);
      expect(mockAppend).not.toHaveBeenCalled();
    });
  });

  describe('handleManualCompaction', () => {
    it('should perform compaction with server-provided messages', async () => {
      mockManageContextFromBackend.mockResolvedValue({
        messages: [
          {
            role: 'assistant',
            content: [
              { type: 'summarizationRequested', msg: 'Conversation compacted and summarized' },
            ],
          },
          {
            role: 'assistant',
            content: [{ type: 'text', text: 'Manual summary content' }],
          },
          {
            role: 'assistant',
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
        content: [{ type: 'summarizationRequested', msg: 'Conversation compacted and summarized' }],
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
            role: 'assistant',
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
            role: 'assistant',
            content: [
              { type: 'summarizationRequested', msg: 'Conversation compacted and summarized' },
            ],
          },
          {
            role: 'assistant',
            content: [{ type: 'text', text: 'Manual summary content' }],
          },
          {
            role: 'assistant',
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
        content: [{ type: 'summarizationRequested', msg: 'Conversation compacted and summarized' }],
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

  describe('Error Handling', () => {
    it('should handle backend errors with unknown error type', async () => {
      mockManageContextFromBackend.mockRejectedValue('String error');

      const { result } = renderContextManager();

      await act(async () => {
        await result.current.handleAutoCompaction(
          mockMessages,
          mockSetMessages,
          mockAppend,
          'test-session-id'
        );
      });

      expect(result.current.compactionError).toBe('Unknown error during compaction');
    });

    it('should handle missing summary content gracefully with server-provided messages', async () => {
      mockManageContextFromBackend.mockResolvedValue({
        messages: [
          {
            role: 'assistant',
            content: [
              { type: 'toolResponse', id: 'test', toolResult: { content: 'Not text content' } },
            ],
          },
        ],
        tokenCounts: [100, 50],
      });

      const { result } = renderContextManager();

      await act(async () => {
        await result.current.handleAutoCompaction(
          mockMessages,
          mockSetMessages,
          mockAppend,
          'test-session-id'
        );
      });

      // Should complete without error even if content is not text
      expect(result.current.isCompacting).toBe(false);
      expect(result.current.compactionError).toBe(null);

      // Should still set messages with the converted message (with generated id/created)
      expect(mockSetMessages).toHaveBeenCalledTimes(1);
      const setMessagesCall = mockSetMessages.mock.calls[0][0];
      expect(setMessagesCall).toHaveLength(1);
      expect(setMessagesCall[0]).toMatchObject({
        role: 'assistant',
        content: [
          { type: 'toolResponse', id: 'test', toolResult: { content: 'Not text content' } },
        ],
      });
      expect(setMessagesCall[0]).toHaveProperty('id');
      expect(setMessagesCall[0]).toHaveProperty('created');
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
