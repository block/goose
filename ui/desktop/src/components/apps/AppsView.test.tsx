/**
 * @vitest-environment jsdom
 */
import { render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import AppsView from './AppsView';
import { IntlTestWrapper } from '../../i18n/test-utils';

const mocks = vi.hoisted(() => ({
  listApps: vi.fn(),
}));

vi.mock('../../api', async () => {
  const actual = await vi.importActual<typeof import('../../api')>('../../api');

  return {
    ...actual,
    exportApp: vi.fn(),
    importApp: vi.fn(),
    listApps: mocks.listApps,
  };
});

vi.mock('../../contexts/ChatContext', () => ({
  useChatContext: () => ({
    chat: {
      sessionId: 'session-1',
    },
  }),
}));

describe('AppsView', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.listApps.mockResolvedValue({
      data: {
        apps: [
          {
            description: 'Analyze mixed IOC input and normalize results.',
            mcpServers: ['apps'],
            mimeType: 'text/html;profile=mcp-app',
            name: 'ioc-toolbox',
            text: '<html></html>',
            uri: 'ui://apps/ioc-toolbox',
          },
          {
            description: 'Classic offline encoding, decoding, and hashing workbench for analysts.',
            mcpServers: ['apps'],
            mimeType: 'text/html;profile=mcp-app',
            name: 'encode-hash-lab',
            text: '<html></html>',
            uri: 'ui://apps/encode-hash-lab',
          },
          {
            description: 'Scan logs, configs, and snippets for secrets and credentials.',
            mcpServers: ['apps'],
            mimeType: 'text/html;profile=mcp-app',
            name: 'secret-credential-scanner',
            text: '<html></html>',
            uri: 'ui://apps/secret-credential-scanner',
          },
          {
            description: 'Decode JWT structure and claims locally for quick security review.',
            mcpServers: ['apps'],
            mimeType: 'text/html;profile=mcp-app',
            name: 'jwt-inspector',
            text: '<html></html>',
            uri: 'ui://apps/jwt-inspector',
          },
          {
            description: 'User imported custom app.',
            mcpServers: ['apps'],
            mimeType: 'text/html;profile=mcp-app',
            name: 'my-custom-helper',
            text: '<html></html>',
            uri: 'ui://apps/my-custom-helper',
          },
        ],
      },
    });
  });

  it('groups built-in security tools separately from imported custom apps', async () => {
    render(
      <IntlTestWrapper>
        <AppsView />
      </IntlTestWrapper>
    );

    await waitFor(() => {
      expect(mocks.listApps).toHaveBeenCalled();
    });

    expect(await screen.findByTestId('apps-built-in-security-section')).toBeInTheDocument();
    expect(screen.getByTestId('apps-imported-custom-section')).toBeInTheDocument();

    expect(screen.getByTestId('apps-card-ioc-toolbox')).toBeInTheDocument();
    expect(screen.getByTestId('apps-card-encode-hash-lab')).toBeInTheDocument();
    expect(screen.getByTestId('apps-card-secret-credential-scanner')).toBeInTheDocument();
    expect(screen.getByTestId('apps-card-jwt-inspector')).toBeInTheDocument();
    expect(screen.getByTestId('apps-card-my-custom-helper')).toBeInTheDocument();

    expect(screen.getAllByText('Built-in security tool')).toHaveLength(4);
    expect(screen.getByText('Imported / custom app')).toBeInTheDocument();
    expect(screen.getByText('IOC triage')).toBeInTheDocument();
    expect(screen.getByText('Encoding / decoding / hashing')).toBeInTheDocument();
    expect(screen.getByText('Secret / credential review')).toBeInTheDocument();
    expect(screen.getByText('Auth token review')).toBeInTheDocument();
    expect(screen.queryByText('Header Diff Lab')).not.toBeInTheDocument();
    expect(screen.queryByText('Clock')).not.toBeInTheDocument();
    expect(screen.queryByText('Chat')).not.toBeInTheDocument();
  });
});
