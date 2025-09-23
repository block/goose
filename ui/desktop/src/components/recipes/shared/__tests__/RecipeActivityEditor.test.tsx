import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import RecipeActivityEditor from '../../RecipeActivityEditor';

describe('RecipeActivityEditor', () => {
  const mockOnChange = vi.fn();
  const mockOnBlur = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Basic Rendering', () => {
    it('renders without crashing', () => {
      render(<RecipeActivityEditor activities={[]} setActivities={mockOnChange} />);
      expect(screen.getByText('Activities')).toBeInTheDocument();
    });

    it('displays the activities label', () => {
      render(<RecipeActivityEditor activities={[]} setActivities={mockOnChange} />);
      expect(screen.getByText('Activities')).toBeInTheDocument();
    });

    it('shows helper text', () => {
      render(<RecipeActivityEditor activities={[]} setActivities={mockOnChange} />);
      expect(screen.getByText(/top-line prompts and activity buttons/)).toBeInTheDocument();
    });
  });

  describe('Empty State', () => {
    it('shows message input when no activities', () => {
      render(<RecipeActivityEditor activities={[]} setActivities={mockOnChange} />);
      expect(screen.getByText('Message')).toBeInTheDocument();
      expect(
        screen.getByPlaceholderText(/Enter a user facing introduction message/)
      ).toBeInTheDocument();
    });
  });

  describe('With Activities', () => {
    it('displays existing activities as visual boxes', () => {
      const activities = ['message: Hello World', 'button: Click me', 'action: Do something'];
      render(<RecipeActivityEditor activities={activities} setActivities={mockOnChange} />);

      // Should show the message content in the message textarea (the component strips "message:" prefix but keeps the space)
      const messageTextarea = screen.getByPlaceholderText(
        /Enter a user facing introduction message/
      );
      expect(messageTextarea).toHaveValue(' Hello World');

      // Should show non-message activities as visual boxes with remove buttons
      expect(screen.getByText('button: Click me')).toBeInTheDocument();
      expect(screen.getByText('action: Do something')).toBeInTheDocument();

      // Should have remove buttons (×) for each activity box
      const removeButtons = screen.getAllByText('×');
      expect(removeButtons).toHaveLength(2); // Two non-message activities
    });

    it('truncates long activity text in boxes', () => {
      const longActivity = 'button: ' + 'a'.repeat(150); // Create a very long activity
      const activities = [longActivity];
      render(<RecipeActivityEditor activities={activities} setActivities={mockOnChange} />);

      // Should show truncated text with ellipsis
      expect(screen.getByText(/button: a+\.\.\./)).toBeInTheDocument();

      // Should have title attribute with full text for tooltip
      const activityBox = screen.getByText(/button: a+\.\.\./).closest('div');
      expect(activityBox).toHaveAttribute('title', longActivity);
    });

    it('handles empty activities array', () => {
      render(<RecipeActivityEditor activities={[]} setActivities={mockOnChange} />);
      expect(screen.getByText('Activities')).toBeInTheDocument();

      // Should not show any activity boxes
      expect(screen.queryByText('×')).not.toBeInTheDocument();
    });

    it('allows removing activities via remove buttons', async () => {
      const user = userEvent.setup();
      const activities = ['button: Click me', 'action: Do something'];
      render(<RecipeActivityEditor activities={activities} setActivities={mockOnChange} />);

      // Click the remove button for the first activity
      const removeButtons = screen.getAllByText('×');
      await user.click(removeButtons[0]);

      // Should call setActivities with the activity removed
      expect(mockOnChange).toHaveBeenCalledWith(['action: Do something']);
    });
  });

  describe('User Interactions', () => {
    it('allows typing in message field', async () => {
      const user = userEvent.setup();
      render(<RecipeActivityEditor activities={[]} setActivities={mockOnChange} />);

      const messageInput = screen.getByPlaceholderText(/Enter a user facing introduction message/);
      await user.type(messageInput, 'Test message');

      expect(messageInput).toHaveValue('Test message');
    });

    it('calls onBlur when provided', async () => {
      const user = userEvent.setup();
      render(
        <RecipeActivityEditor activities={[]} setActivities={mockOnChange} onBlur={mockOnBlur} />
      );

      const messageInput = screen.getByPlaceholderText(/Enter a user facing introduction message/);
      await user.click(messageInput);
      await user.tab(); // Blur the input

      expect(mockOnBlur).toHaveBeenCalled();
    });
  });

  describe('Props Handling', () => {
    it('works without onBlur callback', () => {
      expect(() => {
        render(<RecipeActivityEditor activities={[]} setActivities={mockOnChange} />);
      }).not.toThrow();
    });

    it('handles undefined activities gracefully', () => {
      // The component should handle undefined activities by defaulting to empty array
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      render(<RecipeActivityEditor activities={undefined as any} setActivities={mockOnChange} />);
      expect(screen.getByText('Activities')).toBeInTheDocument();
    });
  });
});
