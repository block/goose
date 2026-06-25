/**
 * @vitest-environment jsdom
 */
import { render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import SchedulesView from './SchedulesView';
import { IntlTestWrapper } from '../../i18n/test-utils';

vi.mock('react-router-dom', () => ({
  useLocation: () => ({ state: null }),
}));

vi.mock('../../acp/schedules', () => ({
  acpListSchedules: vi.fn().mockResolvedValue([]),
  acpCreateSchedule: vi.fn(),
  acpDeleteSchedule: vi.fn(),
  acpPauseSchedule: vi.fn(),
  acpUnpauseSchedule: vi.fn(),
  acpUpdateSchedule: vi.fn(),
  acpKillRunningJob: vi.fn(),
  acpInspectRunningJob: vi.fn(),
}));

vi.mock('../Layout/MainPanelLayout', () => ({
  MainPanelLayout: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));

vi.mock('../../toasts', () => ({
  toastError: vi.fn(),
  toastSuccess: vi.fn(),
}));

vi.mock('../../utils/analytics', () => ({
  getErrorType: vi.fn(),
  trackScheduleCreated: vi.fn(),
  trackScheduleDeleted: vi.fn(),
}));

describe('SchedulesView', () => {
  it('hides the create schedule button from ApeCloud builds', async () => {
    render(<SchedulesView />, { wrapper: IntlTestWrapper });

    await waitFor(() => {
      expect(screen.getByText('No schedules yet')).toBeInTheDocument();
    });

    expect(screen.getByRole('button', { name: 'Refresh' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Create Schedule' })).not.toBeInTheDocument();
  });
});
