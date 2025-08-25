import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { CompactionMarker } from '../CompactionMarker';
import { Message } from '../../../types/message';

describe('CompactionMarker', () => {
  it('should render default message when no summarizationRequested content found', () => {
    const message: Message = {
      id: '1',
      role: 'assistant',
      created: 1000,
      content: [{ type: 'text', text: 'Regular message' }],
      display: true,
      sendToLLM: false,
    };

    render(<CompactionMarker message={message} />);

    expect(screen.getByText('Conversation compacted')).toBeInTheDocument();
  });

  it('should render custom message from summarizationRequested content', () => {
    const message: Message = {
      id: '1',
      role: 'assistant',
      created: 1000,
      content: [
        { type: 'text', text: 'Some other content' },
        { type: 'summarizationRequested', msg: 'Custom compaction message' },
      ],
      display: true,
      sendToLLM: false,
    };

    render(<CompactionMarker message={message} />);

    expect(screen.getByText('Custom compaction message')).toBeInTheDocument();
  });

  it('should handle empty message content array', () => {
    const message: Message = {
      id: '1',
      role: 'assistant',
      created: 1000,
      content: [],
      display: true,
      sendToLLM: false,
    };

    render(<CompactionMarker message={message} />);

    expect(screen.getByText('Conversation compacted')).toBeInTheDocument();
  });

  it('should handle summarizationRequested content with empty msg', () => {
    const message: Message = {
      id: '1',
      role: 'assistant',
      created: 1000,
      content: [{ type: 'summarizationRequested', msg: '' }],
      display: true,
      sendToLLM: false,
    };

    render(<CompactionMarker message={message} />);

    // Empty string falls back to default due to || operator
    expect(screen.getByText('Conversation compacted')).toBeInTheDocument();
  });

  it('should handle summarizationRequested content with undefined msg', () => {
    const message: Message = {
      id: '1',
      role: 'assistant',
      created: 1000,
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      content: [{ type: 'summarizationRequested' } as any],
      display: true,
      sendToLLM: false,
    };

    render(<CompactionMarker message={message} />);

    // Should render the default message when msg is undefined
    expect(screen.getByText('Conversation compacted')).toBeInTheDocument();
  });
});
