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

  it('keeps the zh-CN security launcher copy and recipe/preview badge split stable', () => {
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
    expect(screen.getAllByText('Recipe')).toHaveLength(3);
    expect(screen.getAllByText('预览')).toHaveLength(3);
  });
});
