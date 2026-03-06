import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import RecipeActivityEditor from '../../RecipeActivityEditor';
import { LocalizationProvider } from '../../../../contexts/LocalizationContext';

const renderWithLocalization = (component: React.ReactElement) =>
  render(<LocalizationProvider>{component}</LocalizationProvider>);

describe('RecipeActivityEditor', () => {
  const mockOnChange = vi.fn();
  const mockOnBlur = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Basic Rendering', () => {
    it('renders without crashing', () => {
      renderWithLocalization(<RecipeActivityEditor activities={[]} setActivities={mockOnChange} />);
      expect(screen.getByText('Activities')).toBeInTheDocument();
    });

    it('displays the activities label', () => {
      renderWithLocalization(<RecipeActivityEditor activities={[]} setActivities={mockOnChange} />);
      expect(screen.getByText('Activities')).toBeInTheDocument();
    });

    it('shows helper text', () => {
      renderWithLocalization(<RecipeActivityEditor activities={[]} setActivities={mockOnChange} />);
      expect(screen.getByText(/top-line prompts and activity buttons/)).toBeInTheDocument();
    });
  });

  describe('Empty State', () => {
    it('shows message input when no activities', () => {
      renderWithLocalization(<RecipeActivityEditor activities={[]} setActivities={mockOnChange} />);
      expect(screen.getByText('Message')).toBeInTheDocument();
      expect(
        screen.getByPlaceholderText(/Enter a user-facing introduction message/)
      ).toBeInTheDocument();
    });
  });

  describe('With Activities', () => {
    it('displays existing activities as visual boxes', () => {
      const activities = ['message: Hello World', 'button: Click me', 'action: Do something'];
      renderWithLocalization(
        <RecipeActivityEditor activities={activities} setActivities={mockOnChange} />
      );

      const messageTextarea = screen.getByPlaceholderText(
        /Enter a user-facing introduction message/
      );
      expect(messageTextarea).toHaveValue(' Hello World');

      expect(screen.getByText('button: Click me')).toBeInTheDocument();
      expect(screen.getByText('action: Do something')).toBeInTheDocument();

      const removeButtons = screen.getAllByText('×');
      expect(removeButtons).toHaveLength(2);
    });

    it('truncates long activity text in boxes', () => {
      const longActivity = 'button: ' + 'a'.repeat(150);
      const activities = [longActivity];
      renderWithLocalization(
        <RecipeActivityEditor activities={activities} setActivities={mockOnChange} />
      );

      expect(screen.getByText(/button: a+\.\.\./)).toBeInTheDocument();

      const activityBox = screen.getByText(/button: a+\.\.\./).closest('div');
      expect(activityBox).toHaveAttribute('title', longActivity);
    });

    it('handles empty activities array', () => {
      renderWithLocalization(<RecipeActivityEditor activities={[]} setActivities={mockOnChange} />);
      expect(screen.getByText('Activities')).toBeInTheDocument();

      expect(screen.queryByText('×')).not.toBeInTheDocument();
    });

    it('allows removing activities via remove buttons', async () => {
      const user = userEvent.setup();
      const activities = ['button: Click me', 'action: Do something'];
      renderWithLocalization(
        <RecipeActivityEditor activities={activities} setActivities={mockOnChange} />
      );

      const removeButtons = screen.getAllByText('×');
      await user.click(removeButtons[0]);

      expect(mockOnChange).toHaveBeenCalledWith(['action: Do something']);
    });
  });

  describe('User Interactions', () => {
    it('allows typing in message field', async () => {
      const user = userEvent.setup();
      renderWithLocalization(<RecipeActivityEditor activities={[]} setActivities={mockOnChange} />);

      const messageInput = screen.getByPlaceholderText(
        /Enter a user-facing introduction message/
      );
      await user.type(messageInput, 'Test message');

      expect(messageInput).toHaveValue('Test message');
    });

    it('calls onBlur when provided', async () => {
      const user = userEvent.setup();
      renderWithLocalization(
        <RecipeActivityEditor activities={[]} setActivities={mockOnChange} onBlur={mockOnBlur} />
      );

      const messageInput = screen.getByPlaceholderText(
        /Enter a user-facing introduction message/
      );
      await user.click(messageInput);
      await user.tab();

      expect(mockOnBlur).toHaveBeenCalled();
    });
  });
});
