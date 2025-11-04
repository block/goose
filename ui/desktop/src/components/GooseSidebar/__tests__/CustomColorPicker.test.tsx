import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { CustomColorPicker } from '../CustomColorPicker';

// Mock lucide-react icons
vi.mock('lucide-react', () => ({
  Plus: () => <div data-testid="plus-icon">+</div>,
}));

// Mock colorUtils
vi.mock('../../../utils/colorUtils', () => ({
  isValidHexColor: (hex: string) => /^#[0-9a-f]{6}$/i.test(hex),
  DEFAULT_THEME_COLOR: '#32353b',
}));

describe('CustomColorPicker', () => {
  const mockOnChange = vi.fn();
  const defaultProps = {
    value: '#32353b',
    onChange: mockOnChange,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render preset colors and add button', () => {
    render(<CustomColorPicker {...defaultProps} />);

    // Should have 6 preset colors
    const presetButtons = screen.getAllByRole('button', { name: /select color/i });
    expect(presetButtons).toHaveLength(6);

    // Should have add custom color button
    expect(screen.getByTestId('add-custom-color-button')).toBeInTheDocument();
  });

  it('should call onChange when preset is clicked', async () => {
    render(<CustomColorPicker {...defaultProps} />);

    const tealPreset = screen.getByRole('button', { name: 'Select color #13bbaf' });
    await userEvent.click(tealPreset);

    expect(mockOnChange).toHaveBeenCalledWith('#13bbaf');
  });

  it('should open dialog when add button is clicked', async () => {
    render(<CustomColorPicker {...defaultProps} />);

    const addButton = screen.getByTestId('add-custom-color-button');
    await userEvent.click(addButton);

    await waitFor(() => {
      expect(screen.getByRole('dialog')).toBeInTheDocument();
      expect(screen.getByText('Add Custom Color')).toBeInTheDocument();
    });
  });

  it('should validate hex input in dialog', async () => {
    render(<CustomColorPicker {...defaultProps} />);

    const addButton = screen.getByTestId('add-custom-color-button');
    await userEvent.click(addButton);

    await waitFor(() => {
      expect(screen.getByRole('dialog')).toBeInTheDocument();
    });

    const hexInput = screen.getByLabelText('Hex color input');
    await userEvent.clear(hexInput);
    await userEvent.type(hexInput, 'invalid');

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('Invalid hex color');
      expect(hexInput).toHaveAttribute('aria-invalid', 'true');
    });
  });
});
