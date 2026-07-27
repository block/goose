import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { IntlTestWrapper } from '../../i18n/test-utils';
import { ContextWindowIndicator } from './ContextWindowIndicator';

describe('ContextWindowIndicator', () => {
  it('announces the token values alongside the action', () => {
    render(
      <IntlTestWrapper>
        <ContextWindowIndicator totalTokens={38_200} tokenLimit={200_000} onOpenXray={vi.fn()} />
      </IntlTestWrapper>
    );

    expect(
      screen.getByRole('button', { name: '38k of 200k tokens used, open context x-ray' })
    ).toBeInTheDocument();
  });

  it('renders nothing without a token limit', () => {
    const { container } = render(
      <IntlTestWrapper>
        <ContextWindowIndicator totalTokens={38_200} tokenLimit={0} />
      </IntlTestWrapper>
    );

    expect(container).toBeEmptyDOMElement();
  });
});
