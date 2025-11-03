import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { CustomColorPicker } from '../CustomColorPicker';

// Mock lucide-react icons
vi.mock('lucide-react', () => ({
  RotateCcw: () => <div data-testid="reset-icon">Reset</div>,
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
    it('should render color picker input', () => {
      render(<CustomColorPicker {...defaultProps} />);

      const colorInput = screen.getByLabelText('Color picker');
      expect(colorInput).toBeInTheDocument();
      expect(colorInput).toHaveAttribute('type', 'color');
      expect(colorInput).toHaveValue('#32353b');
    });

    it('should render hex input field', () => {
      render(<CustomColorPicker {...defaultProps} />);

      const hexInput = screen.getByLabelText('Hex color input');
      expect(hexInput).toBeInTheDocument();
      expect(hexInput).toHaveValue('#32353b');
      expect(hexInput).toHaveAttribute('placeholder', '#32353b');
    });

    it('should render reset button', () => {
      render(<CustomColorPicker {...defaultProps} />);

      const resetButton = screen.getByRole('button', { name: /reset color/i });
      expect(resetButton).toBeInTheDocument();
    });

    it('should render preset colors', () => {
      render(<CustomColorPicker {...defaultProps} />);

      const presetButtons = screen.getAllByRole('button', { name: /select color/i });
      expect(presetButtons).toHaveLength(10); // 10 preset colors
    });

    it('should render presets label', () => {
      render(<CustomColorPicker {...defaultProps} />);

      expect(screen.getByText('Presets')).toBeInTheDocument();
    });
  });

  describe('Color Picker Interaction', () => {
    it('should call onChange when color picker value changes', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const colorInput = screen.getByLabelText('Color picker');
      await userEvent.clear(colorInput);
      await userEvent.type(colorInput, '#ff0000');

      await waitFor(() => {
        expect(mockOnChange).toHaveBeenCalledWith('#ff0000');
      });
    });

    it('should update input value when color picker changes', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const colorInput = screen.getByLabelText('Color picker') as HTMLInputElement;
      fireEvent.change(colorInput, { target: { value: '#ff0000' } });

      const hexInput = screen.getByLabelText('Hex color input') as HTMLInputElement;
      await waitFor(() => {
        expect(hexInput.value).toBe('#ff0000');
      });
    });
  });

  describe('Hex Input Validation', () => {
    it('should accept valid hex colors', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const hexInput = screen.getByLabelText('Hex color input');
      await userEvent.clear(hexInput);
      await userEvent.type(hexInput, '#ff0000');

      expect(mockOnChange).toHaveBeenCalledWith('#ff0000');
      expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    });

    it('should show error for invalid hex colors', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const hexInput = screen.getByLabelText('Hex color input');
      await userEvent.clear(hexInput);
      await userEvent.type(hexInput, 'invalid');

      await waitFor(() => {
        expect(screen.getByRole('alert')).toHaveTextContent('Invalid hex color');
      });
    });

    it('should not call onChange for invalid hex', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const hexInput = screen.getByLabelText('Hex color input');
      await userEvent.clear(hexInput);
      await userEvent.type(hexInput, 'invalid');

      // Should not have been called with invalid value
      expect(mockOnChange).not.toHaveBeenCalledWith('invalid');
    });

    it('should mark input as invalid with aria-invalid', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const hexInput = screen.getByLabelText('Hex color input');
      await userEvent.clear(hexInput);
      await userEvent.type(hexInput, 'invalid');

      await waitFor(() => {
        expect(hexInput).toHaveAttribute('aria-invalid', 'true');
      });
    });

    it('should link error message with aria-describedby', async () => {
      render(<CustomColorPicker {...defaultProps} />);

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

    it('should update hex input when preset is clicked', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const orangePreset = screen.getByRole('button', { name: 'Select color #ff4f00' });
      await userEvent.click(orangePreset);

      const hexInput = screen.getByLabelText('Hex color input') as HTMLInputElement;
      await waitFor(() => {
        expect(hexInput.value).toBe('#ff4f00');
      });
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

  describe('Reset Functionality', () => {
    it('should call onReset when reset button is clicked', async () => {
      render(<CustomColorPicker {...defaultProps} />);

      const resetButton = screen.getByRole('button', { name: /reset color/i });
      await userEvent.click(resetButton);

      expect(mockOnReset).toHaveBeenCalledTimes(1);
    });
  });

  describe('Controlled Component', () => {
    it('should update when value prop changes', () => {
      const { rerender } = render(<CustomColorPicker {...defaultProps} value="#ff0000" />);

      let hexInput = screen.getByLabelText('Hex color input') as HTMLInputElement;
      expect(hexInput.value).toBe('#ff0000');

      rerender(<CustomColorPicker {...defaultProps} value="#00ff00" />);

      hexInput = screen.getByLabelText('Hex color input') as HTMLInputElement;
      expect(hexInput.value).toBe('#00ff00');
    });
  });

  describe('Accessibility', () => {
    it('should have proper ARIA labels', () => {
      render(<CustomColorPicker {...defaultProps} />);

      expect(screen.getByLabelText('Color picker')).toBeInTheDocument();
      expect(screen.getByLabelText('Hex color input')).toBeInTheDocument();
      expect(screen.getByLabelText('Reset color')).toBeInTheDocument();
    });

    it('should have role="group" for presets', () => {
      render(<CustomColorPicker {...defaultProps} />);

      const presetsGroup = screen.getByRole('group', { name: 'Preset colors' });
      expect(presetsGroup).toBeInTheDocument();
    });

    it('should have proper button types', () => {
      render(<CustomColorPicker {...defaultProps} />);

      const presetButtons = screen.getAllByRole('button', { name: /select color/i });
      presetButtons.forEach((button) => {
        expect(button).toHaveAttribute('type', 'button');
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
