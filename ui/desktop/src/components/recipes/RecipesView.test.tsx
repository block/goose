/**
 * @vitest-environment jsdom
 */
import { render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import RecipesView from './RecipesView';
import { IntlTestWrapper } from '../../i18n/test-utils';
import zhCN from '../../i18n/messages/zh-CN.json';

const mocks = vi.hoisted(() => ({
  listSavedRecipes: vi.fn(),
  setView: vi.fn(),
  startAgent: vi.fn(),
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

vi.mock('../../toasts', () => ({
  toastSuccess: vi.fn(),
  toastError: vi.fn(),
}));

vi.mock('../../api', async () => {
  const actual = await vi.importActual<typeof import('../../api')>('../../api');

  return {
    ...actual,
    startAgent: mocks.startAgent,
  };
});

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

describe('RecipesView built-in security recipes', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    delete (window as unknown as Record<string, unknown>).appConfig;
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
          title: 'Vulnerability Triage',
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
        id: 'ioc-analysis',
        file_path: '/tmp/security-goose/.goose/recipes/ioc-analysis.yaml',
        recipe: {
          title: 'IOC Analysis',
          description: 'Investigate IOC clues',
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
      {
        id: 'report-writing',
        file_path: '/tmp/security-goose/.goose/recipes/report-writing.yaml',
        recipe: {
          title: 'Report Writing',
          description: 'Turn findings into a report',
        },
        last_modified: '2026-06-14T00:00:00.000Z',
      },
      {
        id: 'wooyun-legacy',
        file_path: '/tmp/security-goose/.goose/recipes/wooyun-legacy.yaml',
        recipe: {
          title: 'WooYun-style Review',
          description: 'Review business workflows',
        },
        last_modified: '2026-06-14T00:00:00.000Z',
      },
    ]);
    mocks.startAgent.mockResolvedValue({
      data: {
        id: 'session-1',
        recipe: {
          prompt: 'Recipe prompt',
        },
      },
    });
  });

  it('renders the six built-in security task templates through the native saved recipes list', async () => {
    render(
      <IntlTestWrapper>
        <RecipesView />
      </IntlTestWrapper>
    );

    await waitFor(() => {
      expect(mocks.listSavedRecipes).toHaveBeenCalledTimes(1);
    });
    await screen.findByText('Vulnerability Triage');

    expect(screen.queryByRole('heading', { name: 'Security task starters' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Open Extensions' })).not.toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Saved task templates' })).toBeInTheDocument();
    expect(screen.getByText('Vulnerability Triage')).toBeInTheDocument();
    expect(screen.getByText('Alert Investigation')).toBeInTheDocument();
    expect(screen.getByText('IOC Analysis')).toBeInTheDocument();
    expect(screen.getByText('Web Investigation')).toBeInTheDocument();
    expect(screen.getByText('Report Writing')).toBeInTheDocument();
    expect(screen.getByText('WooYun-style Review')).toBeInTheDocument();
    expect(screen.getAllByRole('button', { name: 'Start task' })).toHaveLength(6);
    expect(screen.queryByRole('button', { name: 'Start guided chat' })).not.toBeInTheDocument();
    expect(screen.getByText(/does not show explicit skill-load telemetry/i)).toBeInTheDocument();
  });

  it('starts a built-in security task template with the curated starter prompt contract', async () => {
    render(
      <IntlTestWrapper>
        <RecipesView />
      </IntlTestWrapper>
    );

    await waitFor(() => {
      expect(mocks.listSavedRecipes).toHaveBeenCalledTimes(1);
    });
    await screen.findByText('Vulnerability Triage');

    screen.getAllByRole('button', { name: 'Start task' })[0].click();

    await waitFor(() => {
      expect(mocks.startAgent).toHaveBeenCalledWith(
        expect.objectContaining({
          body: expect.objectContaining({
            recipe_id: 'security-vuln-triage',
          }),
        })
      );
      expect(mocks.setView).toHaveBeenCalledWith(
        'pair',
        expect.objectContaining({
          resumeSessionId: 'session-1',
          initialMessage: {
            msg: expect.stringContaining('security-vuln-triage recipe'),
            images: [],
          },
        })
      );
    });
  });

  it('opens built-in security recipes in a new window through the native recipe action', async () => {
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
    await screen.findByText('IOC Analysis');

    const openButtons = screen.getAllByRole('button', { name: 'Open in new window' });
    openButtons[2].click();

    await waitFor(() => {
      expect(createChatWindow).toHaveBeenCalledWith(
        expect.objectContaining({
          dir: '/tmp/security-goose',
          viewType: 'pair',
          recipeId: 'ioc-analysis',
          query: expect.stringContaining('ioc-analysis recipe'),
        })
      );
    });
  });

  it('keeps the zh-CN Recipes view on the native saved task templates path', async () => {
    render(
      <IntlTestWrapper locale="zh-CN" defaultLocale="zh-CN" messages={toIntlMessages(zhCN)}>
        <RecipesView />
      </IntlTestWrapper>
    );

    await waitFor(() => {
      expect(mocks.listSavedRecipes).toHaveBeenCalledTimes(1);
    });
    await screen.findByText('IOC Analysis');

    expect(screen.queryByRole('heading', { name: '安全任务入口' })).not.toBeInTheDocument();
    expect(screen.getByRole('heading', { name: '已保存任务模板' })).toBeInTheDocument();
    expect(screen.getAllByRole('button', { name: '启动任务' })).toHaveLength(6);
    expect(screen.queryByRole('button', { name: '启动引导对话' })).not.toBeInTheDocument();
    expect(screen.getByText(/还不能显式显示技能加载遥测/)).toBeInTheDocument();
  });
});
