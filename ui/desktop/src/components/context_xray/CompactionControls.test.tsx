import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { IntlTestWrapper } from '../../i18n/test-utils';
import { CompactionControls } from './CompactionControls';

const mocks = vi.hoisted(() => ({
  read: vi.fn(),
  upsert: vi.fn(),
  remove: vi.fn(),
  saveAutoCompactThreshold: vi.fn(),
  listProviderDetails: vi.fn(),
  toastError: vi.fn(),
}));

vi.mock('../ConfigContext', () => ({
  useConfig: () => ({ read: mocks.read, upsert: mocks.upsert, remove: mocks.remove }),
}));

vi.mock('../../acp/providers', () => ({
  acpListProviderDetails: mocks.listProviderDetails,
  acpSaveAutoCompactThreshold: mocks.saveAutoCompactThreshold,
}));

vi.mock('../../toasts', () => ({ toastError: mocks.toastError }));

// react-select measures its menu with ResizeObserver, which jsdom lacks.
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

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

const STORED: Record<string, unknown> = {
  GOOSE_AUTO_COMPACT_THRESHOLD: 0.8,
  GOOSE_TOOL_PAIR_SUMMARIZATION: true,
  GOOSE_TOOL_CALL_CUTOFF: 24,
  GOOSE_COMPACTION_MODEL: null,
  GOOSE_FAST_MODEL: null,
};

beforeEach(() => {
  vi.clearAllMocks();
  vi.spyOn(console, 'error').mockImplementation(() => {});
  mocks.read.mockImplementation(async (key: string) => STORED[key] ?? null);
  mocks.upsert.mockResolvedValue(undefined);
  mocks.remove.mockResolvedValue(undefined);
  mocks.saveAutoCompactThreshold.mockResolvedValue(undefined);
  mocks.listProviderDetails.mockResolvedValue([]);
});

afterEach(() => {
  vi.restoreAllMocks();
});

async function renderControls() {
  render(<CompactionControls provider={null} contextLimit={200_000} />, {
    wrapper: IntlTestWrapper,
  });
  const threshold = await screen.findByLabelText('Auto-compact at, percent');
  await waitFor(() => expect(threshold).toBeEnabled());
  return {
    threshold: threshold as HTMLInputElement,
    cutoff: screen.getByLabelText('Keep last, tool calls') as HTMLInputElement,
  };
}

function commit(input: HTMLInputElement, value: string) {
  fireEvent.change(input, { target: { value } });
  fireEvent.blur(input);
}

describe('CompactionControls threshold input', () => {
  it('loads the persisted threshold as a percent', async () => {
    const { threshold } = await renderControls();
    expect(threshold.value).toBe('80');
  });

  it.each(['0', '0.4', 'abc'])(
    'reverts %s instead of persisting the most aggressive threshold',
    async (typed) => {
      const { threshold } = await renderControls();

      commit(threshold, typed);

      expect(threshold.value).toBe('80');
      expect(mocks.saveAutoCompactThreshold).not.toHaveBeenCalled();
    }
  );

  it('reverts an empty threshold to the saved value', async () => {
    const { threshold } = await renderControls();

    commit(threshold, '');

    expect(threshold.value).toBe('80');
    expect(mocks.saveAutoCompactThreshold).not.toHaveBeenCalled();
  });

  it('persists an in-range threshold as a fraction', async () => {
    const { threshold } = await renderControls();

    commit(threshold, '50');

    expect(threshold.value).toBe('50');
    expect(mocks.saveAutoCompactThreshold).toHaveBeenCalledWith(0.5);
  });

  it('persists 100 percent, the only value the backend accepts to disable auto-compaction', async () => {
    const { threshold } = await renderControls();

    commit(threshold, '100');

    expect(threshold.value).toBe('100');
    expect(mocks.saveAutoCompactThreshold).toHaveBeenCalledWith(1);
  });

  it('shows a stored disabled threshold as 100 rather than 99', async () => {
    mocks.read.mockImplementation(async (key: string) =>
      key === 'GOOSE_AUTO_COMPACT_THRESHOLD' ? 1 : (STORED[key] ?? null)
    );

    const { threshold } = await renderControls();

    expect(threshold.value).toBe('100');
  });
});

describe('CompactionControls keep-last input', () => {
  it('loads and persists an in-range cutoff', async () => {
    const { cutoff } = await renderControls();
    expect(cutoff.value).toBe('24');

    commit(cutoff, '40');

    expect(cutoff.value).toBe('40');
    expect(mocks.upsert).toHaveBeenCalledWith('GOOSE_TOOL_CALL_CUTOFF', 40, false);
  });

  it('reverts an out-of-range cutoff instead of clamping', async () => {
    const { cutoff } = await renderControls();

    commit(cutoff, '900');

    expect(cutoff.value).toBe('24');
    expect(mocks.upsert).not.toHaveBeenCalled();
  });

  it('clears the override when emptied', async () => {
    const { cutoff } = await renderControls();

    commit(cutoff, '');

    expect(mocks.remove).toHaveBeenCalledWith('GOOSE_TOOL_CALL_CUTOFF', false);
  });
});

describe('CompactionControls summarize toggle', () => {
  it('persists the new value', async () => {
    await renderControls();

    await userEvent.click(screen.getByRole('switch'));

    expect(mocks.upsert).toHaveBeenCalledWith('GOOSE_TOOL_PAIR_SUMMARIZATION', false, false);
  });

  it('rolls back when its own save fails', async () => {
    mocks.upsert.mockRejectedValueOnce(new Error('nope'));
    await renderControls();
    const toggle = screen.getByRole('switch');

    await userEvent.click(toggle);

    await waitFor(() => expect(toggle).toBeChecked());
    expect(mocks.toastError).toHaveBeenCalled();
  });
});

describe('CompactionControls concurrent saves', () => {
  it('rolls back when the newest save rejects', async () => {
    const first = deferred<void>();
    mocks.saveAutoCompactThreshold.mockReturnValueOnce(first.promise);
    const { threshold } = await renderControls();

    commit(threshold, '50');
    first.reject(new Error('rejected'));

    await waitFor(() => expect(threshold.value).toBe('80'));
    expect(mocks.toastError).toHaveBeenCalled();
  });

  it('keeps the newest value when an older save rejects after a newer one resolves', async () => {
    const first = deferred<void>();
    const second = deferred<void>();
    mocks.saveAutoCompactThreshold
      .mockReturnValueOnce(first.promise)
      .mockReturnValueOnce(second.promise);
    const { threshold } = await renderControls();

    commit(threshold, '50');
    commit(threshold, '60');
    expect(mocks.saveAutoCompactThreshold).toHaveBeenNthCalledWith(1, 0.5);
    expect(mocks.saveAutoCompactThreshold).toHaveBeenNthCalledWith(2, 0.6);

    second.resolve();
    await waitFor(() => expect(threshold.value).toBe('60'));

    first.reject(new Error('stale rejection'));
    await waitFor(() => expect(mocks.toastError).toHaveBeenCalled());

    expect(threshold.value).toBe('60');
  });
});
