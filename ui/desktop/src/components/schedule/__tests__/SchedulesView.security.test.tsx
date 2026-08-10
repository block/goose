import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router';
import type { ScheduledJobDto } from '@aaif/goose-sdk';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { acpDeleteSchedule, acpListSchedules } from '../../../acp/schedules';
import { IntlTestWrapper } from '../../../i18n/test-utils';
import SchedulesView from '../SchedulesView';

vi.mock('../../../acp/schedules', () => ({
  acpListSchedules: vi.fn(),
  acpCreateSchedule: vi.fn(),
  acpDeleteSchedule: vi.fn(),
  acpPauseSchedule: vi.fn(),
  acpUnpauseSchedule: vi.fn(),
  acpUpdateSchedule: vi.fn(),
  acpKillRunningJob: vi.fn(),
  acpInspectRunningJob: vi.fn(),
}));

vi.mock('../../Layout/MainPanelLayout', () => ({
  MainPanelLayout: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));

vi.mock('../ScheduleModal', () => ({
  ScheduleModal: () => null,
}));

vi.mock('../ScheduleDetailView', () => ({
  default: () => null,
}));

const schedule = (currentlyRunning: boolean): ScheduledJobDto => ({
  id: currentlyRunning ? 'running-job' : 'idle-job',
  source: 'recipe.yaml',
  cron: '0 0 * * * *',
  lastRun: null,
  currentlyRunning,
  paused: false,
});

function renderSchedules() {
  render(
    <MemoryRouter>
      <IntlTestWrapper>
        <SchedulesView />
      </IntlTestWrapper>
    </MemoryRouter>
  );
}

describe('SchedulesView running schedule controls', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.spyOn(window, 'confirm').mockReturnValue(true);
    vi.mocked(acpDeleteSchedule).mockResolvedValue(undefined);
  });

  it('does not offer deletion while the schedule is running', async () => {
    vi.mocked(acpListSchedules).mockResolvedValue([schedule(true)]);

    renderSchedules();

    await screen.findByText('running-job');
    expect(screen.getByRole('button', { name: 'Inspect' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Kill' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '' })).not.toBeInTheDocument();
  });

  it('keeps deletion available for an idle schedule', async () => {
    vi.mocked(acpListSchedules).mockResolvedValue([schedule(false)]);

    renderSchedules();

    await screen.findByText('idle-job');
    fireEvent.click(screen.getByRole('button', { name: '' }));

    await waitFor(() => expect(acpDeleteSchedule).toHaveBeenCalledWith('idle-job'));
  });
});
