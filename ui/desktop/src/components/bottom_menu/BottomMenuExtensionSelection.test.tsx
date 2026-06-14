import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, type RenderOptions, screen, waitFor } from '@testing-library/react';
import { BottomMenuExtensionSelection } from './BottomMenuExtensionSelection';
import { IntlTestWrapper } from '../../i18n/test-utils';

const renderWithIntl = (ui: React.ReactElement, options?: RenderOptions) =>
  render(ui, { wrapper: IntlTestWrapper, ...options });

let mockExtensionsList: Array<{ name: string; enabled: boolean; description?: string }> = [];
const mockGetSessionExtensions = vi.fn();

vi.mock('../ConfigContext', () => ({
  useConfig: () => ({
    extensionsList: mockExtensionsList,
  }),
}));

vi.mock('../../api', () => ({
  getSessionExtensions: (...args: unknown[]) => mockGetSessionExtensions(...args),
}));

vi.mock('../settings/extensions/agent-api', () => ({
  addToAgent: vi.fn(),
  removeFromAgent: vi.fn(),
}));

vi.mock('../../store/extensionOverrides', () => ({
  setExtensionOverride: vi.fn(),
  getExtensionOverride: () => undefined,
  getExtensionOverrides: () => new Map(),
}));

vi.mock('../settings/extensions/subcomponents/ExtensionList', () => ({
  formatExtensionName: (name: string) => name,
}));

vi.mock('../settings/extensions/utils', () => ({
  nameToKey: (name: string) => name,
}));

vi.mock('../../toasts', () => ({
  toastService: { success: vi.fn(), error: vi.fn() },
}));

// Radix dropdown doesn't open in jsdom — render children directly so the
// trigger button is always present and we can assert on its visibility.
vi.mock('../ui/dropdown-menu', () => ({
  DropdownMenu: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  DropdownMenuTrigger: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  DropdownMenuContent: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));

describe('BottomMenuExtensionSelection', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockExtensionsList = [{ name: 'developer', enabled: true }];
    mockGetSessionExtensions.mockResolvedValue({
      data: { extensions: [{ name: 'developer', enabled: true }] },
    });
  });

  it('shows the extensions trigger for an active session without opening the menu', async () => {
    renderWithIntl(<BottomMenuExtensionSelection sessionId="session-A" />);

    await waitFor(() => {
      expect(mockGetSessionExtensions).toHaveBeenCalledWith(
        expect.objectContaining({ path: { session_id: 'session-A' } })
      );
    });
    await waitFor(() => {
      expect(screen.getByTitle('manage extensions')).not.toHaveClass('invisible');
    });
  });

  it('keeps the trigger visible after switching to another session', async () => {
    const { rerender } = renderWithIntl(<BottomMenuExtensionSelection sessionId="session-A" />);
    await waitFor(() => {
      expect(screen.getByTitle('manage extensions')).not.toHaveClass('invisible');
    });

    rerender(<BottomMenuExtensionSelection sessionId="session-B" />);

    await waitFor(() => {
      expect(mockGetSessionExtensions).toHaveBeenCalledWith(
        expect.objectContaining({ path: { session_id: 'session-B' } })
      );
    });
    await waitFor(() => {
      expect(screen.getByTitle('manage extensions')).not.toHaveClass('invisible');
    });
  });
});
