import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ThemeSelector } from '../ThemeSelector';

// Mock the CustomColorPicker component
vi.mock('../CustomColorPicker', () => ({
  CustomColorPicker: ({
    value,
    onChange,
    onReset,
  }: {
    value: string;
    onChange: (color: string) => void;
    onReset: () => void;
  }) => (
    <div data-testid="custom-color-picker">
      <input data-testid="color-input" value={value} onChange={(e) => onChange(e.target.value)} />
      <button data-testid="reset-button" onClick={onReset}>
        Reset
      </button>
    </div>
  ),
}));

// Mock colorUtils
vi.mock('../../../utils/colorUtils', () => ({
  applyCustomTheme: vi.fn(),
  resetThemeColors: vi.fn(),
  DEFAULT_THEME_COLOR: '#32353b',
  isValidHexColor: vi.fn((color: string) => /^#[0-9A-Fa-f]{6}$/.test(color)),
}));

// Mock lucide-react icons
vi.mock('lucide-react', () => ({
  Moon: () => <span>🌙</span>,
  Sun: () => <span>☀️</span>,
  Sliders: () => <span>⚙️</span>,
  Palette: () => <span>🎨</span>,
}));

describe('ThemeSelector', () => {
  let mockLocalStorage: Record<string, string>;

  beforeEach(() => {
    // Mock localStorage
    mockLocalStorage = {};
    Object.defineProperty(window, 'localStorage', {
      value: {
        getItem: vi.fn((key: string) => mockLocalStorage[key] || null),
        setItem: vi.fn((key: string, value: string) => {
          mockLocalStorage[key] = value;
        }),
        removeItem: vi.fn(),
        clear: vi.fn(),
        length: 0,
        key: vi.fn(),
      },
      writable: true,
      configurable: true,
    });

    // Mock window.matchMedia
    Object.defineProperty(window, 'matchMedia', {
      value: vi.fn().mockImplementation((query) => ({
        matches: query === '(prefers-color-scheme: dark)',
        media: query,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
      })),
      writable: true,
      configurable: true,
    });

    // Mock window.electron
    Object.defineProperty(window, 'electron', {
      value: {
        broadcastThemeChange: vi.fn(),
      },
      writable: true,
      configurable: true,
    });

    // Mock document.documentElement
    Object.defineProperty(document, 'documentElement', {
      value: {
        classList: {
          add: vi.fn(),
          remove: vi.fn(),
        },
        style: {
          setProperty: vi.fn(),
          removeProperty: vi.fn(),
        },
      },
      writable: true,
      configurable: true,
    });
  });

  it('should render theme mode buttons', () => {
    render(<ThemeSelector />);

    expect(screen.getByTestId('light-mode-button')).toBeInTheDocument();
    expect(screen.getByTestId('dark-mode-button')).toBeInTheDocument();
    expect(screen.getByTestId('system-mode-button')).toBeInTheDocument();
  });

  it('should render custom accent color toggle', () => {
    render(<ThemeSelector />);

    expect(screen.getByText('Custom Accent Color')).toBeInTheDocument();
    expect(screen.getByTestId('custom-color-toggle')).toBeInTheDocument();
  });

  it('should switch between light and dark modes', async () => {
    render(<ThemeSelector />);

    const darkButton = screen.getByTestId('dark-mode-button');
    await userEvent.click(darkButton);

    await waitFor(() => {
      expect(mockLocalStorage['theme']).toBe('dark');
    });
  });

  it('should show color picker when custom color is enabled', async () => {
    render(<ThemeSelector />);

    const toggle = screen.getByTestId('custom-color-toggle');
    await userEvent.click(toggle);

    await waitFor(() => {
      expect(screen.getByTestId('custom-color-picker')).toBeInTheDocument();
    });
  });

  it('should hide color picker when custom color is disabled', async () => {
    mockLocalStorage['custom_theme_enabled'] = 'true';
    render(<ThemeSelector />);

    expect(screen.getByTestId('custom-color-picker')).toBeInTheDocument();

    const toggle = screen.getByTestId('custom-color-toggle');
    await userEvent.click(toggle);

    await waitFor(() => {
      expect(screen.queryByTestId('custom-color-picker')).not.toBeInTheDocument();
    });
  });

  it('should persist custom color to localStorage', async () => {
    mockLocalStorage['custom_theme_enabled'] = 'true';
    render(<ThemeSelector />);

    const colorInput = screen.getByTestId('color-input');
    await userEvent.clear(colorInput);
    await userEvent.type(colorInput, '#ff0000');

    await waitFor(() => {
      expect(mockLocalStorage['custom_theme_color']).toBe('#ff0000');
    });
  });

  it('should handle localStorage errors gracefully', () => {
    Object.defineProperty(window, 'localStorage', {
      value: {
        getItem: vi.fn(() => {
          throw new Error('localStorage error');
        }),
        setItem: vi.fn(),
        removeItem: vi.fn(),
        clear: vi.fn(),
        length: 0,
        key: vi.fn(),
      },
      writable: true,
      configurable: true,
    });

    // Should not throw even if localStorage fails
    expect(() => render(<ThemeSelector />)).not.toThrow();
  });
});
