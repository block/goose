import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { BottomMenuModeSelection } from './BottomMenuModeSelection';

let mockConfig: Record<string, unknown> = {};
const mockUpsert = vi.fn();
const mockUpdateSession = vi.fn().mockResolvedValue({});

vi.mock('../ConfigContext', () => ({
  useConfig: () => ({
    config: mockConfig,
    upsert: mockUpsert,
  }),
}));

vi.mock('../../utils/analytics', () => ({
  trackModeChanged: vi.fn(),
}));

vi.mock('../../api', () => ({
  updateSession: (...args: unknown[]) => mockUpdateSession(...args),
}));

// Radix dropdown doesn't open in jsdom — render children directly
vi.mock('../ui/dropdown-menu', () => ({
  DropdownMenu: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  DropdownMenuTrigger: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  DropdownMenuContent: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  DropdownMenuItem: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));

describe('BottomMenuModeSelection', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockConfig = {};
  });

  it('displays mode from config', async () => {
    mockConfig.GOOSE_MODE = 'approve';
    render(<BottomMenuModeSelection sessionId={null} />);
    await waitFor(() => {
      expect(screen.getByText('manual')).toBeInTheDocument();
    });
  });

  it('defaults to auto when config has no mode', async () => {
    mockConfig.GOOSE_MODE = undefined;
    render(<BottomMenuModeSelection sessionId={null} />);
    await waitFor(() => {
      expect(screen.getByText('autonomous')).toBeInTheDocument();
    });
  });

  it('calls updateSession when sessionId is present', async () => {
    mockConfig.GOOSE_MODE = 'auto';
    render(<BottomMenuModeSelection sessionId="test-session-123" />);

    fireEvent.click(screen.getByText('Manual'));

    await waitFor(() => {
      expect(mockUpdateSession).toHaveBeenCalledWith({
        body: { session_id: 'test-session-123', goose_mode: 'approve' },
      });
    });
    expect(mockUpsert).toHaveBeenCalledWith('GOOSE_MODE', 'approve', false);
  });

  it('does not call updateSession when sessionId is null', async () => {
    mockConfig.GOOSE_MODE = 'auto';
    render(<BottomMenuModeSelection sessionId={null} />);

    fireEvent.click(screen.getByText('Manual'));

    await waitFor(() => {
      expect(mockUpsert).toHaveBeenCalledWith('GOOSE_MODE', 'approve', false);
    });
    expect(mockUpdateSession).not.toHaveBeenCalled();
  });
});
