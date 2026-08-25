import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import Hub from './Hub';
import { IntlTestWrapper } from '../i18n/test-utils';
import type { FixedExtensionEntry } from './ConfigContext';
import { createSession } from '../sessions';

const mockSetView = vi.fn();

vi.mock('./ConfigContext', () => ({
  useConfig: () => ({
    extensionsList: [
      {
        name: 'developer',
        type: 'builtin',
        description: 'developer',
        enabled: true,
      },
      {
        name: 'memory',
        type: 'builtin',
        description: 'memory',
        enabled: false,
      },
    ] satisfies FixedExtensionEntry[],
  }),
}));

vi.mock('./ChatInput', () => ({
  default: ({
    handleSubmit,
  }: {
    handleSubmit: (input: { msg: string; images: unknown[] }) => void;
  }) => (
    <button type="button" onClick={() => handleSubmit({ msg: 'hello from hub', images: [] })}>
      Submit
    </button>
  ),
}));

vi.mock('../sessions', () => ({
  createSession: vi.fn(),
}));

describe('Hub', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    Object.defineProperty(window, 'appConfig', {
      configurable: true,
      writable: true,
      value: {
        get: (key: string) => (key === 'GOOSE_WORKING_DIR' ? '/tmp/hub-dir' : null),
      },
    });
  });

  it('navigates to pair immediately without waiting for createSession', () => {
    render(
      <IntlTestWrapper>
        <Hub setView={mockSetView} />
      </IntlTestWrapper>
    );

    fireEvent.click(screen.getByRole('button', { name: 'Submit' }));

    expect(createSession).not.toHaveBeenCalled();
    expect(mockSetView).toHaveBeenCalledWith('pair', {
      disableAnimation: true,
      initialMessage: { msg: 'hello from hub', images: [] },
      workingDir: '/tmp/hub-dir',
      allExtensions: [
        {
          name: 'developer',
          type: 'builtin',
          description: 'developer',
          enabled: true,
        },
        {
          name: 'memory',
          type: 'builtin',
          description: 'memory',
          enabled: false,
        },
      ],
    });
  });
});
