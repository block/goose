import { render, type RenderOptions, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { confirmToolAction } from '../api';
import { resolveAcpPermissionRequest } from '../acp/permissionRequests';
import { IntlTestWrapper } from '../i18n/test-utils';
import ToolApprovalButtons from './ToolApprovalButtons';

const acpChatFeatureFlagMock = vi.hoisted(() => ({
  useAcpChat: true,
}));

vi.mock('../api', () => ({
  confirmToolAction: vi.fn(),
}));

vi.mock('../acp/permissionRequests', () => ({
  resolveAcpPermissionRequest: vi.fn(),
}));

vi.mock('../acpChatFeatureFlag', () => ({
  get USE_ACP_CHAT() {
    return acpChatFeatureFlagMock.useAcpChat;
  },
}));

const renderWithIntl = (ui: React.ReactElement, options?: RenderOptions) =>
  render(ui, { wrapper: IntlTestWrapper, ...options });

const confirmToolActionMock = vi.mocked(confirmToolAction);
const resolveAcpPermissionRequestMock = vi.mocked(resolveAcpPermissionRequest);

describe('ToolApprovalButtons', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    acpChatFeatureFlagMock.useAcpChat = true;
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

  it('shows a stale request error when ACP has no pending request', async () => {
    resolveAcpPermissionRequestMock.mockReturnValueOnce(false);

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
    expect(confirmToolActionMock).not.toHaveBeenCalled();
    expect(screen.getByText('This approval request is no longer active.')).toBeInTheDocument();
    expect(screen.queryByText('developer__shell - Allowed once')).not.toBeInTheDocument();
  });

  it('uses the REST confirmation path when ACP chat is disabled', async () => {
    acpChatFeatureFlagMock.useAcpChat = false;
    confirmToolActionMock.mockResolvedValueOnce({ error: undefined } as Awaited<
      ReturnType<typeof confirmToolAction>
    >);

    renderWithIntl(
      <ToolApprovalButtons
        data={{
          id: 'tool-call-rest',
          toolName: 'developer__shell',
          sessionId: 'session-1',
        }}
      />
    );

    await userEvent.click(screen.getByRole('button', { name: 'Allow Once' }));

    expect(resolveAcpPermissionRequestMock).not.toHaveBeenCalled();
    expect(confirmToolActionMock).toHaveBeenCalledWith({
      body: {
        sessionId: 'session-1',
        id: 'tool-call-rest',
        action: 'allow_once',
        principalType: 'Tool',
      },
    });
    expect(screen.getByText('developer__shell - Allowed once')).toBeInTheDocument();
  });
});
