/**
 * @vitest-environment jsdom
 */
import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useChatEngine } from './useChatEngine';
import { Message, getTextContent } from '../types/message';
import { ChatType } from '../types/chat';
import type { Mock } from 'vitest';

// Mock the useMessageStream hook which is a dependency of useChatEngine
vi.mock('./useMessageStream', () => ({
  useMessageStream: vi.fn(),
}));

// Mock the sessions API which is another dependency
vi.mock('../sessions', () => ({
  fetchSessionDetails: vi.fn().mockResolvedValue({ metadata: {} }),
}));

describe('useChatEngine', () => {
  let mockUseMessageStream: Mock;

  beforeEach(async () => {
    // Mock the appConfig and electron APIs on the existing window object
    Object.defineProperty(window, 'appConfig', {
      value: {
        get: vi.fn((key: string) => {
          if (key === 'GOOSE_API_HOST') return 'http://localhost';
          if (key === 'GOOSE_PORT') return '8000';
          return null;
        }),
      },
      writable: true,
    });

    Object.defineProperty(window, 'electron', {
      value: {
        logInfo: vi.fn(),
      },
      writable: true,
    });

    // Dynamically import the hook so we can get a reference to the mock
    const { useMessageStream } = await import('./useMessageStream');
    mockUseMessageStream = useMessageStream as Mock;

    // Reset all mocks before each test to ensure a clean state
    vi.clearAllMocks();

    // Provide a complete, default mock implementation for useMessageStream
    mockUseMessageStream.mockReturnValue({
      messages: [],
      append: vi.fn(),
      stop: vi.fn(),
      chatState: 'idle',
      error: undefined,
      setMessages: vi.fn(),
      input: '',
      setInput: vi.fn(),
      handleInputChange: vi.fn(),
      handleSubmit: vi.fn(),
      updateMessageStreamBody: vi.fn(),
      notifications: [],
      sessionMetadata: undefined,
      setError: vi.fn(),
    });
  });

  describe('onMessageUpdate', () => {
    it('should truncate history and append the updated message when a message is edited', () => {
      // --- 1. ARRANGE ---
      const initialMessages: Message[] = [
        { id: '1', role: 'user', content: [{ type: 'text', text: 'First message' }], created: 0 },
        {
          id: '2',
          role: 'assistant',
          content: [{ type: 'text', text: 'First response' }],
          created: 1,
        },
        {
          id: '3',
          role: 'user',
          content: [{ type: 'text', text: 'Message to be edited' }],
          created: 2,
        },
        {
          id: '4',
          role: 'assistant',
          content: [{ type: 'text', text: 'Response to be deleted' }],
          created: 3,
        },
      ];

      const mockSetMessages = vi.fn();
      const mockAppend = vi.fn();

      // Configure the mock to return specific values for this test case
      mockUseMessageStream.mockReturnValue({
        messages: initialMessages,
        append: mockAppend,
        setMessages: mockSetMessages,
        notifications: [],
        stop: vi.fn(),
        chatState: 'idle',
        error: undefined,
        input: '',
        setInput: vi.fn(),
        handleInputChange: vi.fn(),
        handleSubmit: vi.fn(),
        updateMessageStreamBody: vi.fn(),
        sessionMetadata: undefined,
        setError: vi.fn(),
      });

      const mockChat: ChatType = {
        id: 'test-chat',
        messages: initialMessages,
        title: 'Test Chat',
        messageHistoryIndex: 0,
      };

      // Render the hook with our test setup
      const { result } = renderHook(() =>
        useChatEngine({
          chat: mockChat,
          setChat: vi.fn(),
        })
      );

      const messageIdToUpdate = '3';
      const newContent = 'This is the edited message.';

      // --- 2. ACT ---
      // Call the function we want to test
      act(() => {
        result.current.onMessageUpdate(messageIdToUpdate, newContent);
      });

      // --- 3. ASSERT ---
      // Verify that setMessages was called with the correctly truncated history
      const expectedTruncatedHistory = initialMessages.slice(0, 2);
      expect(mockSetMessages).toHaveBeenCalledWith(expectedTruncatedHistory);

      // Verify that append was called with the new message
      expect(mockAppend).toHaveBeenCalledTimes(1);
      const appendedMessage = mockAppend.mock.calls[0][0];
      expect(getTextContent(appendedMessage)).toBe(newContent);
      expect(appendedMessage.role).toBe('user');
    });
  });
});
