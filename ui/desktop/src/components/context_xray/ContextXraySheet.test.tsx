import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ContextReport } from '../../types/contextReport';
import { IntlTestWrapper } from '../../i18n/test-utils';
import { ContextXraySheet } from './ContextXraySheet';

const getContextReport = vi.hoisted(() => vi.fn());

vi.mock('../../acp/contextReport', () => ({ getContextReport }));

vi.mock('./CompactionControls', () => ({
  CompactionControls: ({
    onCompact,
    compactDisabled,
  }: {
    onCompact?: () => void;
    compactDisabled?: boolean;
  }) => (
    <button type="button" onClick={onCompact} disabled={compactDisabled}>
      Compact now
    </button>
  ),
}));

class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}

beforeAll(() => {
  vi.stubGlobal('ResizeObserver', ResizeObserverStub);
});

afterAll(() => {
  vi.unstubAllGlobals();
});

function reportWith(wireTotalTokens: number): ContextReport {
  return {
    model: { modelName: 'test-model', contextLimit: 200_000 },
    estimatedTotalTokens: wireTotalTokens,
    wireTotalTokens,
    segments: [
      {
        category: 'messages',
        label: 'Conversation',
        tokenCount: wireTotalTokens,
        charCount: wireTotalTokens * 4,
      },
    ],
  };
}

function renderSheet(props: Partial<React.ComponentProps<typeof ContextXraySheet>> = {}) {
  return render(
    <IntlTestWrapper>
      <ContextXraySheet open onOpenChange={() => {}} sessionId="session-1" {...props} />
    </IntlTestWrapper>
  );
}

beforeEach(() => {
  vi.useRealTimers();
  getContextReport.mockReset();
});

describe('ContextXraySheet error recovery', () => {
  it('clears the error once a background refresh succeeds', async () => {
    getContextReport.mockRejectedValueOnce(new Error('backend hiccup'));
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});

    const { rerender } = renderSheet({ refreshSignal: 1 });

    expect(await screen.findByText('Could not load the context report.')).toBeInTheDocument();

    getContextReport.mockResolvedValue(reportWith(38_200));
    rerender(
      <IntlTestWrapper>
        <ContextXraySheet open onOpenChange={() => {}} sessionId="session-1" refreshSignal={2} />
      </IntlTestWrapper>
    );

    expect(await screen.findByText('38.2k')).toBeInTheDocument();
    expect(screen.queryByText('Could not load the context report.')).not.toBeInTheDocument();

    consoleError.mockRestore();
  });
});

describe('ContextXraySheet compaction', () => {
  it('refetches after the compaction finishes, not when the button is clicked', async () => {
    const user = userEvent.setup();
    getContextReport.mockResolvedValue(reportWith(121_500));
    const onCompact = vi.fn();

    const { rerender } = renderSheet({ onCompact, agentBusy: false, refreshSignal: 1 });

    await screen.findByText('121.5k');
    expect(getContextReport).toHaveBeenCalledTimes(1);

    await user.click(screen.getByRole('button', { name: 'Compact now' }));
    expect(onCompact).toHaveBeenCalledTimes(1);
    expect(getContextReport).toHaveBeenCalledTimes(1);

    const withBusy = (agentBusy: boolean) => (
      <IntlTestWrapper>
        <ContextXraySheet
          open
          onOpenChange={() => {}}
          sessionId="session-1"
          onCompact={onCompact}
          agentBusy={agentBusy}
          refreshSignal={1}
        />
      </IntlTestWrapper>
    );

    rerender(withBusy(true));
    expect(getContextReport).toHaveBeenCalledTimes(1);

    getContextReport.mockResolvedValue(reportWith(12_345));
    rerender(withBusy(false));

    await waitFor(() => expect(getContextReport).toHaveBeenCalledTimes(2));
    expect(await screen.findByText('12.3k')).toBeInTheDocument();
  });

  it('disables compaction while the agent is working', async () => {
    getContextReport.mockResolvedValue(reportWith(121_500));

    renderSheet({ onCompact: vi.fn(), agentBusy: true });

    expect(await screen.findByRole('button', { name: 'Compact now' })).toBeDisabled();
  });
});
