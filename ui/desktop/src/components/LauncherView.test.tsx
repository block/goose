/**
 * @vitest-environment jsdom
 */
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import LauncherView from './LauncherView';
import { IntlTestWrapper } from '../i18n/test-utils';
import zhCN from '../i18n/messages/zh-CN.json';

vi.mock('../branding/productText', () => ({
  getConfiguredProductName: () => 'Security Goose',
}));

vi.mock('../utils/workingDir', () => ({
  getInitialWorkingDir: () => '/tmp/security-goose',
}));

const createChatWindow = vi.fn();
const closeWindow = vi.fn();

function toIntlMessages(
  localeMessages: Record<string, { defaultMessage?: string }>
): Record<string, string> {
  return Object.fromEntries(
    Object.entries(localeMessages).map(([messageId, descriptor]) => [
      messageId,
      descriptor.defaultMessage ?? '',
    ])
  );
}

describe('LauncherView', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    delete (window as unknown as Record<string, unknown>).appConfig;
    (window as unknown as { electron: unknown }).electron = {
      createChatWindow,
      closeWindow,
    };
  });

  it('renders the curated security task launchers', () => {
    render(
      <IntlTestWrapper>
        <LauncherView />
      </IntlTestWrapper>
    );

    expect(screen.getByText('Vulnerability Triage')).toBeInTheDocument();
    expect(screen.getByText('Alert Investigation')).toBeInTheDocument();
    expect(screen.getByText('IOC Analysis')).toBeInTheDocument();
    expect(screen.getByText('Web Investigation')).toBeInTheDocument();
    expect(screen.getByText('Report Writing')).toBeInTheDocument();
    expect(screen.getByText('WooYun-style Review')).toBeInTheDocument();
    expect(screen.getAllByText('Threat Intel').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Browser Assist').length).toBeGreaterThan(0);
    expect(screen.getAllByText('AiseeSec').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Security Gateway').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Local preview').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Disabled stub').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Blocked').length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Primary path/).length).toBeGreaterThan(0);
    expect(screen.getByText(/does not show explicit skill-load telemetry/i)).toBeInTheDocument();
  });

  it('starts recipe-backed tasks through the existing chat window launcher', async () => {
    const user = userEvent.setup();

    render(
      <IntlTestWrapper>
        <LauncherView />
      </IntlTestWrapper>
    );

    await user.click(screen.getByRole('button', { name: /Vulnerability Triage/i }));

    expect(createChatWindow).toHaveBeenCalledWith(
      expect.objectContaining({
        dir: '/tmp/security-goose',
        recipeId: 'security-vuln-triage',
        query: expect.stringContaining('vuln-triage'),
      })
    );
  });

  it('downgrades missing runtime recipes to preview and surfaces runtime warnings', () => {
    const runtimeDiagnostics = {
      sourceSkillIds: ['vuln-triage'],
      sourceRecipeIds: ['security-vuln-triage'],
      missingSkillIds: ['vuln-triage'],
      driftedSkillIds: [],
      missingRecipeIds: ['security-vuln-triage'],
      driftedRecipeIds: [],
    };

    (window as unknown as Record<string, unknown>).appConfig = {
      get: (key: string) => {
        if (key === 'SECURITY_RUNTIME_DIAGNOSTICS') {
          return runtimeDiagnostics;
        }
        if (key === 'SECURITY_PREVIEW_SESSION_MODE') {
          return 'repo-preview';
        }
        return undefined;
      },
      getAll: () => ({
        SECURITY_RUNTIME_DIAGNOSTICS: runtimeDiagnostics,
        SECURITY_PREVIEW_SESSION_MODE: 'repo-preview',
      }),
    };

    render(
      <IntlTestWrapper>
        <LauncherView />
      </IntlTestWrapper>
    );

    expect(screen.getByTestId('launcher-security-task-badge-vuln-triage')).toHaveTextContent(
      'Preview'
    );
    expect(screen.getByTestId('security-runtime-overview-notice')).toHaveTextContent(
      'Missing skills: vuln-triage'
    );
    expect(screen.getByTestId('security-task-runtime-hint-vuln-triage')).toHaveTextContent(
      'Skill asset missing'
    );
  });

  it('uses the preview starter path when launcher diagnostics say the recipe runtime is missing', async () => {
    const user = userEvent.setup();
    const runtimeDiagnostics = {
      sourceSkillIds: ['vuln-triage'],
      sourceRecipeIds: ['security-vuln-triage'],
      missingSkillIds: [],
      driftedSkillIds: [],
      missingRecipeIds: ['security-vuln-triage'],
      driftedRecipeIds: [],
    };

    (window as unknown as Record<string, unknown>).appConfig = {
      get: (key: string) => {
        if (key === 'SECURITY_RUNTIME_DIAGNOSTICS') {
          return runtimeDiagnostics;
        }
        if (key === 'SECURITY_PREVIEW_SESSION_MODE') {
          return 'repo-preview';
        }
        return undefined;
      },
      getAll: () => ({
        SECURITY_RUNTIME_DIAGNOSTICS: runtimeDiagnostics,
        SECURITY_PREVIEW_SESSION_MODE: 'repo-preview',
      }),
    };

    render(
      <IntlTestWrapper>
        <LauncherView />
      </IntlTestWrapper>
    );

    await user.click(screen.getByRole('button', { name: /Vulnerability Triage/i }));

    expect(createChatWindow).toHaveBeenCalledWith(
      expect.objectContaining({
        dir: '/tmp/security-goose',
        recipeId: undefined,
        query: expect.stringContaining('recipe runtime is unavailable in this workspace'),
      })
    );
  });

  it('keeps the zh-CN security launcher copy and template badge stable', () => {
    render(
      <IntlTestWrapper locale="zh-CN" defaultLocale="zh-CN" messages={toIntlMessages(zhCN)}>
        <LauncherView />
      </IntlTestWrapper>
    );

    expect(screen.getByText('安全任务入口')).toBeInTheDocument();
    expect(screen.getByText('漏洞研判')).toBeInTheDocument();
    expect(screen.getByText('告警分析')).toBeInTheDocument();
    expect(screen.getByText('IOC 研判')).toBeInTheDocument();
    expect(screen.getByText('网页调查')).toBeInTheDocument();
    expect(screen.getByText('报告生成')).toBeInTheDocument();
    expect(screen.getByText('业务逻辑排查')).toBeInTheDocument();
    expect(screen.getAllByText('推荐扩展').length).toBeGreaterThan(0);
    expect(screen.getAllByText('模板')).toHaveLength(6);
    expect(screen.queryByText('预览')).not.toBeInTheDocument();
  });
});
