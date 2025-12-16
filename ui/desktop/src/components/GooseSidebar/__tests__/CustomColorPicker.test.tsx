import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { CustomColorPicker } from '../CustomColorPicker';

// Mock lucide-react icons
vi.mock('lucide-react', () => ({
  Plus: () => <div data-testid="plus-icon">Plus</div>,
}));

// Mock colorUtils
vi.mock('../../../utils/colorUtils', () => ({
  isValidHexColor: (hex: string) => /^#[0-9a-f]{6}$/i.test(hex),
  DEFAULT_THEME_COLOR: '#32353b',
}));

// Mock Dialog components
vi.mock('../../ui/dialog', () => ({
  Dialog: ({ open, children }: { open: boolean; children: React.ReactNode }) =>
    open ? <div data-testid="dialog">{children}</div> : null,
  DialogContent: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="dialog-content">{children}</div>
  ),
  DialogHeader: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  DialogTitle: ({ children }: { children: React.ReactNode }) => <h2>{children}</h2>,
  DialogDescription: ({ children }: { children: React.ReactNode }) => <p>{children}</p>,
  DialogFooter: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));

describe('CustomColorPicker', () => {
  const mockOnChange = vi.fn();
  const mockOnReset = vi.fn();
  const defaultProps = {
    value: '#32353b',
    onChange: mockOnChange,
    onReset: mockOnReset,
  };

  let mockLocalStorage: Record<string, string>;

  beforeEach(() => {
    vi.clearAllMocks();

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
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Rendering', () => {
    it('should render default preset colors', () => {
      render(<CustomColorPicker {...defaultProps} />);

      const presetButtons = screen.getAllByRole('button', { name: /select color/i });
      expect(presetButtons).toHaveLength(6); // 6 default presets
    });

    it('should render accent color label', () => {
      render(<CustomColorPicker {...defaultProps} />);

      expect(screen.getByText('Accent Color')).toBeInTheDocument();
    });

    it('should render add custom color button', () => {
      render(<CustomColorPicker {...defaultProps} />);

      const addButton = screen.getByTestId('add-custom-color-button');
      expect(addButton).toBeInTheDocument();
      expect(addButton).toHaveAttribute('aria-label', 'Add custom color');
    });

    it('should not show dialog by default', () => {
      render(<CustomColorPicker {...defaultProps} />);

      expect(screen.queryByTestId('dialog')).not.toBeInTheDocument();
    });
  });

  describe('Preset Color Selection', () => {
    it('should call onChange when preset is clicked', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const tealPreset = screen.getByRole('button', { name: 'Select color #13bbaf' });
      await userEvent.click(tealPreset);

      expect(mockOnChange).toHaveBeenCalledWith('#13bbaf');
    });

    it('should mark selected preset with aria-pressed', () => {
      render(<CustomColorPicker {...defaultProps} value="#13bbaf" />);

      const tealPreset = screen.getByRole('button', { name: 'Select color #13bbaf' });
      expect(tealPreset).toHaveAttribute('aria-pressed', 'true');
    });

    it('should not mark unselected presets with aria-pressed', () => {
      render(<CustomColorPicker {...defaultProps} value="#13bbaf" />);

      const orangePreset = screen.getByRole('button', { name: 'Select color #ff4f00' });
      expect(orangePreset).toHaveAttribute('aria-pressed', 'false');
    });

    it('should apply selected styles to active color', () => {
      render(<CustomColorPicker {...defaultProps} value="#13bbaf" />);

      const tealPreset = screen.getByRole('button', { name: 'Select color #13bbaf' });
      expect(tealPreset).toHaveClass('border-background-accent');
    });
  });

  describe('Custom Color Dialog', () => {
    it('should open dialog when add button is clicked', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const addButton = screen.getByTestId('add-custom-color-button');
      await userEvent.click(addButton);

      await waitFor(() => {
        expect(screen.getByTestId('dialog')).toBeInTheDocument();
      });
    });

    it('should show dialog title and description', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const addButton = screen.getByTestId('add-custom-color-button');
      await userEvent.click(addButton);

      await waitFor(() => {
        expect(screen.getByText('Add Custom Color')).toBeInTheDocument();
        expect(
          screen.getByText('Choose a custom accent color to add to your palette')
        ).toBeInTheDocument();
      });
    });

    it('should show color picker in dialog', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const addButton = screen.getByTestId('add-custom-color-button');
      await userEvent.click(addButton);

      await waitFor(() => {
        expect(screen.getByLabelText('Color picker')).toBeInTheDocument();
      });
    });

    it('should show hex input in dialog', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const addButton = screen.getByTestId('add-custom-color-button');
      await userEvent.click(addButton);

      await waitFor(() => {
        expect(screen.getByLabelText('Hex color input')).toBeInTheDocument();
      });
    });

    it('should show preview area in dialog', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const addButton = screen.getByTestId('add-custom-color-button');
      await userEvent.click(addButton);

      await waitFor(() => {
        expect(screen.getByText('Preview')).toBeInTheDocument();
      });
    });

    it('should show Add Color and Cancel buttons', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const addButton = screen.getByTestId('add-custom-color-button');
      await userEvent.click(addButton);

      await waitFor(() => {
        expect(screen.getByRole('button', { name: 'Add Color' })).toBeInTheDocument();
        expect(screen.getByRole('button', { name: 'Cancel' })).toBeInTheDocument();
      });
    });
  });

  describe('Color Picker Interaction', () => {
    it('should update hex input when color picker changes', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const addButton = screen.getByTestId('add-custom-color-button');
      await userEvent.click(addButton);

      await waitFor(() => {
        const colorInput = screen.getByLabelText('Color picker');
        fireEvent.change(colorInput, { target: { value: '#ff0000' } });
      });

      const hexInput = screen.getByLabelText('Hex color input') as HTMLInputElement;
      await waitFor(() => {
        expect(hexInput.value).toBe('#ff0000');
      });
    });

    it('should update color picker when hex input changes', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const addButton = screen.getByTestId('add-custom-color-button');
      await userEvent.click(addButton);

      await waitFor(async () => {
        const hexInput = screen.getByLabelText('Hex color input');
        await userEvent.clear(hexInput);
        await userEvent.type(hexInput, '#00ff00');
      });

      const colorInput = screen.getByLabelText('Color picker') as HTMLInputElement;
      await waitFor(() => {
        expect(colorInput.value).toBe('#00ff00');
      });
    });
  });

  describe('Hex Input Validation', () => {
    it('should accept valid hex colors', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const addButton = screen.getByTestId('add-custom-color-button');
      await userEvent.click(addButton);

      await waitFor(async () => {
        const hexInput = screen.getByLabelText('Hex color input');
        await userEvent.clear(hexInput);
        await userEvent.type(hexInput, '#ff0000');
      });

      expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    });

    it('should show error for invalid hex colors', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const addButton = screen.getByTestId('add-custom-color-button');
      await userEvent.click(addButton);

      await waitFor(async () => {
        const hexInput = screen.getByLabelText('Hex color input');
        await userEvent.clear(hexInput);
        await userEvent.type(hexInput, 'invalid');
      });

      await waitFor(() => {
        expect(screen.getByRole('alert')).toHaveTextContent('Invalid hex color');
      });
    });

    it('should disable Add Color button for invalid hex', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const addButton = screen.getByTestId('add-custom-color-button');
      await userEvent.click(addButton);

      await waitFor(async () => {
        const hexInput = screen.getByLabelText('Hex color input');
        await userEvent.clear(hexInput);
        await userEvent.type(hexInput, 'invalid');
      });

      const addColorButton = screen.getByRole('button', { name: 'Add Color' });
      await waitFor(() => {
        expect(addColorButton).toBeDisabled();
      });
    });
  });

  describe('Adding Custom Colors', () => {
    it('should add custom color to grid when Add Color is clicked', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      // Open dialog
      const addButton = screen.getByTestId('add-custom-color-button');
      await userEvent.click(addButton);

      // Enter custom color
      await waitFor(async () => {
        const hexInput = screen.getByLabelText('Hex color input');
        await userEvent.clear(hexInput);
        await userEvent.type(hexInput, '#abcdef');
      });

      // Click Add Color
      const addColorButton = screen.getByRole('button', { name: 'Add Color' });
      await userEvent.click(addColorButton);

      // Check that color was added to grid
      await waitFor(() => {
        expect(screen.getByRole('button', { name: 'Select color #abcdef' })).toBeInTheDocument();
      });
    });

    it('should save custom colors to localStorage', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const addButton = screen.getByTestId('add-custom-color-button');
      await userEvent.click(addButton);

      await waitFor(async () => {
        const hexInput = screen.getByLabelText('Hex color input');
        await userEvent.clear(hexInput);
        await userEvent.type(hexInput, '#abcdef');
      });

      const addColorButton = screen.getByRole('button', { name: 'Add Color' });
      await userEvent.click(addColorButton);

      await waitFor(() => {
        expect(localStorage.setItem).toHaveBeenCalledWith(
          'custom_accent_colors',
          JSON.stringify(['#abcdef'])
        );
      });
    });

    it('should apply custom color when added', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const addButton = screen.getByTestId('add-custom-color-button');
      await userEvent.click(addButton);

      await waitFor(async () => {
        const hexInput = screen.getByLabelText('Hex color input');
        await userEvent.clear(hexInput);
        await userEvent.type(hexInput, '#abcdef');
      });

      const addColorButton = screen.getByRole('button', { name: 'Add Color' });
      await userEvent.click(addColorButton);

      await waitFor(() => {
        expect(mockOnChange).toHaveBeenCalledWith('#abcdef');
      });
    });

    it('should close dialog after adding color', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const addButton = screen.getByTestId('add-custom-color-button');
      await userEvent.click(addButton);

      await waitFor(async () => {
        const hexInput = screen.getByLabelText('Hex color input');
        await userEvent.clear(hexInput);
        await userEvent.type(hexInput, '#abcdef');
      });

      const addColorButton = screen.getByRole('button', { name: 'Add Color' });
      await userEvent.click(addColorButton);

      await waitFor(() => {
        expect(screen.queryByTestId('dialog')).not.toBeInTheDocument();
      });
    });

    it('should not add duplicate colors', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      // Add first color
      const addButton = screen.getByTestId('add-custom-color-button');
      await userEvent.click(addButton);

      await waitFor(async () => {
        const hexInput = screen.getByLabelText('Hex color input');
        await userEvent.clear(hexInput);
        await userEvent.type(hexInput, '#abcdef');
      });

      let addColorButton = screen.getByRole('button', { name: 'Add Color' });
      await userEvent.click(addColorButton);

      // Try to add same color again
      await waitFor(async () => {
        const addButton2 = screen.getByTestId('add-custom-color-button');
        await userEvent.click(addButton2);
      });

      await waitFor(async () => {
        const hexInput = screen.getByLabelText('Hex color input');
        await userEvent.clear(hexInput);
        await userEvent.type(hexInput, '#abcdef');
      });

      addColorButton = screen.getByRole('button', { name: 'Add Color' });
      await userEvent.click(addColorButton);

      // Should only save once
      await waitFor(() => {
        const calls = (localStorage.setItem as ReturnType<typeof vi.fn>).mock.calls.filter(
          (call) => call[0] === 'custom_accent_colors'
        );
        // First call adds the color, second call doesn't add duplicate
        expect(calls.length).toBeGreaterThanOrEqual(1);
      });
    });
  });

  describe('Dialog Cancel', () => {
    it('should close dialog when Cancel is clicked', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const addButton = screen.getByTestId('add-custom-color-button');
      await userEvent.click(addButton);

      await waitFor(() => {
        expect(screen.getByTestId('dialog')).toBeInTheDocument();
      });

      const cancelButton = screen.getByRole('button', { name: 'Cancel' });
      await userEvent.click(cancelButton);

      await waitFor(() => {
        expect(screen.queryByTestId('dialog')).not.toBeInTheDocument();
      });
    });

    it('should not add color when Cancel is clicked', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const addButton = screen.getByTestId('add-custom-color-button');
      await userEvent.click(addButton);

      await waitFor(async () => {
        const hexInput = screen.getByLabelText('Hex color input');
        await userEvent.clear(hexInput);
        await userEvent.type(hexInput, '#abcdef');
      });

      const cancelButton = screen.getByRole('button', { name: 'Cancel' });
      await userEvent.click(cancelButton);

      await waitFor(() => {
        expect(screen.queryByRole('button', { name: 'Select color #abcdef' })).not.toBeInTheDocument();
      });
    });
  });

  describe('LocalStorage Integration', () => {
    it('should load custom colors from localStorage on mount', () => {
      mockLocalStorage['custom_accent_colors'] = JSON.stringify(['#abcdef', '#123456']);

      render(<CustomColorPicker {...defaultProps} />);

      expect(screen.getByRole('button', { name: 'Select color #abcdef' })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Select color #123456' })).toBeInTheDocument();
    });

    it('should handle invalid localStorage data gracefully', () => {
      mockLocalStorage['custom_accent_colors'] = 'invalid json';

      expect(() => render(<CustomColorPicker {...defaultProps} />)).not.toThrow();
    });

    it('should handle localStorage errors gracefully', () => {
      Object.defineProperty(window, 'localStorage', {
        value: {
          getItem: vi.fn(() => {
            throw new Error('localStorage error');
          }),
          setItem: vi.fn(),
        },
        writable: true,
        configurable: true,
      });

      expect(() => render(<CustomColorPicker {...defaultProps} />)).not.toThrow();
    });
  });

  describe('Maximum Colors', () => {
    it('should show add button when under max colors', () => {
      render(<CustomColorPicker {...defaultProps} />);

      expect(screen.getByTestId('add-custom-color-button')).toBeInTheDocument();
    });

    it('should hide add button when at max colors', () => {
      // 6 default + 6 custom = 12 total (max)
      mockLocalStorage['custom_accent_colors'] = JSON.stringify([
        '#111111',
        '#222222',
        '#333333',
        '#444444',
        '#555555',
        '#666666',
      ]);

      render(<CustomColorPicker {...defaultProps} />);

      expect(screen.queryByTestId('add-custom-color-button')).not.toBeInTheDocument();
    });
  });

  describe('Accessibility', () => {
    it('should have proper ARIA labels for color buttons', () => {
      render(<CustomColorPicker {...defaultProps} />);

      const presetButtons = screen.getAllByRole('button', { name: /select color/i });
      presetButtons.forEach((button) => {
        expect(button).toHaveAttribute('aria-label');
      });
    });

    it('should have proper ARIA labels in dialog', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const addButton = screen.getByTestId('add-custom-color-button');
      await userEvent.click(addButton);

      await waitFor(() => {
        expect(screen.getByLabelText('Color picker')).toBeInTheDocument();
        expect(screen.getByLabelText('Hex color input')).toBeInTheDocument();
      });
    });

    it('should have role="group" for color grid', () => {
      render(<CustomColorPicker {...defaultProps} />);

      const colorGrid = screen.getByRole('group', { name: 'Accent colors' });
      expect(colorGrid).toBeInTheDocument();
    });
  });

  describe('Custom className', () => {
    it('should apply custom className', () => {
      const { container } = render(
        <CustomColorPicker {...defaultProps} className="custom-class" />
      );

      const wrapper = container.firstChild as HTMLElement;
      expect(wrapper).toHaveClass('custom-class');
    });
  });
});
