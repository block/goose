/**
 * @vitest-environment jsdom
 */
import type { PlanEntry } from '@agentclientprotocol/sdk';
import { act, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { IntlTestWrapper } from '../i18n/test-utils';
import TodoDock from './TodoDock';

const incompletePlan: PlanEntry[] = [
  {
    content: 'Continue restored work',
    priority: 'medium',
    status: 'in_progress',
  },
];

describe('TodoDock', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it('keeps an incomplete restored plan visible', () => {
    const { container } = render(<TodoDock entries={incompletePlan} />, {
      wrapper: IntlTestWrapper,
    });

    expect(screen.getByRole('button', { name: 'Expand task list' })).toBeVisible();
    expect(container.firstElementChild).toHaveAttribute('aria-hidden', 'false');
    expect(container.firstElementChild).not.toHaveAttribute('inert');
  });

  it('makes the dock inert after a completed plan is dismissed', () => {
    vi.useFakeTimers();
    const completedPlan: PlanEntry[] = [
      {
        content: 'Finished work',
        priority: 'medium',
        status: 'completed',
      },
    ];
    const { container } = render(<TodoDock entries={completedPlan} />, {
      wrapper: IntlTestWrapper,
    });

    act(() => vi.advanceTimersByTime(650));

    expect(container.firstElementChild).toHaveAttribute('aria-hidden', 'true');
    expect(container.firstElementChild).toHaveAttribute('inert');
  });
});
