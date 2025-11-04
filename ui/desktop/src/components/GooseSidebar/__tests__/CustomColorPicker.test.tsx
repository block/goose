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

  it('should render all UI elements', () => {
    render(<CustomColorPicker {...defaultProps} />);

    expect(screen.getByLabelText('Color picker')).toBeInTheDocument();
    expect(screen.getByLabelText('Hex color input')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /reset color/i })).toBeInTheDocument();
    expect(screen.getAllByRole('button', { name: /select color/i })).toHaveLength(10);
  });

  it('should call onChange when color picker changes', async () => {
    render(<CustomColorPicker {...defaultProps} />);

    const colorInput = screen.getByLabelText('Color picker');
    fireEvent.change(colorInput, { target: { value: '#ff0000' } });

    await waitFor(() => {
      expect(mockOnChange).toHaveBeenCalledWith('#ff0000');
    });
  });

  it('should accept valid hex input', async () => {
    render(<CustomColorPicker {...defaultProps} />);

    const hexInput = screen.getByLabelText('Hex color input');
    await userEvent.clear(hexInput);
    await userEvent.type(hexInput, '#ff0000');

    expect(mockOnChange).toHaveBeenCalledWith('#ff0000');
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('should show error for invalid hex input', async () => {
    render(<CustomColorPicker {...defaultProps} />);

    const hexInput = screen.getByLabelText('Hex color input');
    await userEvent.clear(hexInput);
    await userEvent.type(hexInput, 'invalid');

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('Invalid hex color');
      expect(hexInput).toHaveAttribute('aria-invalid', 'true');
    });
    expect(mockOnChange).not.toHaveBeenCalledWith('invalid');
  });

  it('should call onChange when preset is clicked', async () => {
    render(<CustomColorPicker {...defaultProps} />);

    const tealPreset = screen.getByRole('button', { name: 'Select color #13bbaf' });
    await userEvent.click(tealPreset);

    expect(mockOnChange).toHaveBeenCalledWith('#13bbaf');
  });

  it('should call onReset when reset button is clicked', async () => {
    render(<CustomColorPicker {...defaultProps} />);

    const resetButton = screen.getByRole('button', { name: /reset color/i });
    await userEvent.click(resetButton);

    expect(mockOnReset).toHaveBeenCalledTimes(1);
  });

  it('should update when value prop changes', () => {
    const { rerender } = render(<CustomColorPicker {...defaultProps} value="#ff0000" />);

    let hexInput = screen.getByLabelText('Hex color input') as HTMLInputElement;
    expect(hexInput.value).toBe('#ff0000');

    rerender(<CustomColorPicker {...defaultProps} value="#00ff00" />);

    hexInput = screen.getByLabelText('Hex color input') as HTMLInputElement;
    expect(hexInput.value).toBe('#00ff00');
  });
});
