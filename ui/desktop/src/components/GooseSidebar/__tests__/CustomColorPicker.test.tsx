import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { CustomColorPicker } from '../CustomColorPicker';

// Mock lucide-react icons
vi.mock('lucide-react', () => ({
  RotateCcw: () => <div data-testid="reset-icon">Reset</div>,
  Plus: () => <div data-testid="plus-icon">Plus</div>,
}));

// Mock colorUtils
vi.mock('../../../utils/colorUtils', () => ({
  isValidHexColor: (hex: string) => /^#[0-9a-f]{6}$/i.test(hex),
  DEFAULT_THEME_COLOR: '#32353b',
}));

describe('CustomColorPicker', () => {
  const mockOnChange = vi.fn();
  const mockOnReset = vi.fn();
  const defaultProps = {
    value: '#32353b',
    onChange: mockOnChange,
    onReset: mockOnReset,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Rendering', () => {
    it('should render preset colors', () => {
      render(<CustomColorPicker {...defaultProps} />);

      const presetButtons = screen.getAllByRole('button', { name: /select color/i });
      expect(presetButtons).toHaveLength(6); // 6 preset colors
    });

    it('should render accent color label', () => {
      render(<CustomColorPicker {...defaultProps} />);

      expect(screen.getByText('Accent Color')).toBeInTheDocument();
    });

    it('should render custom color button', () => {
      render(<CustomColorPicker {...defaultProps} />);

      const customButton = screen.getByTestId('show-custom-picker-button');
      expect(customButton).toBeInTheDocument();
      expect(customButton).toHaveTextContent('Custom Color');
    });

    it('should not show custom picker by default', () => {
      render(<CustomColorPicker {...defaultProps} />);

      expect(screen.queryByTestId('custom-picker-expanded')).not.toBeInTheDocument();
      expect(screen.queryByLabelText('Color picker')).not.toBeInTheDocument();
    });
  });

  describe('Custom Picker Expansion', () => {
    it('should show custom picker when button is clicked', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const customButton = screen.getByTestId('show-custom-picker-button');
      await userEvent.click(customButton);

      await waitFor(() => {
        expect(screen.getByTestId('custom-picker-expanded')).toBeInTheDocument();
        expect(screen.getByLabelText('Color picker')).toBeInTheDocument();
        expect(screen.getByLabelText('Hex color input')).toBeInTheDocument();
      });
    });

    it('should hide custom color button when picker is expanded', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const customButton = screen.getByTestId('show-custom-picker-button');
      await userEvent.click(customButton);

      await waitFor(() => {
        expect(screen.queryByTestId('show-custom-picker-button')).not.toBeInTheDocument();
      });
    });

    it('should show hide button when picker is expanded', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const customButton = screen.getByTestId('show-custom-picker-button');
      await userEvent.click(customButton);

      await waitFor(() => {
        expect(screen.getByTestId('hide-custom-picker-button')).toBeInTheDocument();
      });
    });

    it('should hide custom picker when hide button is clicked', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      // Expand picker
      const customButton = screen.getByTestId('show-custom-picker-button');
      await userEvent.click(customButton);

      await waitFor(() => {
        expect(screen.getByTestId('custom-picker-expanded')).toBeInTheDocument();
      });

      // Hide picker
      const hideButton = screen.getByTestId('hide-custom-picker-button');
      await userEvent.click(hideButton);

      await waitFor(() => {
        expect(screen.queryByTestId('custom-picker-expanded')).not.toBeInTheDocument();
        expect(screen.getByTestId('show-custom-picker-button')).toBeInTheDocument();
      });
    });
  });

  describe('Color Picker Interaction', () => {
    beforeEach(async () => {
      // Expand the custom picker for these tests
      render(<CustomColorPicker {...defaultProps} />);
      const customButton = screen.getByTestId('show-custom-picker-button');
      await userEvent.click(customButton);
    });

    it('should call onChange when color picker value changes', async () => {
      const colorInput = screen.getByLabelText('Color picker');
      fireEvent.change(colorInput, { target: { value: '#ff0000' } });

      await waitFor(() => {
        expect(mockOnChange).toHaveBeenCalledWith('#ff0000');
      });
    });

    it('should update input value when color picker changes', async () => {
      const colorInput = screen.getByLabelText('Color picker') as HTMLInputElement;
      fireEvent.change(colorInput, { target: { value: '#ff0000' } });

      const hexInput = screen.getByLabelText('Hex color input') as HTMLInputElement;
      await waitFor(() => {
        expect(hexInput.value).toBe('#ff0000');
      });
    });
  });

  describe('Hex Input Validation', () => {
    beforeEach(async () => {
      // Expand the custom picker for these tests
      render(<CustomColorPicker {...defaultProps} />);
      const customButton = screen.getByTestId('show-custom-picker-button');
      await userEvent.click(customButton);
    });

    it('should accept valid hex colors', async () => {
      const hexInput = screen.getByLabelText('Hex color input');
      await userEvent.clear(hexInput);
      await userEvent.type(hexInput, '#ff0000');

      expect(mockOnChange).toHaveBeenCalledWith('#ff0000');
      expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    });

    it('should show error for invalid hex colors', async () => {
      const hexInput = screen.getByLabelText('Hex color input');
      await userEvent.clear(hexInput);
      await userEvent.type(hexInput, 'invalid');

      await waitFor(() => {
        expect(screen.getByRole('alert')).toHaveTextContent('Invalid hex color');
      });
    });

    it('should not call onChange for invalid hex', async () => {
      const hexInput = screen.getByLabelText('Hex color input');
      await userEvent.clear(hexInput);
      await userEvent.type(hexInput, 'invalid');

      // Should not have been called with invalid value
      expect(mockOnChange).not.toHaveBeenCalledWith('invalid');
    });

    it('should mark input as invalid with aria-invalid', async () => {
      const hexInput = screen.getByLabelText('Hex color input');
      await userEvent.clear(hexInput);
      await userEvent.type(hexInput, 'invalid');

      await waitFor(() => {
        expect(hexInput).toHaveAttribute('aria-invalid', 'true');
      });
    });

    it('should link error message with aria-describedby', async () => {
      const hexInput = screen.getByLabelText('Hex color input');
      await userEvent.clear(hexInput);
      await userEvent.type(hexInput, 'invalid');

      await waitFor(() => {
        expect(hexInput).toHaveAttribute('aria-describedby', 'color-error');
        expect(screen.getByRole('alert')).toHaveAttribute('id', 'color-error');
      });
    });
  });

  describe('Preset Colors', () => {
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
  });

  describe('Custom Color Indicator', () => {
    it('should show custom color indicator when color is not a preset', () => {
      render(<CustomColorPicker {...defaultProps} value="#abcdef" />);

      expect(screen.getByText('Custom: #abcdef')).toBeInTheDocument();
    });

    it('should not show custom color indicator when color is a preset', () => {
      render(<CustomColorPicker {...defaultProps} value="#13bbaf" />);

      expect(screen.queryByText(/Custom:/)).not.toBeInTheDocument();
    });

    it('should not show custom color indicator when picker is expanded', async () => {
      render(<CustomColorPicker {...defaultProps} value="#abcdef" />);

      // Should show indicator initially
      expect(screen.getByText('Custom: #abcdef')).toBeInTheDocument();

      // Expand picker
      const customButton = screen.getByTestId('show-custom-picker-button');
      await userEvent.click(customButton);

      // Indicator should be hidden
      await waitFor(() => {
        expect(screen.queryByText('Custom: #abcdef')).not.toBeInTheDocument();
      });
    });
  });

  describe('Reset Functionality', () => {
    beforeEach(async () => {
      // Expand the custom picker for these tests
      render(<CustomColorPicker {...defaultProps} />);
      const customButton = screen.getByTestId('show-custom-picker-button');
      await userEvent.click(customButton);
    });

    it('should call onReset when reset button is clicked', async () => {
      const resetButton = screen.getByRole('button', { name: /reset color/i });
      await userEvent.click(resetButton);

      expect(mockOnReset).toHaveBeenCalledTimes(1);
    });
  });

  describe('Accessibility', () => {
    it('should have role="group" for presets', () => {
      render(<CustomColorPicker {...defaultProps} />);

      const presetsGroup = screen.getByRole('group', { name: 'Preset colors' });
      expect(presetsGroup).toBeInTheDocument();
    });

    it('should have proper button types for presets', () => {
      render(<CustomColorPicker {...defaultProps} />);

      const presetButtons = screen.getAllByRole('button', { name: /select color/i });
      presetButtons.forEach((button) => {
        expect(button).toHaveAttribute('type', 'button');
      });
    });

    it('should have proper ARIA labels when picker is expanded', async () => {
      render(<CustomColorPicker {...defaultProps} />);
      
      const customButton = screen.getByTestId('show-custom-picker-button');
      await userEvent.click(customButton);

      await waitFor(() => {
        expect(screen.getByLabelText('Color picker')).toBeInTheDocument();
        expect(screen.getByLabelText('Hex color input')).toBeInTheDocument();
        expect(screen.getByLabelText('Reset color')).toBeInTheDocument();
      });
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
