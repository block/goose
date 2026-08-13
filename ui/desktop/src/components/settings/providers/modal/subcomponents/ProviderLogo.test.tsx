import { describe, expect, it } from 'vitest';
import { render, screen, type RenderOptions } from '@testing-library/react';
import type { ReactElement } from 'react';
import ProviderLogo from './ProviderLogo';
import { IntlTestWrapper } from '../../../../../i18n/test-utils';

const renderWithIntl = (ui: ReactElement, options?: RenderOptions) =>
  render(ui, { wrapper: IntlTestWrapper, ...options });

describe('ProviderLogo', () => {
  it('renders the Avocado brand mark for the avocado provider', () => {
    const { container } = renderWithIntl(<ProviderLogo providerName="avocado" />);

    expect(screen.getByRole('img', { name: 'Avocado' })).toBeInTheDocument();
    expect(container.querySelector('img')).toBeNull();
  });

  it('matches the avocado provider case-insensitively', () => {
    renderWithIntl(<ProviderLogo providerName="Avocado" />);

    expect(screen.getByRole('img', { name: 'Avocado' })).toBeInTheDocument();
  });

  it('renders an image logo for third-party providers', () => {
    renderWithIntl(<ProviderLogo providerName="openai" />);

    expect(screen.getByRole('img', { name: 'openai logo' })).toBeInTheDocument();
  });
});
