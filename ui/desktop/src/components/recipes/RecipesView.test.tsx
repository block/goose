/**
 * @vitest-environment jsdom
 */
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import RecipesView from './RecipesView';
import { IntlTestWrapper } from '../../i18n/test-utils';
import zhCN from '../../i18n/messages/zh-CN.json';

const mocks = vi.hoisted(() => ({
  listSavedRecipes: vi.fn(),
  setView: vi.fn(),
  startNewSession: vi.fn(),
  toastSuccess: vi.fn(),
}));

vi.mock('../../recipe/recipe_management', async () => {
  const actual = await vi.importActual<typeof import('../../recipe/recipe_management')>(
    '../../recipe/recipe_management'
  );

  return {
    ...actual,
    listSavedRecipes: mocks.listSavedRecipes,
  };
});

vi.mock('../../hooks/useNavigation', () => ({
  useNavigation: () => mocks.setView,
}));

vi.mock('../../sessions', () => ({
  startNewSession: mocks.startNewSession,
}));

vi.mock('../../toasts', () => ({
  toastSuccess: mocks.toastSuccess,
  toastError: vi.fn(),
}));

vi.mock('../../utils/workingDir', () => ({
  getInitialWorkingDir: () => '/tmp/security-goose',
}));

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

function getTaskCard(title: string): HTMLElement {
  const heading = screen.getByRole('heading', { name: title });
  const card = heading.closest('[class*="p-4"]');
  if (!card) {
    throw new Error(`Task card not found for ${title}`);
  }
  return card as HTMLElement;
}

