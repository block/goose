import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import MentionPopover from '../MentionPopover';
import { IntlTestWrapper } from '../../i18n/test-utils';

vi.mock('../../acp/autocomplete', () => ({
  listSlashCommandItems: vi.fn(async () => [
    {
      name: 'obsidian-daily-note',
      extra: 'Create or open today\'s daily note',
      itemType: 'Skill',
      relativePath: 'obsidian-daily-note',
    },
  ]),
  listAgentMentionItems: vi.fn(async () => []),
}));

vi.mock('../../utils/workingDir', () => ({
  getInitialWorkingDir: () => '/tmp/test',
}));

describe('MentionPopover', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    Element.prototype.scrollIntoView = vi.fn();
  });

  it('portals above a composer card that uses overflow-hidden and transform', async () => {
    const trap = document.createElement('div');
    trap.setAttribute('data-testid', 'composer-trap');
    // Reproduces ChatInputCard + fadein: overflow clip + transform containing block.
    trap.style.overflow = 'hidden';
    trap.style.transform = 'translateY(0)';
    document.body.appendChild(trap);

    render(
      <IntlTestWrapper>
        <MentionPopover
          isOpen
          onClose={vi.fn()}
          onSelect={vi.fn()}
          position={{ x: 40, y: 400 }}
          query=""
          isSlashCommand
          selectedIndex={0}
          onSelectedIndexChange={vi.fn()}
          workingDir="/tmp/test"
        />
      </IntlTestWrapper>,
      { container: trap }
    );

    const popover = await waitFor(() => screen.getByTestId('mention-popover'));
    expect(popover.parentElement).toBe(document.body);
    expect(trap.contains(popover)).toBe(false);
    expect(await screen.findByText('obsidian-daily-note')).toBeInTheDocument();

    trap.remove();
  });
});
