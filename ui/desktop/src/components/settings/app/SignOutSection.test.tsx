import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, type RenderOptions } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { ReactElement } from 'react';
import SignOutSection from './SignOutSection';
import { IntlTestWrapper } from '../../../i18n/test-utils';

const acpDeleteProviderConfig = vi.fn();

vi.mock('../../../acp/providers', () => ({
  acpDeleteProviderConfig: (...args: unknown[]) => acpDeleteProviderConfig(...args),
}));

vi.mock('../../../toasts', () => ({
  toastError: vi.fn(),
}));

const renderWithIntl = (ui: ReactElement, options?: RenderOptions) =>
  render(ui, { wrapper: IntlTestWrapper, ...options });

describe('SignOutSection', () => {
  beforeEach(() => {
    acpDeleteProviderConfig.mockReset();
    acpDeleteProviderConfig.mockResolvedValue(undefined);
    vi.mocked(window.electron.reloadApp).mockClear();
  });

  it('deletes avocado config and reloads after confirm', async () => {
    const user = userEvent.setup();
    renderWithIntl(<SignOutSection />);

    await user.click(screen.getByTestId('avocado-sign-out'));
    await user.click(screen.getByTestId('avocado-sign-out-confirm'));

    await waitFor(() => {
      expect(acpDeleteProviderConfig).toHaveBeenCalledWith('avocado');
    });
    expect(window.electron.reloadApp).toHaveBeenCalled();
  });

  it('does not delete config when confirm is cancelled', async () => {
    const user = userEvent.setup();
    renderWithIntl(<SignOutSection />);

    await user.click(screen.getByTestId('avocado-sign-out'));
    await user.click(screen.getByRole('button', { name: 'Cancel' }));

    expect(acpDeleteProviderConfig).not.toHaveBeenCalled();
    expect(window.electron.reloadApp).not.toHaveBeenCalled();
  });
});
