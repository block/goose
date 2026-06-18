import { render, type RenderOptions, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { confirmToolAction } from '../api';
import { resolveAcpPermissionRequest } from '../acp/permissionRequests';
import { IntlTestWrapper } from '../i18n/test-utils';
import ToolApprovalButtons from './ToolApprovalButtons';

vi.mock('../api', () => ({
  confirmToolAction: vi.fn(),
}));

vi.mock('../acp/permissionRequests', () => ({
  resolveAcpPermissionRequest: vi.fn(),
}));

vi.mock('../acpChatFeatureFlag', () => ({
  USE_ACP_CHAT: true,
}));

const renderWithIntl = (ui: React.ReactElement, options?: RenderOptions) =>
  render(ui, { wrapper: IntlTestWrapper, ...options });

const confirmToolActionMock = vi.mocked(confirmToolAction);
const resolveAcpPermissionRequestMock = vi.mocked(resolveAcpPermissionRequest);

describe('ToolApprovalButtons', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('marks the approval accepted when the ACP request resolves', async () => {
    resolveAcpPermissionRequestMock.mockReturnValueOnce(true);

    renderWithIntl(
      <ToolApprovalButtons
        data={{
          id: 'tool-call-approved',
          toolName: 'developer__shell',
          sessionId: 'session-1',
        }}
      />
    );

    await userEvent.click(screen.getByRole('button', { name: 'Allow Once' }));

    expect(resolveAcpPermissionRequestMock).toHaveBeenCalledWith(
      'session-1',
      'tool-call-approved',
      'allow_once'
    );
    expect(confirmToolActionMock).not.toHaveBeenCalled();
    expect(screen.getByText('developer__shell - Allowed once')).toBeInTheDocument();
  });

  it('falls back to the REST confirmation when no ACP request is pending', async () => {
    resolveAcpPermissionRequestMock.mockReturnValueOnce(false);
    confirmToolActionMock.mockResolvedValueOnce({ error: undefined } as Awaited<
      ReturnType<typeof confirmToolAction>
    >);

    renderWithIntl(
      <ToolApprovalButtons
        data={{
          id: 'tool-call-rerun',
          toolName: 'developer__shell',
          sessionId: 'session-1',
        }}
      />
    );

    await userEvent.click(screen.getByRole('button', { name: 'Allow Once' }));

    expect(resolveAcpPermissionRequestMock).toHaveBeenCalledWith(
      'session-1',
      'tool-call-rerun',
      'allow_once'
    );
    expect(confirmToolActionMock).toHaveBeenCalledWith({
      body: {
        sessionId: 'session-1',
        id: 'tool-call-rerun',
        action: 'allow_once',
        principalType: 'Tool',
      },
    });
    expect(screen.getByText('developer__shell - Allowed once')).toBeInTheDocument();
  });
});
