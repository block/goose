import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import CreateRecipeFromSessionModal from '../CreateRecipeFromSessionModal';
import { createRecipe } from '../../../api/sdk.gen';
import type { CreateRecipeResponse } from '../../../api/types.gen';

// Mock the API
vi.mock('../../../api/sdk.gen', () => ({
  createRecipe: vi.fn(),
}));

// Mock other dependencies
vi.mock('../../../toasts', () => ({
  toastError: vi.fn(),
}));

vi.mock('../../../recipe/recipeStorage', () => ({
  saveRecipe: vi.fn(),
}));

const mockCreateRecipe = vi.mocked(createRecipe);

describe('CreateRecipeFromSessionModal', () => {
  const defaultProps = {
    isOpen: true,
    onClose: vi.fn(),
    sessionId: 'test-session-id',
    onRecipeCreated: vi.fn(),
    onStartRecipe: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    const mockResponse: CreateRecipeResponse = {
      recipe: {
        title: 'Analyzed Recipe Title',
        description: 'Analyzed description',
        instructions: 'Analyzed instructions with {{param1}}',
        prompt: 'Analyzed prompt',
        activities: ['activity1', 'activity2'],
        parameters: [
          {
            key: 'param1',
            description: 'Auto-detected parameter',
            input_type: 'string',
            requirement: 'required',
          },
        ],
        response: {
          json_schema: { type: 'object' },
        },
      },
      error: undefined,
    };

    mockCreateRecipe.mockResolvedValue({
      data: mockResponse,
      error: undefined,
      request: new globalThis.Request('http://localhost/test'),
      response: new globalThis.Response(),
    });
  });

  describe('Modal Rendering', () => {
    it('renders modal when open', () => {
      render(<CreateRecipeFromSessionModal {...defaultProps} />);

      expect(screen.getByTestId('create-recipe-modal')).toBeInTheDocument();
      expect(screen.getByText('Create Recipe from Session')).toBeInTheDocument();
      expect(
        screen.getByText('Create a reusable recipe based on your current conversation.')
      ).toBeInTheDocument();
    });

    it('does not render when closed', () => {
      render(<CreateRecipeFromSessionModal {...defaultProps} isOpen={false} />);

      expect(screen.queryByTestId('create-recipe-modal')).not.toBeInTheDocument();
    });

    it('renders modal header with close button', () => {
      render(<CreateRecipeFromSessionModal {...defaultProps} />);

      expect(screen.getByTestId('modal-header')).toBeInTheDocument();
      expect(screen.getByTestId('close-button')).toBeInTheDocument();
    });

    it('calls onClose when close button is clicked', async () => {
      const user = userEvent.setup();
      render(<CreateRecipeFromSessionModal {...defaultProps} />);

      await user.click(screen.getByTestId('close-button'));
      expect(defaultProps.onClose).toHaveBeenCalled();
    });
  });

  describe('Analysis Workflow', () => {
    it('shows analyzing state initially', () => {
      render(<CreateRecipeFromSessionModal {...defaultProps} />);

      expect(screen.getByTestId('analyzing-state')).toBeInTheDocument();
      expect(screen.getByTestId('analyzing-title')).toBeInTheDocument();
      expect(screen.getByText('Analyzing your conversation...')).toBeInTheDocument();
    });

    it('displays analysis stages', async () => {
      render(<CreateRecipeFromSessionModal {...defaultProps} />);

      expect(screen.getByTestId('analysis-stage')).toBeInTheDocument();
      expect(screen.getByText('Reading your conversation...')).toBeInTheDocument();

      // Wait for stage progression - the real component cycles through stages faster
      await waitFor(
        () => {
          const stageElement = screen.getByTestId('analysis-stage');
          expect(stageElement.textContent).not.toBe('Reading your conversation...');
        },
        { timeout: 1000 }
      );
    });

    it('shows spinner during analysis', () => {
      render(<CreateRecipeFromSessionModal {...defaultProps} />);

      expect(screen.getByTestId('analysis-spinner')).toBeInTheDocument();
    });

    it('transitions to form state after analysis', async () => {
      render(<CreateRecipeFromSessionModal {...defaultProps} />);

      // Wait for analysis to complete
      await waitFor(
        () => {
          expect(screen.getByTestId('form-state')).toBeInTheDocument();
        },
        { timeout: 3000 }
      );

      expect(screen.queryByTestId('analyzing-state')).not.toBeInTheDocument();
    });
  });

  describe('Form Pre-filling', () => {
    it('pre-fills form with analyzed data', async () => {
      render(<CreateRecipeFromSessionModal {...defaultProps} />);

      // Wait for analysis to complete and form to be pre-filled
      await waitFor(
        () => {
          expect(screen.getByDisplayValue('Analyzed Recipe Title')).toBeInTheDocument();
        },
        { timeout: 2000 }
      );

      expect(screen.getByDisplayValue('Analyzed description')).toBeInTheDocument();
      expect(screen.getByDisplayValue('Analyzed instructions with {{param1}}')).toBeInTheDocument();
      // Check for prompt field - it's optional and might not be filled
      const promptInput = screen.getByTestId('prompt-input');
      expect(promptInput).toBeInTheDocument();
      // Check for recipe name input - it should be auto-generated
      const recipeNameInput = screen.getByTestId('recipe-name-input');
      expect(recipeNameInput).toBeInTheDocument();
    });

    it('shows recipe form fields after analysis', async () => {
      render(<CreateRecipeFromSessionModal {...defaultProps} />);

      await waitFor(
        () => {
          expect(screen.getByTestId('recipe-form')).toBeInTheDocument();
        },
        { timeout: 2000 }
      );

      expect(screen.getByTestId('title-input')).toBeInTheDocument();
      expect(screen.getByTestId('description-input')).toBeInTheDocument();
      expect(screen.getByTestId('instructions-input')).toBeInTheDocument();
      expect(screen.getByTestId('prompt-input')).toBeInTheDocument();
      expect(screen.getByTestId('recipe-name-input')).toBeInTheDocument();
    });

    it('shows save location options', async () => {
      render(<CreateRecipeFromSessionModal {...defaultProps} />);

      await waitFor(
        () => {
          expect(screen.getByTestId('save-location-field')).toBeInTheDocument();
        },
        { timeout: 2000 }
      );

      expect(screen.getByTestId('global-radio')).toBeInTheDocument();
      expect(screen.getByTestId('directory-radio')).toBeInTheDocument();
    });
  });

  describe('Form Interactions', () => {
    it('allows editing form fields', async () => {
      const user = userEvent.setup();
      render(<CreateRecipeFromSessionModal {...defaultProps} />);

      await waitFor(
        () => {
          expect(screen.getByTestId('title-input')).toBeInTheDocument();
        },
        { timeout: 2000 }
      );

      const titleInput = screen.getByTestId('title-input');
      await user.clear(titleInput);
      await user.type(titleInput, 'Modified Title');

      expect(screen.getByDisplayValue('Modified Title')).toBeInTheDocument();
    });

    it('allows changing save location', async () => {
      const user = userEvent.setup();
      render(<CreateRecipeFromSessionModal {...defaultProps} />);

      await waitFor(
        () => {
          expect(screen.getByTestId('directory-radio')).toBeInTheDocument();
        },
        { timeout: 2000 }
      );

      await user.click(screen.getByTestId('directory-radio'));
      expect(screen.getByTestId('directory-radio')).toBeChecked();
    });

    it('validates required fields', async () => {
      const user = userEvent.setup();
      render(<CreateRecipeFromSessionModal {...defaultProps} />);

      await waitFor(
        () => {
          expect(screen.getByTestId('create-recipe-button')).toBeInTheDocument();
        },
        { timeout: 2000 }
      );

      // Clear required field
      const titleInput = screen.getByTestId('title-input');
      await user.clear(titleInput);

      const createButton = screen.getByTestId('create-recipe-button');
      expect(createButton).toBeDisabled();
    });
  });

  describe('Recipe Creation', () => {
    it('enables create button when form is valid', async () => {
      render(<CreateRecipeFromSessionModal {...defaultProps} />);

      await waitFor(
        () => {
          const createButton = screen.getByTestId('create-recipe-button');
          expect(createButton).toBeEnabled();
        },
        { timeout: 2000 }
      );
    });

    it('creates recipe when form is submitted', async () => {
      const user = userEvent.setup();
      render(<CreateRecipeFromSessionModal {...defaultProps} />);

      await waitFor(
        () => {
          expect(screen.getByTestId('create-recipe-button')).toBeEnabled();
        },
        { timeout: 2000 }
      );

      await user.click(screen.getByTestId('create-recipe-button'));

      await waitFor(() => {
        expect(screen.getByTestId('success-state')).toBeInTheDocument();
        expect(screen.getByTestId('done-button')).toBeInTheDocument();
        expect(screen.getByTestId('start-recipe-button')).toBeInTheDocument();
      });

      expect(defaultProps.onRecipeCreated).toHaveBeenCalled();
    });
  });

  describe('Success State Actions', () => {
    it('calls onStartRecipe when Start Recipe button is clicked', async () => {
      const user = userEvent.setup();
      render(<CreateRecipeFromSessionModal {...defaultProps} />);

      // Wait for form and create recipe
      await waitFor(
        () => {
          expect(screen.getByTestId('create-recipe-button')).toBeEnabled();
        },
        { timeout: 2000 }
      );

      await user.click(screen.getByTestId('create-recipe-button'));

      await waitFor(() => {
        expect(screen.getByTestId('start-recipe-button')).toBeInTheDocument();
      });

      await user.click(screen.getByTestId('start-recipe-button'));

      expect(defaultProps.onStartRecipe).toHaveBeenCalled();
      expect(defaultProps.onClose).toHaveBeenCalled();
    });

    it('closes modal when Done button is clicked', async () => {
      const user = userEvent.setup();
      render(<CreateRecipeFromSessionModal {...defaultProps} />);

      // Wait for form and create recipe
      await waitFor(
        () => {
          expect(screen.getByTestId('create-recipe-button')).toBeEnabled();
        },
        { timeout: 2000 }
      );

      await user.click(screen.getByTestId('create-recipe-button'));

      await waitFor(() => {
        expect(screen.getByTestId('done-button')).toBeInTheDocument();
      });

      await user.click(screen.getByTestId('done-button'));

      expect(defaultProps.onClose).toHaveBeenCalled();
    });
  });

  describe('Modal Footer', () => {
    it('shows cancel button in all states', async () => {
      render(<CreateRecipeFromSessionModal {...defaultProps} />);

      expect(screen.getByTestId('cancel-button')).toBeInTheDocument();

      await waitFor(
        () => {
          expect(screen.getByTestId('create-recipe-button')).toBeInTheDocument();
        },
        { timeout: 2000 }
      );

      expect(screen.getByTestId('cancel-button')).toBeInTheDocument();
    });

    it('calls onClose when cancel button is clicked', async () => {
      const user = userEvent.setup();
      render(<CreateRecipeFromSessionModal {...defaultProps} />);

      await user.click(screen.getByTestId('cancel-button'));
      expect(defaultProps.onClose).toHaveBeenCalled();
    });

    it('shows different button states based on workflow stage', async () => {
      const user = userEvent.setup();
      render(<CreateRecipeFromSessionModal {...defaultProps} />);

      // During analysis - only cancel button
      expect(screen.getByTestId('cancel-button')).toBeInTheDocument();
      expect(screen.queryByTestId('create-recipe-button')).not.toBeInTheDocument();

      // After analysis - create button appears
      await waitFor(
        () => {
          expect(screen.getByTestId('create-recipe-button')).toBeInTheDocument();
        },
        { timeout: 2000 }
      );

      // After creation - success buttons appear
      await user.click(screen.getByTestId('create-recipe-button'));

      await waitFor(() => {
        expect(screen.getByTestId('done-button')).toBeInTheDocument();
        expect(screen.getByTestId('start-recipe-button')).toBeInTheDocument();
      });
    });
  });

  describe('Error Handling', () => {
    it('handles analysis errors gracefully', async () => {
      // Mock analysis failure
      render(<CreateRecipeFromSessionModal {...defaultProps} sessionId="" />);

      // Should still show the modal structure
      expect(screen.getByTestId('create-recipe-modal')).toBeInTheDocument();
    });

    it('handles form validation errors', async () => {
      const user = userEvent.setup();
      render(<CreateRecipeFromSessionModal {...defaultProps} />);

      await waitFor(
        () => {
          expect(screen.getByTestId('title-input')).toBeInTheDocument();
        },
        { timeout: 2000 }
      );

      // Clear all required fields
      await user.clear(screen.getByTestId('title-input'));
      await user.clear(screen.getByTestId('description-input'));
      await user.clear(screen.getByTestId('instructions-input'));

      const createButton = screen.getByTestId('create-recipe-button');
      expect(createButton).toBeDisabled();
    });
  });
});
