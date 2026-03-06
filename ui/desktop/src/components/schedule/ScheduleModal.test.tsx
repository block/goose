import type { ReactElement } from 'react';
import { render as rtlRender, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { LocalizationProvider, useLocalization } from '../../contexts/LocalizationContext';
import { ScheduleModal } from './ScheduleModal';

function render(ui: ReactElement) {
  return rtlRender(<LocalizationProvider>{ui}</LocalizationProvider>);
}

function LanguageSwitchHarness() {
  const { setLanguage } = useLocalization();

  return (
    <>
      <button data-testid="switch-language" onClick={() => void setLanguage('zh-CN')}>
        Switch language
      </button>
      <ScheduleModal
        isOpen
        onClose={() => undefined}
        onSubmit={async () => undefined}
        schedule={null}
        isLoadingExternally={false}
        apiErrorExternally={null}
        initialDeepLink={null}
      />
    </>
  );
}

describe('ScheduleModal', () => {
  it('preserves in-progress form state when the ui language changes', async () => {
    const user = userEvent.setup();
    const { container } = render(<LanguageSwitchHarness />);

    const scheduleIdInput = screen.getByLabelText('Name:');
    await user.type(scheduleIdInput, 'my-schedule');
    await user.click(screen.getByRole('button', { name: 'Deep link' }));

    expect(
      container.querySelector('input[placeholder*="goose://recipe"]')
    ).toBeInTheDocument();

    await user.click(screen.getByTestId('switch-language'));

    await waitFor(() => {
      expect(document.documentElement.lang).toBe('zh-CN');
    });

    expect(scheduleIdInput).toHaveValue('my-schedule');
    expect(
      container.querySelector('input[placeholder*="goose://recipe"]')
    ).toBeInTheDocument();
  });
});
