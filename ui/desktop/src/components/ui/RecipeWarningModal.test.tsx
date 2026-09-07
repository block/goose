import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { IntlTestWrapper } from '../../i18n/test-utils';
import { RecipeWarningModal } from './RecipeWarningModal';

const baseProps = {
  isOpen: true,
  onConfirm: vi.fn(),
  onCancel: vi.fn(),
  recipeDetails: { title: 'Analyzer', description: 'Shared recipe' },
};

describe('RecipeWarningModal', () => {
  it('lists the commands a recipe will run', () => {
    render(
      <RecipeWarningModal
        {...baseProps}
        commands={[
          { source: 'extension "analyzer"', command: 'sh -c id' },
          { source: 'retry check', command: 'test -f done' },
        ]}
      />,
      { wrapper: IntlTestWrapper }
    );

    expect(screen.getByText(/Commands this recipe will run/i)).toBeInTheDocument();
    expect(screen.getByText('sh -c id')).toBeInTheDocument();
    expect(screen.getByText('test -f done')).toBeInTheDocument();
    expect(screen.getByText(/extension "analyzer"/)).toBeInTheDocument();
  });

  it('omits the commands section when there are none', () => {
    render(<RecipeWarningModal {...baseProps} commands={[]} />, { wrapper: IntlTestWrapper });

    expect(screen.queryByText(/Commands this recipe will run/i)).not.toBeInTheDocument();
  });

  it('disables Trust and Execute while the scan is pending', () => {
    render(<RecipeWarningModal {...baseProps} scanPending />, { wrapper: IntlTestWrapper });

    expect(screen.getByRole('button', { name: /Trust and Execute/i })).toBeDisabled();
    expect(screen.getByText(/Checking what this recipe will run/i)).toBeInTheDocument();
  });

  it('disables Trust and Execute when the scan fails', () => {
    render(<RecipeWarningModal {...baseProps} scanFailed />, { wrapper: IntlTestWrapper });

    expect(screen.getByRole('button', { name: /Trust and Execute/i })).toBeDisabled();
    expect(screen.getByText(/Could not check what this recipe will run/i)).toBeInTheDocument();
  });
});