describe('RecipesView security task starters', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (window as unknown as { electron: unknown }).electron = {
      createChatWindow: vi.fn(),
      on: vi.fn(),
      off: vi.fn(),
    };
    (globalThis as { ResizeObserver?: typeof ResizeObserver }).ResizeObserver = class {
      observe() {}
      unobserve() {}
      disconnect() {}
    } as unknown as typeof ResizeObserver;
    mocks.listSavedRecipes.mockResolvedValue([
      {
        id: 'security-vuln-triage',
        file_path: '/tmp/security-goose/.goose/recipes/security-vuln-triage.yaml',
        recipe: {
          title: 'Security Vulnerability Triage',
          description: 'Triage a vulnerability',
        },
        last_modified: '2026-06-14T00:00:00.000Z',
      },
      {
        id: 'alert-investigation',
        file_path: '/tmp/security-goose/.goose/recipes/alert-investigation.yaml',
        recipe: {
          title: 'Alert Investigation',
          description: 'Investigate a security alert',
        },
        last_modified: '2026-06-14T00:00:00.000Z',
      },
      {
        id: 'web-investigation',
        file_path: '/tmp/security-goose/.goose/recipes/web-investigation.yaml',
        recipe: {
          title: 'Web Investigation',
          description: 'Inspect a suspicious page',
        },
        last_modified: '2026-06-14T00:00:00.000Z',
      },
    ]);
    mocks.startNewSession.mockResolvedValue(undefined);
  });

  it('renders recipe-backed and preview security starters with the expected mappings', async () => {
    render(
      <IntlTestWrapper>
        <RecipesView />
      </IntlTestWrapper>
    );

    await waitFor(() => {
      expect(mocks.listSavedRecipes).toHaveBeenCalledTimes(1);
    });

    const section = screen
      .getByRole('heading', { name: 'Security task starters' })
      .closest('section');

    expect(section).not.toBeNull();

    const securitySection = within(section as HTMLElement);

    expect(securitySection.getByText('security-vuln-triage')).toBeInTheDocument();
    expect(securitySection.getByText('alert-investigation')).toBeInTheDocument();
    expect(securitySection.getByText('web-investigation')).toBeInTheDocument();
    expect(securitySection.getAllByText('ioc-analysis')).toHaveLength(2);
    expect(securitySection.getByText('report-writing')).toBeInTheDocument();
    expect(securitySection.getByText('wooyun-legacy')).toBeInTheDocument();

    expect(securitySection.getAllByRole('button', { name: 'Use recipe' })).toHaveLength(3);
    expect(securitySection.getAllByRole('button', { name: 'Start guided chat' })).toHaveLength(
      3
    );
    expect(securitySection.getAllByText('Threat Intel').length).toBeGreaterThan(0);
    expect(securitySection.getAllByText('Browser Assist').length).toBeGreaterThan(0);
    expect(securitySection.getAllByText('AiseeSec').length).toBeGreaterThan(0);
    expect(securitySection.getAllByText('Security Gateway').length).toBeGreaterThan(0);
    expect(securitySection.getAllByText('Local preview').length).toBeGreaterThan(0);
    expect(securitySection.getAllByText('Disabled stub').length).toBeGreaterThan(0);
    expect(securitySection.getAllByText('Blocked').length).toBeGreaterThan(0);
  });

  it('opens the existing extensions view from the security starter section', async () => {
    const user = userEvent.setup();

    render(
      <IntlTestWrapper>
        <RecipesView />
      </IntlTestWrapper>
    );

    await waitFor(() => {
      expect(mocks.listSavedRecipes).toHaveBeenCalledTimes(1);
    });

    await user.click(screen.getByRole('button', { name: 'Open Extensions' }));

    expect(mocks.setView).toHaveBeenCalledWith('extensions');
  });

  it('starts preview-only security tasks without a recipeId and shows the preview toast', async () => {
    const user = userEvent.setup();

    (window as unknown as { electron: unknown }).electron = {
      createChatWindow: vi.fn(),
      on: vi.fn(),
      off: vi.fn(),
    };

    render(
      <IntlTestWrapper>
        <RecipesView />
      </IntlTestWrapper>
    );

    await waitFor(() => {
      expect(mocks.listSavedRecipes).toHaveBeenCalledTimes(1);
    });

    const iocCard = getTaskCard('IOC Analysis');
    await user.click(within(iocCard).getByRole('button', { name: 'Start guided chat' }));

    expect(mocks.startNewSession).toHaveBeenCalledWith(
      expect.stringContaining('IOC clues:'),
      mocks.setView,
      '/tmp/security-goose',
      { recipeId: undefined }
    );
    expect(mocks.toastSuccess).toHaveBeenCalledWith(
      expect.objectContaining({
        title: 'Preview',
      })
    );
  });

  it('opens recipe-backed tasks in a new window through the existing desktop launcher bridge', async () => {
    const user = userEvent.setup();
    const createChatWindow = vi.fn();

    (window as unknown as { electron: unknown }).electron = {
      createChatWindow,
      on: vi.fn(),
      off: vi.fn(),
    };

    render(
      <IntlTestWrapper>
        <RecipesView />
      </IntlTestWrapper>
    );

    await waitFor(() => {
      expect(mocks.listSavedRecipes).toHaveBeenCalledTimes(1);
    });

    const vulnCard = getTaskCard('Vulnerability Triage');
    await user.click(within(vulnCard).getByRole('button', { name: 'Open in new window' }));

    expect(createChatWindow).toHaveBeenCalledWith(
      expect.objectContaining({
        dir: '/tmp/security-goose',
        recipeId: 'security-vuln-triage',
        query: expect.stringContaining('security-vuln-triage'),
      })
    );
  });

  it('keeps the zh-CN recipes security section copy and task labels stable', async () => {
    render(
      <IntlTestWrapper locale="zh-CN" defaultLocale="zh-CN" messages={toIntlMessages(zhCN)}>
        <RecipesView />
      </IntlTestWrapper>
    );

    await waitFor(() => {
      expect(mocks.listSavedRecipes).toHaveBeenCalledTimes(1);
    });

    expect(screen.getByRole('heading', { name: '安全任务入口' })).toBeInTheDocument();
    expect(screen.getByText('漏洞研判')).toBeInTheDocument();
    expect(screen.getByText('告警分析')).toBeInTheDocument();
    expect(screen.getByText('IOC 研判')).toBeInTheDocument();
    expect(screen.getByText('网页调查')).toBeInTheDocument();
    expect(screen.getByText('报告生成')).toBeInTheDocument();
    expect(screen.getByText('业务逻辑排查')).toBeInTheDocument();
    expect(screen.getAllByText('推荐扩展').length).toBeGreaterThan(0);
  });
});
