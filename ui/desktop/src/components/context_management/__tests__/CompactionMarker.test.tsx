import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { CompactionMarker } from '../CompactionMarker';
import { Message } from '../../../api';

const default_message: Message = {
  metadata: {
    agentVisible: false,
    userVisible: false,
  },
  id: '1',
  role: 'assistant',
  created: 1000,content: []
};

describe('CompactionMarker', () => {
  it('should render default message when no conversationCompacted content found', () => {
    const message: Message = {
      ...default_message,
      content: [{ type: 'text', text: 'Regular message' }],
    };

    render(<CompactionMarker message={message} />);

    expect(screen.getByText('Conversation compacted')).toBeInTheDocument();
  });

  it('should render custom message from conversationCompacted content', () => {
    const message: Message = {
      ...default_message,
      content: [
        { type: 'text', text: 'Some other content' },
        { type: 'conversationCompacted', msg: 'Custom compaction message' },
      ],
    };

    render(<CompactionMarker message={message} />);

    expect(screen.getByText('Custom compaction message')).toBeInTheDocument();
  });

  it('should handle empty message content array', () => {
    const message: Message = {
      ...default_message,
      content: [],
    };

    render(<CompactionMarker message={message} />);

    expect(screen.getByText('Conversation compacted')).toBeInTheDocument();
  });

  it('should handle summarizationRequested content with empty msg', () => {
    const message: Message = {
      ...default_message,
      content: [{ type: 'conversationCompacted', msg: '' }],
    };

    render(<CompactionMarker message={message} />);

    // Empty string falls back to default due to || operator
    expect(screen.getByText('Conversation compacted')).toBeInTheDocument();
  });

  it('should handle summarizationRequested content with undefined msg', () => {
    const message: Message = {
      ...default_message,
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      content: [{ type: 'conversationCompacted' } as any],
    };

    render(<CompactionMarker message={message} />);

    // Should render the default message when msg is undefined
    expect(screen.getByText('Conversation compacted')).toBeInTheDocument();
  });
});
