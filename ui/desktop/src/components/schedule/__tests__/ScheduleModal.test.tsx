import { describe, it, expect, vi } from 'vitest';
import { render, type RenderOptions, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { ScheduledJobDto } from '@aaif/goose-sdk';
import { ScheduleModal } from '../ScheduleModal';
import { IntlTestWrapper } from '../../../i18n/test-utils';

const renderWithIntl = (ui: React.ReactElement, options?: RenderOptions) =>
  render(ui, { wrapper: IntlTestWrapper, ...options });

const existingSchedule = {
  id: 'daily-summary-job',
  cron: '0 0 14 * * *',
} as ScheduledJobDto;

const baseProps = {
  onClose: vi.fn(),
  onSubmit: vi.fn().mockResolvedValue(undefined),
  isLoadingExternally: false,
  apiErrorExternally: null,
  initialDeepLink: null,
};

describe('ScheduleModal', () => {
  it('preserves the form when the recipe picker is cancelled', async () => {
    const user = userEvent.setup();
    const selectRecipeFile = vi.fn().mockResolvedValue(null);
    window.electron.selectRecipeFile = selectRecipeFile;
    renderWithIntl(<ScheduleModal {...baseProps} isOpen schedule={null} />);

    await user.click(screen.getByRole('button', { name: 'Browse for YAML file...' }));

    expect(selectRecipeFile).toHaveBeenCalledOnce();
    expect(screen.queryByText(/Failed to read|Invalid file type/)).not.toBeInTheDocument();
  });

  it('clears a validation error from create mode when reopened to edit a schedule', async () => {
    const user = userEvent.setup();
    const { rerender } = renderWithIntl(<ScheduleModal {...baseProps} isOpen schedule={null} />);

    await user.type(screen.getByLabelText(/name/i), 'my-job');
    await user.click(screen.getByRole('button', { name: 'Create Schedule' }));
    await waitFor(() => {
      expect(screen.getByText('Please provide a valid recipe source.')).toBeInTheDocument();
    });

    rerender(<ScheduleModal {...baseProps} isOpen={false} schedule={null} />);
    rerender(<ScheduleModal {...baseProps} isOpen schedule={existingSchedule} />);

    expect(screen.getByText('Edit Schedule')).toBeInTheDocument();
    expect(screen.queryByText('Please provide a valid recipe source.')).not.toBeInTheDocument();
  });
});
