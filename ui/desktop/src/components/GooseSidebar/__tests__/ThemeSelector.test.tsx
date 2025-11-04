import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
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
        removeItem: vi.fn((key: string) => {
          delete mockLocalStorage[key];
        }),
        clear: vi.fn(() => {
          mockLocalStorage = {};
        }),
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
        onchange: null,
        addListener: vi.fn(),
        removeListener: vi.fn(),
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        dispatchEvent: vi.fn(),
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
          contains: vi.fn(),
        },
      },
      writable: true,
      configurable: true,
    });

    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Rendering', () => {
    it('should render theme mode buttons', () => {
      render(<ThemeSelector />);

      expect(screen.getByTestId('light-mode-button')).toBeInTheDocument();
      expect(screen.getByTestId('dark-mode-button')).toBeInTheDocument();
      expect(screen.getByTestId('system-mode-button')).toBeInTheDocument();
    });

    it('should render custom accent section', () => {
      render(<ThemeSelector />);

      expect(screen.getByText('Custom Accent')).toBeInTheDocument();
      expect(screen.getByTestId('custom-color-toggle')).toBeInTheDocument();
    });

    it('should render theme title by default', () => {
      render(<ThemeSelector />);

      expect(screen.getByText('Theme')).toBeInTheDocument();
    });

    it('should hide title when hideTitle is true', () => {
      render(<ThemeSelector hideTitle />);

      expect(screen.queryByText('Theme')).not.toBeInTheDocument();
    });

    it('should not show color picker when custom color is disabled', () => {
      render(<ThemeSelector />);

      expect(screen.queryByTestId('custom-color-picker')).not.toBeInTheDocument();
    });
  });

  describe('Theme Mode Selection', () => {
    it('should select light mode by default', () => {
      render(<ThemeSelector />);

      const lightButton = screen.getByTestId('light-mode-button');
      expect(lightButton).toHaveClass('bg-background-accent');
    });

    it('should switch to dark mode when clicked', async () => {
      render(<ThemeSelector />);

      const darkButton = screen.getByTestId('dark-mode-button');
      await userEvent.click(darkButton);

      await waitFor(() => {
        expect(localStorage.setItem).toHaveBeenCalledWith('theme', 'dark');
        expect(localStorage.setItem).toHaveBeenCalledWith('use_system_theme', 'false');
      });
    });

    it('should switch to system mode when clicked', async () => {
      render(<ThemeSelector />);

      const systemButton = screen.getByTestId('system-mode-button');
      await userEvent.click(systemButton);

      await waitFor(() => {
        expect(localStorage.setItem).toHaveBeenCalledWith('use_system_theme', 'true');
      });
    });

    it('should apply dark class to documentElement in dark mode', async () => {
      render(<ThemeSelector />);

      const darkButton = screen.getByTestId('dark-mode-button');
      await userEvent.click(darkButton);

      await waitFor(() => {
        expect(document.documentElement.classList.add).toHaveBeenCalledWith('dark');
        expect(document.documentElement.classList.remove).toHaveBeenCalledWith('light');
      });
    });

    it('should broadcast theme change to electron', async () => {
      render(<ThemeSelector />);

      const darkButton = screen.getByTestId('dark-mode-button');
      await userEvent.click(darkButton);

      await waitFor(() => {
        expect(window.electron.broadcastThemeChange).toHaveBeenCalledWith({
          mode: 'dark',
          useSystemTheme: false,
          theme: 'dark',
        });
      });
    });
  });

  describe('Custom Color Toggle', () => {
    it('should show color picker when toggle is enabled', async () => {
      render(<ThemeSelector />);

      const toggle = screen.getByTestId('custom-color-toggle');
      await userEvent.click(toggle);

      await waitFor(() => {
        expect(screen.getByTestId('custom-color-picker')).toBeInTheDocument();
      });
    });

    it('should save custom color enabled state to localStorage', async () => {
      render(<ThemeSelector />);

      const toggle = screen.getByTestId('custom-color-toggle');
      await userEvent.click(toggle);

      await waitFor(() => {
        expect(localStorage.setItem).toHaveBeenCalledWith('custom_theme_enabled', 'true');
      });
    });

    it('should hide color picker when toggle is disabled', async () => {
      mockLocalStorage['custom_theme_enabled'] = 'true';
      render(<ThemeSelector />);

      expect(screen.getByTestId('custom-color-picker')).toBeInTheDocument();

      const toggle = screen.getByTestId('custom-color-toggle');
      await userEvent.click(toggle);

      await waitFor(() => {
        expect(screen.queryByTestId('custom-color-picker')).not.toBeInTheDocument();
      });
    });
  });

  describe('Custom Color Management', () => {
    it('should save custom color to localStorage', async () => {
      mockLocalStorage['custom_theme_enabled'] = 'true';
      render(<ThemeSelector />);

      const colorInput = screen.getByTestId('color-input');
      fireEvent.change(colorInput, { target: { value: '#ff0000' } });

      await waitFor(() => {
        expect(localStorage.setItem).toHaveBeenCalledWith('custom_theme_color', '#ff0000');
      });
    });

    it('should reset color to default', async () => {
      mockLocalStorage['custom_theme_enabled'] = 'true';
      mockLocalStorage['custom_theme_color'] = '#ff0000';
      render(<ThemeSelector />);

      const resetButton = screen.getByTestId('reset-button');
      await userEvent.click(resetButton);

      await waitFor(() => {
        expect(localStorage.setItem).toHaveBeenCalledWith('custom_theme_color', '#32353b');
      });
    });
  });

  describe('localStorage Integration', () => {
    it('should load theme mode from localStorage', () => {
      mockLocalStorage['theme'] = 'dark';
      render(<ThemeSelector />);

      const darkButton = screen.getByTestId('dark-mode-button');
      expect(darkButton).toHaveClass('bg-background-accent');
    });

    it('should load custom color from localStorage', () => {
      mockLocalStorage['custom_theme_enabled'] = 'true';
      mockLocalStorage['custom_theme_color'] = '#ff0000';
      render(<ThemeSelector />);

      const colorInput = screen.getByTestId('color-input') as HTMLInputElement;
      expect(colorInput.value).toBe('#ff0000');
    });

    it('should use default color when not in localStorage', () => {
      mockLocalStorage['custom_theme_enabled'] = 'true';
      render(<ThemeSelector />);

      const colorInput = screen.getByTestId('color-input') as HTMLInputElement;
      expect(colorInput.value).toBe('#32353b');
    });

    it('should validate custom color from storage events', async () => {
      mockLocalStorage['custom_theme_enabled'] = 'true';
      mockLocalStorage['custom_theme_color'] = '#ff0000';
      render(<ThemeSelector />);

      const colorInput = screen.getByTestId('color-input') as HTMLInputElement;
      expect(colorInput.value).toBe('#ff0000');

      // Simulate a storage event with an invalid color
      const invalidEvent = new Event('storage');
      Object.defineProperty(invalidEvent, 'key', { value: 'custom_theme_color' });
      Object.defineProperty(invalidEvent, 'newValue', { value: 'invalid-color' });
      window.dispatchEvent(invalidEvent);

      // Color should not change because validation should reject it
      await waitFor(() => {
        expect(colorInput.value).toBe('#ff0000');
      });

      // Now simulate a storage event with a valid color
      const validEvent = new Event('storage');
      Object.defineProperty(validEvent, 'key', { value: 'custom_theme_color' });
      Object.defineProperty(validEvent, 'newValue', { value: '#00ff00' });
      window.dispatchEvent(validEvent);

      // Color should update because validation should pass
      await waitFor(() => {
        expect(colorInput.value).toBe('#00ff00');
      });
    });
  });

  describe('Error Handling', () => {
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

  describe('System Theme Preference', () => {
    it('should respect system dark mode preference', () => {
      mockLocalStorage['use_system_theme'] = 'true';
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

      render(<ThemeSelector />);

      expect(document.documentElement.classList.add).toHaveBeenCalledWith('dark');
    });

    it('should listen for system theme changes', () => {
      mockLocalStorage['use_system_theme'] = 'true';
      const addEventListenerMock = vi.fn();

      Object.defineProperty(window, 'matchMedia', {
        value: vi.fn().mockImplementation(() => ({
          matches: false,
          addEventListener: addEventListenerMock,
          removeEventListener: vi.fn(),
        })),
        writable: true,
        configurable: true,
      });

      render(<ThemeSelector />);

      expect(addEventListenerMock).toHaveBeenCalledWith('change', expect.any(Function));
    });
  });

  describe('Accessibility', () => {
    it('should have proper test IDs for theme buttons', () => {
      render(<ThemeSelector />);

      expect(screen.getByTestId('light-mode-button')).toBeInTheDocument();
      expect(screen.getByTestId('dark-mode-button')).toBeInTheDocument();
      expect(screen.getByTestId('system-mode-button')).toBeInTheDocument();
      expect(screen.getByTestId('custom-color-toggle')).toBeInTheDocument();
    });
  });

  describe('Custom className', () => {
    it('should apply custom className', () => {
      const { container } = render(<ThemeSelector className="custom-class" />);

      const wrapper = container.firstChild as HTMLElement;
      expect(wrapper).toHaveClass('custom-class');
    });
  });
});
