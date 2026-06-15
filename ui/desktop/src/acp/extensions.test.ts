import { describe, expect, it, vi } from 'vitest';

import { getConfiguredExtensions } from './extensions';

vi.mock('./acpConnection', () => ({
  getAcpClient: vi.fn(),
}));

vi.mock('../api', () => ({
  getExtensions: vi.fn(),
}));

describe('getConfiguredExtensions', () => {
  it('falls back to the REST config/extensions endpoint when ACP parsing fails', async () => {
    const { getAcpClient } = await import('./acpConnection');
    const { getExtensions } = await import('../api');

    vi.mocked(getAcpClient).mockRejectedValueOnce(new Error('ACP schema mismatch'));
    vi.mocked(getExtensions).mockResolvedValueOnce({
      data: {
        extensions: [
          {
            type: 'builtin',
            name: 'developer',
            description: 'Developer tools',
            enabled: true,
          },
        ],
        warnings: ['legacy-goosed'],
      },
    } as Awaited<ReturnType<typeof getExtensions>>);

    await expect(getConfiguredExtensions()).resolves.toEqual({
      extensions: [
        {
          type: 'builtin',
          name: 'developer',
          description: 'Developer tools',
          enabled: true,
        },
      ],
      warnings: ['legacy-goosed'],
    });
  });
});
