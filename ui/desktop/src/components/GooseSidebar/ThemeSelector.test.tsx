import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import ThemeSelector from './ThemeSelector';
import { IntlTestWrapper } from '../../i18n/test-utils';
import type { CatppuccinAccent, ThemePreference } from '../../theme/types';

const setUserThemePreference = vi.fn();
const setCatppuccinAccent = vi.fn();

let preference: ThemePreference = 'light';
let accent: CatppuccinAccent = 'mauve';

vi.mock('../../contexts/ThemeContext', () => ({
  useTheme: () => ({
    userThemePreference: preference,
    setUserThemePreference,
    catppuccinAccent: accent,
    setCatppuccinAccent,
    resolvedThemeId: preference === 'system' ? 'light' : preference,
    resolvedTheme: 'light',
    mcpHostStyles: { variables: {} },
  }),
}));

function renderSelector() {
  return render(
    <IntlTestWrapper>
      <ThemeSelector hideTitle />
    </IntlTestWrapper>
  );
}

describe('ThemeSelector', () => {
  it('keeps built-in theme buttons', () => {
    preference = 'light';
    renderSelector();
    expect(screen.getByTestId('light-mode-button')).toBeInTheDocument();
    expect(screen.getByTestId('dark-mode-button')).toBeInTheDocument();
    expect(screen.getByTestId('aura-mode-button')).toBeInTheDocument();
    expect(screen.getByTestId('system-mode-button')).toBeInTheDocument();
  });

  it('shows Catppuccin flavor previews', () => {
    preference = 'light';
    renderSelector();
    expect(screen.getByTestId('catppuccin-latte-theme-button')).toBeInTheDocument();
    expect(screen.getByTestId('catppuccin-frappe-theme-button')).toBeInTheDocument();
    expect(screen.getByTestId('catppuccin-macchiato-theme-button')).toBeInTheDocument();
    expect(screen.getByTestId('catppuccin-mocha-theme-button')).toBeInTheDocument();
    expect(screen.queryByTestId('catppuccin-accent-mauve')).not.toBeInTheDocument();
  });

  it('selects a Catppuccin flavor', () => {
    preference = 'light';
    renderSelector();
    fireEvent.click(screen.getByTestId('catppuccin-mocha-theme-button'));
    expect(setUserThemePreference).toHaveBeenCalledWith('catppuccin-mocha');
  });

  it('shows accent swatches only for a Catppuccin theme', () => {
    preference = 'catppuccin-mocha';
    renderSelector();
    expect(screen.getByTestId('catppuccin-accent-mauve')).toBeInTheDocument();
    fireEvent.click(screen.getByTestId('catppuccin-accent-teal'));
    expect(setCatppuccinAccent).toHaveBeenCalledWith('teal');
  });
});
