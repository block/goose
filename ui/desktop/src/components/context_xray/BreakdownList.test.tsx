import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import type { ContextReport, ContextSegment } from '../../types/contextReport';
import { IntlTestWrapper } from '../../i18n/test-utils';
import { BreakdownList } from './BreakdownList';

function report(segments: ContextSegment[]): ContextReport {
  return {
    model: { modelName: 'test-model', contextLimit: 100_000 },
    estimatedTotalTokens: segments.reduce((sum, segment) => sum + segment.tokenCount, 0),
    wireTotalTokens: segments.reduce((sum, segment) => sum + segment.tokenCount, 0),
    segments,
  };
}

const STRUCTURED_SUMMARY: ContextSegment = {
  category: 'compaction_summary',
  label: 'Conversation summary',
  source: 'structured',
  tokenCount: 900,
  charCount: 3200,
  parts: [
    {
      label: 'User Intent',
      tokenCount: 120,
      charCount: 400,
      contentPreview: '- Fix the parser bug',
    },
    {
      label: 'Files + Code',
      tokenCount: 780,
      charCount: 2800,
      contentPreview: '### src/parser.rs',
    },
  ],
};

function renderBreakdown(segments: ContextSegment[]) {
  render(
    <IntlTestWrapper>
      <BreakdownList report={report(segments)} />
    </IntlTestWrapper>
  );
}

describe('BreakdownList compaction summary', () => {
  it('shows the summary as its own category and drills down into its sections', async () => {
    const user = userEvent.setup();
    renderBreakdown([STRUCTURED_SUMMARY]);

    await user.click(screen.getByText('Compaction summary'));

    expect(screen.getByText('structured')).toBeInTheDocument();
    await user.click(screen.getByText('Conversation summary'));

    expect(screen.getByText('User Intent')).toBeInTheDocument();
    expect(screen.getByText('Files + Code')).toBeInTheDocument();
  });

  it('labels a raw fallback summary and offers the whole text instead of sections', async () => {
    const user = userEvent.setup();
    renderBreakdown([
      {
        ...STRUCTURED_SUMMARY,
        source: 'raw fallback',
        parts: [],
        contentPreview: 'A prose recap with no summary document.',
      },
    ]);

    await user.click(screen.getByText('Compaction summary'));

    expect(screen.getByText('unstructured model output')).toBeInTheDocument();

    await user.click(screen.getByText('Conversation summary'));
    expect(screen.getByText('A prose recap with no summary document.')).toBeInTheDocument();
  });
});
