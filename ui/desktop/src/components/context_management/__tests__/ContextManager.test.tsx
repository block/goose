import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { ContextManagerProvider, useContextManager } from '../ContextManager';
import { Message } from '../../../types/message';
import * as contextManagement from '../index';
import { ContextManageResponse } from '../../../api';

// Mock the context management functions
vi.mock('../index', () => ({
  manageContextFromBackend: vi.fn(),
  convertApiMessageToFrontendMessage: vi.fn(),
}));

const mockManageContextFromBackend = vi.mocked(contextManagement.manageContextFromBackend);
const mockConvertApiMessageToFrontendMessage = vi.mocked(
  contextManagement.convertApiMessageToFrontendMessage
);

describe('ContextManager', () => {
  const mockMessages: Message[] = [
    {
      id: '1',
      role: 'user',
      created: 1000,
      content: [{ type: 'text', text: 'Hello' }],
      display: true,
      sendToLLM: true,
    },
    {
      id: '2',
      role: 'assistant',
      created: 2000,
      content: [{ type: 'text', text: 'Hi there!' }],
      display: true,
      sendToLLM: true,
    },
  ];

  const mockSummaryMessage: Message = {
    id: 'summary-1',
    role: 'assistant',
    created: 3000,
    content: [{ type: 'text', text: 'This is a summary of the conversation.' }],
    display: false,
    sendToLLM: true,
  };

  const mockSetMessages = vi.fn();
  const mockAppend = vi.fn();
  const mockSetAncestorMessages = vi.fn();

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
        display: true,
        sendToLLM: false,
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
        display: true,
        sendToLLM: true,
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
        display: true,
        sendToLLM: false,
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

      const mockCompactionMarker: Message = {
        id: 'marker-1',
        role: 'assistant',
        created: 3000,
        content: [{ type: 'summarizationRequested', msg: 'Conversation compacted and summarized' }],
        display: true,
        sendToLLM: false,
      };

      const mockContinuationMessage: Message = {
        id: 'continuation-1',
        role: 'assistant',
        created: 3000,
        content: [
          {
            type: 'text',
            text: 'The previous message contains a summary that was prepared because a context limit was reached. Do not mention that you read a summary or that conversation summarization occurred Just continue the conversation naturally based on the summarized context',
          },
        ],
        display: false,
        sendToLLM: true,
      };

      // Mock the conversion function to return different messages based on call order
      mockConvertApiMessageToFrontendMessage
        .mockReturnValueOnce(mockCompactionMarker) // First call - compaction marker (display: true, sendToLLM: false)
        .mockReturnValueOnce(mockSummaryMessage) // Second call - summary (display: false, sendToLLM: true)
        .mockReturnValueOnce(mockContinuationMessage); // Third call - continuation (display: false, sendToLLM: true)

      const { result } = renderContextManager();

      await act(async () => {
        await result.current.handleAutoCompaction(
          mockMessages,
          mockSetMessages,
          mockAppend,
          mockSetAncestorMessages
        );
      });

      expect(mockManageContextFromBackend).toHaveBeenCalledWith({
        messages: mockMessages,
        manageAction: 'summarize',
      });

      // Verify conversion calls with correct parameters
      expect(mockConvertApiMessageToFrontendMessage).toHaveBeenNthCalledWith(
        1,
        expect.objectContaining({
          content: [
            { type: 'summarizationRequested', msg: 'Conversation compacted and summarized' },
          ],
        }),
        true, // display: true
        false // sendToLLM: false
      );
      expect(mockConvertApiMessageToFrontendMessage).toHaveBeenNthCalledWith(
        2,
        expect.objectContaining({
          content: [{ type: 'text', text: 'Summary content' }],
        }),
        false, // display: false
        true // sendToLLM: true
      );
      expect(mockConvertApiMessageToFrontendMessage).toHaveBeenNthCalledWith(
        3,
        expect.objectContaining({
          content: [
            {
              type: 'text',
              text: expect.stringContaining('The previous message contains a summary'),
            },
          ],
        }),
        false, // display: false
        true // sendToLLM: true
      );

      expect(mockSetAncestorMessages).toHaveBeenCalledWith(
        expect.arrayContaining([
          expect.objectContaining({
            id: '1',
            display: true,
            sendToLLM: false,
          }),
          expect.objectContaining({
            id: '2',
            display: true,
            sendToLLM: false,
          }),
        ])
      );

      // Expect setMessages to be called with all 3 converted messages
      expect(mockSetMessages).toHaveBeenCalledWith([
        mockCompactionMarker,
        mockSummaryMessage,
        mockContinuationMessage,
      ]);

      // Fast-forward timers to trigger the append call
      act(() => {
        vi.advanceTimersByTime(150);
      });

      // Should append the continuation message (index 2) for auto-compaction
      expect(mockAppend).toHaveBeenCalledTimes(1);
      expect(mockAppend).toHaveBeenCalledWith(mockContinuationMessage);
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
          mockSetAncestorMessages
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
          mockSetAncestorMessages
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

      mockConvertApiMessageToFrontendMessage.mockReturnValue(mockSummaryMessage);

      await act(async () => {
        await promise;
      });

      // Should no longer be compacting
      expect(result.current.isCompacting).toBe(false);
    });
    it('preserves display: false for ancestor messages', async () => {
      // Backend returns no new messages; we're validating ancestor behavior only
      mockManageContextFromBackend.mockResolvedValue({ messages: [], tokenCounts: [] });

      const hiddenMessage: Message = {
        id: 'hidden-1',
        role: 'user',
        created: 1500,
        content: [{ type: 'text', text: 'Secret' }],
        display: false,
        sendToLLM: true,
      };

      const visibleMessage: Message = {
        id: 'visible-1',
        role: 'assistant',
        created: 1600,
        content: [{ type: 'text', text: 'Public' }],
        display: true,
        sendToLLM: true,
      };

      const messages: Message[] = [hiddenMessage, visibleMessage];

      const { result } = renderContextManager();

      await act(async () => {
        await result.current.handleAutoCompaction(
          messages,
          mockSetMessages,
          mockAppend,
          mockSetAncestorMessages
        );
      });

      expect(mockSetAncestorMessages).toHaveBeenCalledWith(
        expect.arrayContaining([
          expect.objectContaining({ id: 'hidden-1', display: false, sendToLLM: false }),
          expect.objectContaining({ id: 'visible-1', display: true, sendToLLM: false }),
        ])
      );

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

      const mockCompactionMarker: Message = {
        id: 'marker-1',
        role: 'assistant',
        created: 3000,
        content: [{ type: 'summarizationRequested', msg: 'Conversation compacted and summarized' }],
        display: true,
        sendToLLM: false,
      };

      const mockContinuationMessage: Message = {
        id: 'continuation-1',
        role: 'assistant',
        created: 3000,
        content: [
          {
            type: 'text',
            text: 'The previous message contains a summary that was prepared because a context limit was reached. Do not mention that you read a summary or that conversation summarization occurred Just continue the conversation naturally based on the summarized context',
          },
        ],
        display: false,
        sendToLLM: true,
      };

      mockConvertApiMessageToFrontendMessage
        .mockReturnValueOnce(mockCompactionMarker)
        .mockReturnValueOnce(mockSummaryMessage)
        .mockReturnValueOnce(mockContinuationMessage);

      const { result } = renderContextManager();

      await act(async () => {
        await result.current.handleManualCompaction(
          mockMessages,
          mockSetMessages,
          mockAppend,
          mockSetAncestorMessages
        );
      });

      expect(mockManageContextFromBackend).toHaveBeenCalledWith({
        messages: mockMessages,
        manageAction: 'summarize',
      });

      // Verify all three messages are set
      expect(mockSetMessages).toHaveBeenCalledWith([
        mockCompactionMarker,
        mockSummaryMessage,
        mockContinuationMessage,
      ]);

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

      mockConvertApiMessageToFrontendMessage.mockReturnValue(mockSummaryMessage);

      const { result } = renderContextManager();

      await act(async () => {
        await result.current.handleManualCompaction(
          mockMessages,
          mockSetMessages,
          undefined, // No append function
          mockSetAncestorMessages
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

      const mockCompactionMarker: Message = {
        id: 'marker-1',
        role: 'assistant',
        created: 3000,
        content: [{ type: 'summarizationRequested', msg: 'Conversation compacted and summarized' }],
        display: true,
        sendToLLM: false,
      };

      const mockContinuationMessage: Message = {
        id: 'continuation-1',
        role: 'assistant',
        created: 3000,
        content: [
          {
            type: 'text',
            text: 'The previous message contains a summary that was prepared because a context limit was reached. Do not mention that you read a summary or that conversation summarization occurred Just continue the conversation naturally based on the summarized context',
          },
        ],
        display: false,
        sendToLLM: true,
      };

      mockConvertApiMessageToFrontendMessage
        .mockReturnValueOnce(mockCompactionMarker)
        .mockReturnValueOnce(mockSummaryMessage)
        .mockReturnValueOnce(mockContinuationMessage);

      const { result } = renderContextManager();

      await act(async () => {
        await result.current.handleManualCompaction(
          mockMessages,
          mockSetMessages,
          mockAppend, // Provide append function
          mockSetAncestorMessages
        );
      });

      // Verify all three messages are set
      expect(mockSetMessages).toHaveBeenCalledWith([
        mockCompactionMarker,
        mockSummaryMessage,
        mockContinuationMessage,
      ]);

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
          mockSetAncestorMessages
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

      const mockMessageWithoutText: Message = {
        id: 'summary-1',
        role: 'assistant',
        created: 3000,
        content: [{ type: 'toolResponse', id: 'test', toolResult: { status: 'success' } }],
        display: false,
        sendToLLM: true,
      };

      mockConvertApiMessageToFrontendMessage.mockReturnValue(mockMessageWithoutText);

      const { result } = renderContextManager();

      await act(async () => {
        await result.current.handleAutoCompaction(
          mockMessages,
          mockSetMessages,
          mockAppend,
          mockSetAncestorMessages
        );
      });

      // Should complete without error even if content is not text
      expect(result.current.isCompacting).toBe(false);
      expect(result.current.compactionError).toBe(null);

      // Should still set messages with the converted message
      expect(mockSetMessages).toHaveBeenCalledWith([mockMessageWithoutText]);
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
