/**
 * @vitest-environment jsdom
 */
import { act, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  MemoryRouter,
  type NavigateFunction,
  Route,
  Routes,
  useLocation,
  useNavigate,
} from 'react-router-dom';
import ChatSessionsContainer from '../ChatSessionsContainer';
import { AppEvents } from '../../constants/events';

vi.mock('../BaseChat', () => ({
  default: ({ sessionId, isActiveSession }: { sessionId: string; isActiveSession: boolean }) => (
    <div data-testid={`base-chat-${sessionId}`} data-active={isActiveSession ? 'true' : 'false'}>
      {sessionId}
    </div>
  ),
}));

let navigateRef: NavigateFunction | undefined;

function RouterProbe() {
  const location = useLocation();
  navigateRef = useNavigate();
  return <div data-testid="router-location">{`${location.pathname}${location.search}`}</div>;
}

function TestHarness({
  activeSessions,
  initialEntry,
}: {
  activeSessions: Array<{ sessionId: string }>;
  initialEntry: string;
}) {
  return (
    <MemoryRouter initialEntries={[initialEntry]}>
      <Routes>
        <Route
          path="*"
          element={
            <>
              <RouterProbe />
              <ChatSessionsContainer setChat={vi.fn()} activeSessions={activeSessions} />
            </>
          }
        />
      </Routes>
    </MemoryRouter>
  );
}

describe('ChatSessionsContainer deleted-session guard', () => {
  beforeEach(() => {
    navigateRef = undefined;
  });

  it.each([
    [{ session: { id: 'reused-session' } }, 'session.id payload'],
    [{ sessionId: 'reused-session' }, 'sessionId payload'],
  ])(
    'allows session-id reuse after a delete when session-created clears tombstone state (%s)',
    async (createdDetail, _label) => {
      const initialEntry = '/pair?resumeSessionId=reused-session';
      const { rerender } = render(
        <TestHarness activeSessions={[{ sessionId: 'stable' }]} initialEntry={initialEntry} />
      );

      act(() => {
        window.dispatchEvent(
          new CustomEvent(AppEvents.SESSION_DELETED, {
            detail: { sessionId: 'reused-session' },
          })
        );
      });

      rerender(
        <TestHarness activeSessions={[{ sessionId: 'stable' }]} initialEntry={initialEntry} />
      );

      await waitFor(() => {
        expect(screen.getByTestId('router-location')).toHaveTextContent(
          '/pair?resumeSessionId=stable'
        );
      });

      act(() => {
        window.dispatchEvent(
          new CustomEvent(AppEvents.SESSION_CREATED, {
            detail: createdDetail,
          })
        );
      });

      act(() => {
        navigateRef?.('/pair?resumeSessionId=reused-session');
      });

      rerender(
        <TestHarness
          activeSessions={[{ sessionId: 'stable' }, { sessionId: 'reused-session' }]}
          initialEntry={initialEntry}
        />
      );

      await waitFor(() => {
        expect(screen.getByTestId('router-location')).toHaveTextContent(
          '/pair?resumeSessionId=reused-session'
        );
      });
      expect(screen.getByTestId('base-chat-reused-session')).toHaveAttribute('data-active', 'true');
    }
  );

  it('does not render deleted sessions when activeSessions still contains stale deleted id', async () => {
    const initialEntry = '/pair?resumeSessionId=reused-session';
    const { rerender } = render(
      <TestHarness activeSessions={[{ sessionId: 'stable' }]} initialEntry={initialEntry} />
    );

    act(() => {
      window.dispatchEvent(
        new CustomEvent(AppEvents.SESSION_DELETED, {
          detail: { sessionId: 'reused-session' },
        })
      );
    });

    rerender(
      <TestHarness activeSessions={[{ sessionId: 'stable' }]} initialEntry={initialEntry} />
    );

    await waitFor(() => {
      expect(screen.getByTestId('router-location')).toHaveTextContent(
        '/pair?resumeSessionId=stable'
      );
    });

    rerender(
      <TestHarness
        activeSessions={[{ sessionId: 'stable' }, { sessionId: 'reused-session' }]}
        initialEntry={initialEntry}
      />
    );

    await waitFor(() => {
      expect(screen.getByTestId('router-location')).toHaveTextContent(
        '/pair?resumeSessionId=stable'
      );
    });
    expect(screen.queryByTestId('base-chat-reused-session')).not.toBeInTheDocument();
  });

  it('does not navigate away when deleting a non-active session', async () => {
    const initialEntry = '/pair?resumeSessionId=active';
    const { rerender } = render(
      <TestHarness
        activeSessions={[{ sessionId: 'active' }, { sessionId: 'other' }]}
        initialEntry={initialEntry}
      />
    );

    act(() => {
      window.dispatchEvent(
        new CustomEvent(AppEvents.SESSION_DELETED, {
          detail: { sessionId: 'other' },
        })
      );
    });

    rerender(
      <TestHarness activeSessions={[{ sessionId: 'active' }]} initialEntry={initialEntry} />
    );

    await waitFor(() => {
      expect(screen.getByTestId('router-location')).toHaveTextContent(
        '/pair?resumeSessionId=active'
      );
    });
    expect(screen.getByTestId('base-chat-active')).toHaveAttribute('data-active', 'true');
  });
});
