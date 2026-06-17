/**
 * @vitest-environment jsdom
 */
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { Goose } from './Goose';

describe('Goose brand icon', () => {
  it('renders the IBM Carbon AiEnabledEdt mark', () => {
    const { container } = render(<Goose data-testid="brand-icon" className="size-6" />);
    const icon = screen.getByTestId('brand-icon');

    expect(icon.tagName.toLowerCase()).toBe('svg');
    expect(icon).toHaveAttribute('data-brand-icon', 'ibm-carbon-ai-enabled-edt');
    expect(icon).toHaveAttribute('viewBox', '0 0 32 32');
    expect(container.querySelector('path')?.getAttribute('d')).toContain('M18,20v-2h1v-7h-1v-2');
  });
});
