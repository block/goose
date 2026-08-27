import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import RecipeActivities from './RecipeActivities';

describe('RecipeActivities', () => {
  it('disables activity submissions while recipe trust is unresolved', () => {
    const append = vi.fn();
    render(<RecipeActivities append={append} activities={['Run analysis']} disabled />);

    const activity = screen.getByRole('button', { name: 'Run analysis' });
    expect(activity).toBeDisabled();
    fireEvent.click(activity);
    expect(append).not.toHaveBeenCalled();
  });

  it('submits activities after recipe trust resolves', () => {
    const append = vi.fn().mockResolvedValue(true);
    render(<RecipeActivities append={append} activities={['Run analysis']} />);

    fireEvent.click(screen.getByRole('button', { name: 'Run analysis' }));
    expect(append).toHaveBeenCalledWith('Run analysis');
  });
});
