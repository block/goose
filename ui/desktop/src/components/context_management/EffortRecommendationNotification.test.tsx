import { render, type RenderOptions, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { acpSetSessionThinkingEffort } from '../../acp/providers';
import {
  acpChatSessionStore,
  useAcpChatSessionSnapshot,
  type AcpChatSessionSnapshot,
} from '../../acp/chatSessionStore';
import { IntlTestWrapper } from '../../i18n/test-utils';
import type { SystemNotificationContent } from '../../types/message';
import { EffortRecommendationNotification } from './EffortRecommendationNotification';

vi.mock('../../acp/providers', () => ({
  acpSetSessionThinkingEffort: vi.fn(),
}));

vi.mock('../../acp/chatSessionStore', () => ({
  useAcpChatSessionSnapshot: vi.fn(),
  acpChatSessionStore: { getSnapshot: vi.fn() },
}));

const acpSetSessionThinkingEffortMock = vi.mocked(acpSetSessionThinkingEffort);
const useAcpChatSessionSnapshotMock = vi.mocked(useAcpChatSessionSnapshot);
const getSnapshotMock = vi.mocked(acpChatSessionStore.getSnapshot);

function snapshotWithEffort(
  thinkingEffort: string | null,
  thinkingEffortOptions: string[] | null = null
): AcpChatSessionSnapshot {
  return { thinkingEffort, thinkingEffortOptions } as AcpChatSessionSnapshot;
}

const renderWithIntl = (ui: React.ReactElement, options?: RenderOptions) =>
  render(ui, { wrapper: IntlTestWrapper, ...options });

function notification(
  overrides: Partial<SystemNotificationContent> = {}
): SystemNotificationContent {
  return {
    notificationType: 'effortRecommendation',
    msg: 'The remaining work spans several coupled refactors.',
    data: { difficulty: 'high', recommendedEffort: 'high', currentEffort: 'low' },
    ...overrides,
  };
}

describe('EffortRecommendationNotification', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAcpChatSessionSnapshotMock.mockReturnValue(undefined);
    getSnapshotMock.mockReturnValue(undefined);
  });

  it('shows the reason and applies the recommended effort to the session', async () => {
    acpSetSessionThinkingEffortMock.mockResolvedValueOnce(undefined);

    renderWithIntl(
      <EffortRecommendationNotification notification={notification()} sessionId="session-1" />
    );

    expect(
      screen.getByText('The remaining work spans several coupled refactors.')
    ).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Switch thinking effort to high' }));

    expect(acpSetSessionThinkingEffortMock).toHaveBeenCalledWith('session-1', 'high');
    expect(
      await screen.findByText('Thinking effort set to high for this session')
    ).toBeInTheDocument();
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('shows an error and keeps the button when applying fails', async () => {
    acpSetSessionThinkingEffortMock.mockRejectedValueOnce(new Error('acp down'));

    renderWithIntl(
      <EffortRecommendationNotification
        notification={notification({
          data: { difficulty: 'medium', recommendedEffort: 'medium', currentEffort: null },
        })}
        sessionId="session-2"
      />
    );

    await userEvent.click(screen.getByRole('button', { name: 'Switch thinking effort to medium' }));

    expect(
      await screen.findByText('Could not update the thinking effort. Please try again.')
    ).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: 'Switch thinking effort to medium' })
    ).toBeInTheDocument();
  });

  it('replaces the button with a note when the session already runs at or above the recommendation', () => {
    useAcpChatSessionSnapshotMock.mockReturnValue(snapshotWithEffort('high'));

    renderWithIntl(
      <EffortRecommendationNotification
        notification={notification({
          data: { difficulty: 'medium', recommendedEffort: 'medium', currentEffort: 'low' },
        })}
        sessionId="session-4"
      />
    );

    expect(
      screen.getByText('Thinking effort is already set to high for this session')
    ).toBeInTheDocument();
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('keeps the button when the live session effort is below the recommendation', () => {
    useAcpChatSessionSnapshotMock.mockReturnValue(snapshotWithEffort('low'));

    renderWithIntl(
      <EffortRecommendationNotification
        notification={notification({
          data: { difficulty: 'medium', recommendedEffort: 'medium', currentEffort: 'low' },
        })}
        sessionId="session-5"
      />
    );

    expect(
      screen.getByRole('button', { name: 'Switch thinking effort to medium' })
    ).toBeInTheDocument();
  });

  it('hides the button when the live model cannot run the recommended effort', () => {
    useAcpChatSessionSnapshotMock.mockReturnValue(snapshotWithEffort('off', ['off']));

    renderWithIntl(
      <EffortRecommendationNotification notification={notification()} sessionId="session-7" />
    );

    expect(
      screen.getByText('The remaining work spans several coupled refactors.')
    ).toBeInTheDocument();
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('offers a downgrade with savings copy and applies it', async () => {
    acpSetSessionThinkingEffortMock.mockResolvedValueOnce(undefined);

    renderWithIntl(
      <EffortRecommendationNotification
        notification={notification({
          msg: 'The remaining work is mechanical cleanup.',
          data: { difficulty: 'low', recommendedEffort: 'medium', currentEffort: 'high' },
        })}
        sessionId="session-8"
      />
    );

    await userEvent.click(
      screen.getByRole('button', { name: 'Switch thinking effort to medium to save tokens' })
    );

    expect(acpSetSessionThinkingEffortMock).toHaveBeenCalledWith('session-8', 'medium');
    expect(
      await screen.findByText('Thinking effort set to medium for this session')
    ).toBeInTheDocument();
  });

  it('shows the already-set note when the session is at or below a downgrade recommendation', () => {
    useAcpChatSessionSnapshotMock.mockReturnValue(snapshotWithEffort('low'));

    renderWithIntl(
      <EffortRecommendationNotification
        notification={notification({
          data: { difficulty: 'low', recommendedEffort: 'medium', currentEffort: 'high' },
        })}
        sessionId="session-9"
      />
    );

    expect(
      screen.getByText('Thinking effort is already set to low for this session')
    ).toBeInTheDocument();
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('keeps the downgrade button when the live effort is still above the recommendation', () => {
    useAcpChatSessionSnapshotMock.mockReturnValue(snapshotWithEffort('high'));

    renderWithIntl(
      <EffortRecommendationNotification
        notification={notification({
          data: { difficulty: 'low', recommendedEffort: 'medium', currentEffort: 'high' },
        })}
        sessionId="session-10"
      />
    );

    expect(
      screen.getByRole('button', { name: 'Switch thinking effort to medium to save tokens' })
    ).toBeInTheDocument();
  });

  it('does not apply a downgrade when the session effort was lowered before the click lands', async () => {
    getSnapshotMock.mockReturnValue(snapshotWithEffort('low'));

    renderWithIntl(
      <EffortRecommendationNotification
        notification={notification({
          data: { difficulty: 'low', recommendedEffort: 'medium', currentEffort: 'high' },
        })}
        sessionId="session-11"
      />
    );

    await userEvent.click(
      screen.getByRole('button', { name: 'Switch thinking effort to medium to save tokens' })
    );

    expect(acpSetSessionThinkingEffortMock).not.toHaveBeenCalled();
  });

  it('does not apply when the session effort was raised before the click lands', async () => {
    getSnapshotMock.mockReturnValue(snapshotWithEffort('max'));

    renderWithIntl(
      <EffortRecommendationNotification
        notification={notification({
          data: { difficulty: 'high', recommendedEffort: 'high', currentEffort: 'medium' },
        })}
        sessionId="session-6"
      />
    );

    await userEvent.click(screen.getByRole('button', { name: 'Switch thinking effort to high' }));

    expect(acpSetSessionThinkingEffortMock).not.toHaveBeenCalled();
  });

});
