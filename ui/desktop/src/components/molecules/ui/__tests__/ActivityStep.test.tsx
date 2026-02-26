import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { ActivityStep, ThinkingEntry } from '../activity-step';

describe('ActivityStep', () => {
  it('renders description and tool icon', () => {
    render(<ActivityStep description="Running ls command" toolName="developer__shell" />);
    expect(screen.getByText('Running ls command')).toBeInTheDocument();
  });

  it('shows spinner when active', () => {
    const { container } = render(
      <ActivityStep description="Running..." toolName="developer__shell" isActive={true} />
    );
    const spinner = container.querySelector('.animate-spin');
    expect(spinner).toBeInTheDocument();
  });

  it('shows checkmark when complete (not active)', () => {
    const { container } = render(
      <ActivityStep description="Done" toolName="developer__shell" isActive={false} />
    );
    const check = container.querySelector('[class*="text-text-success"]');
    expect(check).toBeInTheDocument();
  });

  it('shows error icon when isError is true', () => {
    const { container } = render(
      <ActivityStep
        description="Failed command"
        toolName="developer__shell"
        isError={true}
        errorMessage="Command not found"
      />
    );
    const errorIcon = container.querySelector('[class*="text-text-danger"]');
    expect(errorIcon).toBeInTheDocument();
  });

  it('is not expandable when no toolArgs/toolResult/errorMessage', () => {
    const { container } = render(
      <ActivityStep description="Simple step" toolName="developer__shell" />
    );
    // When not expandable, a spacer span replaces the chevron
    const spacer = container.querySelector('span.w-3');
    expect(spacer).toBeInTheDocument();
    // No ChevronRight SVG should be present
    expect(container.querySelector('svg.lucide-chevron-right')).not.toBeInTheDocument();
  });

  it('is expandable when toolArgs are provided', () => {
    render(
      <ActivityStep
        description="Reading file"
        toolName="developer__text_editor"
        toolArgs={{ path: '/tmp/test.txt', command: 'view' }}
      />
    );
    fireEvent.click(screen.getByText('Reading file'));
    expect(screen.getByText('path')).toBeInTheDocument();
    expect(screen.getByText('/tmp/test.txt')).toBeInTheDocument();
    expect(screen.getByText('command')).toBeInTheDocument();
    expect(screen.getByText('view')).toBeInTheDocument();
  });

  it('is expandable when toolResult is provided', () => {
    render(
      <ActivityStep
        description="Ran command"
        toolName="developer__shell"
        toolResult="total 42\ndrwxr-xr-x 3 user user 4096 Jan 1 00:00 src"
      />
    );
    fireEvent.click(screen.getByText('Ran command'));
    expect(screen.getByText(/total 42/)).toBeInTheDocument();
  });

  it('shows error message when expanded and isError', () => {
    render(
      <ActivityStep
        description="Failed"
        toolName="developer__shell"
        isError={true}
        errorMessage="Permission denied: /root/secret"
      />
    );
    fireEvent.click(screen.getByText('Failed'));
    expect(screen.getByText(/Permission denied/)).toBeInTheDocument();
  });

  it('shows both args and result when expanded', () => {
    render(
      <ActivityStep
        description="Shell command"
        toolName="developer__shell"
        toolArgs={{ command: 'echo hello' }}
        toolResult="hello"
      />
    );
    fireEvent.click(screen.getByText('Shell command'));
    expect(screen.getByText('command')).toBeInTheDocument();
    expect(screen.getByText('echo hello')).toBeInTheDocument();
    expect(screen.getByText('hello')).toBeInTheDocument();
  });

  it('truncates long tool results and allows expansion', () => {
    const longResult = 'x'.repeat(500);
    render(
      <ActivityStep description="Long output" toolName="developer__shell" toolResult={longResult} />
    );
    fireEvent.click(screen.getByText('Long output'));
    expect(screen.getByText(/Show all/)).toBeInTheDocument();
    fireEvent.click(screen.getByText(/Show all/));
    expect(screen.getByText(/Show less/)).toBeInTheDocument();
  });

  it('truncates long string args and allows expansion', () => {
    const longArg = 'a'.repeat(200);
    render(
      <ActivityStep
        description="Long arg"
        toolName="developer__text_editor"
        toolArgs={{ file_text: longArg }}
      />
    );
    fireEvent.click(screen.getByText('Long arg'));
    expect(screen.getByText('more')).toBeInTheDocument();
    fireEvent.click(screen.getByText('more'));
    expect(screen.getByText('less')).toBeInTheDocument();
  });

  it('renders non-string args (objects, booleans, numbers)', () => {
    render(
      <ActivityStep
        description="Complex args"
        toolName="developer__analyze"
        toolArgs={{
          force: true,
          max_depth: 3,
          config: { nested: 'value' },
          empty: null,
        }}
      />
    );
    fireEvent.click(screen.getByText('Complex args'));
    expect(screen.getByText('force')).toBeInTheDocument();
    expect(screen.getByText('true')).toBeInTheDocument();
    expect(screen.getByText('max_depth')).toBeInTheDocument();
    expect(screen.getByText('3')).toBeInTheDocument();
    expect(screen.getByText('null')).toBeInTheDocument();
  });

  it('collapses when clicked again', () => {
    render(
      <ActivityStep
        description="Toggle me"
        toolName="developer__shell"
        toolArgs={{ command: 'pwd' }}
      />
    );
    fireEvent.click(screen.getByText('Toggle me'));
    expect(screen.getByText('command')).toBeInTheDocument();
    fireEvent.click(screen.getByText('Toggle me'));
    expect(screen.queryByText('command')).not.toBeInTheDocument();
  });
});

describe('ThinkingEntry', () => {
  it('renders thinking text in italic style', () => {
    const { container } = render(<ThinkingEntry text="Let me analyze the codebase..." />);
    expect(screen.getByText('Let me analyze the codebase...')).toBeInTheDocument();
    const el = container.querySelector('.italic');
    expect(el).toBeInTheDocument();
  });

  it('applies custom className', () => {
    const { container } = render(<ThinkingEntry text="Thinking..." className="mt-4" />);
    const el = container.querySelector('.mt-4');
    expect(el).toBeInTheDocument();
  });

  it('renders differently from ActivityStep (no icon, no chevron)', () => {
    const { container } = render(<ThinkingEntry text="Internal reasoning" />);
    expect(container.querySelector('svg')).not.toBeInTheDocument();
  });
});
