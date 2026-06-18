import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { ActionRequired } from '../api';
import { IntlTestWrapper } from '../i18n/test-utils';
import ElicitationRequest from './ElicitationRequest';

const actionRequiredContent = {
  type: 'actionRequired',
  data: {
    actionType: 'elicitation',
    id: 'elicitation-1',
    message: 'Need more information',
    requested_schema: {
      type: 'object',
      properties: {},
    },
  },
} as ActionRequired & { type: 'actionRequired' };

type SubmitElicitationResponse = (
  elicitationId: string,
  userData: Record<string, unknown>
) => Promise<boolean>;

function renderElicitationRequest(onSubmit: SubmitElicitationResponse) {
  return render(
    <ElicitationRequest
      isCancelledMessage={false}
      isClicked={false}
      actionRequiredContent={actionRequiredContent}
      onSubmit={onSubmit}
    />,
    { wrapper: IntlTestWrapper }
  );
}

describe('ElicitationRequest', () => {
  it('shows submitted state when the response is accepted', async () => {
    const onSubmit = vi.fn<SubmitElicitationResponse>().mockResolvedValue(true);

    renderElicitationRequest(onSubmit);

    await userEvent.click(screen.getByRole('button', { name: 'Accept' }));

    expect(onSubmit).toHaveBeenCalledWith('elicitation-1', {});
    expect(await screen.findByText('Information submitted')).toBeInTheDocument();
  });

  it('shows submitted state while the response is pending', async () => {
    let resolveSubmission: (value: boolean) => void = () => {};
    const submission = new Promise<boolean>((resolve) => {
      resolveSubmission = resolve;
    });
    const onSubmit = vi.fn<SubmitElicitationResponse>().mockReturnValue(submission);

    renderElicitationRequest(onSubmit);

    await userEvent.click(screen.getByRole('button', { name: 'Accept' }));

    expect(screen.getByText('Information submitted')).toBeInTheDocument();

    await act(async () => {
      resolveSubmission(true);
      await submission;
    });
  });

  it('keeps the request actionable when no ACP request is pending', async () => {
    const onSubmit = vi.fn<SubmitElicitationResponse>().mockResolvedValue(false);

    renderElicitationRequest(onSubmit);

    await userEvent.click(screen.getByRole('button', { name: 'Accept' }));

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'This request is no longer active. The extension will need to ask again.'
    );
    expect(screen.queryByText('Information submitted')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Accept' })).toBeEnabled();
  });
});
