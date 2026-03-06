import type { ReactElement } from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render as rtlRender, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { LocalizationProvider, useLocalization } from '../../contexts/LocalizationContext';
import SessionListView from './SessionListView';
import * as api from '../../api';

const render = (ui: ReactElement) => rtlRender(<LocalizationProvider>{ui}</LocalizationProvider>);

vi.mock('../../api');

function LanguageSwitchHarness() {
  const { setLanguage } = useLocalization();

  return (
    <>
      <button data-testid="switch-language" onClick={() => void setLanguage('zh-CN')}>
        Switch language
      </button>
      <SessionListView onSelectSession={() => undefined} />
    </>
  );
}

describe('SessionListView', () => {
  beforeEach(() => {
    vi.clearAllMocks();

    vi.mocked(api.listSessions).mockResolvedValue({
      data: {
        sessions: [
          {
            id: 'session-1',
            name: 'Project kickoff',
            message_count: 3,
            created_at: '2026-03-01T10:00:00.000Z',
            updated_at: '2026-03-01T10:00:00.000Z',
            working_dir: '/tmp/project',
            extension_data: { active: [], installed: [] },
            total_tokens: 123,
            user_set_name: true,
          },
        ],
      },
    } as unknown as Awaited<ReturnType<typeof api.listSessions>>);

    vi.mocked(api.searchSessions).mockResolvedValue({
      data: [],
    } as unknown as Awaited<ReturnType<typeof api.searchSessions>>);
  });

  it('does not reload sessions when the ui language changes', async () => {
    const user = userEvent.setup();

    render(<LanguageSwitchHarness />);

    await waitFor(() => {
      expect(screen.getByText('Project kickoff')).toBeInTheDocument();
    });

    expect(api.listSessions).toHaveBeenCalledTimes(1);

    await user.click(screen.getByTestId('switch-language'));

    await waitFor(() => {
      expect(document.documentElement.lang).toBe('zh-CN');
    });

    expect(api.listSessions).toHaveBeenCalledTimes(1);
    expect(screen.getByText('Project kickoff')).toBeInTheDocument();
  });
});
