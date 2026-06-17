/**
 * @vitest-environment jsdom
 */
import { afterEach, describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';

import { IntlTestWrapper } from '../../i18n/test-utils';
import { SecurityPreviewLaunchGuard } from './SecurityPreviewLaunchGuard';

function mockAppConfig(values: Record<string, unknown>) {
  (window as unknown as Record<string, unknown>).appConfig = {
    get: (key: string) => values[key],
    getAll: () => values,
  };
}

describe('SecurityPreviewLaunchGuard', () => {
  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).appConfig;
  });

  it('renders a fallback warning for unsupported packaged preview launches', () => {
    mockAppConfig({
      SECURITY_PREVIEW_SESSION_MODE: 'packaged-preview-fallback',
    });

    render(<SecurityPreviewLaunchGuard />, { wrapper: IntlTestWrapper });

    expect(
      screen.getByText('This local preview was opened outside the supported launcher.')
    ).toBeInTheDocument();
    expect(screen.getByText('./scripts/start-security-packaged-preview.sh')).toBeInTheDocument();
    expect(
      screen.getByText('pnpm --dir ui/desktop run start:packaged-preview')
    ).toBeInTheDocument();
  });

  it('stays hidden for supported packaged preview launches', () => {
    mockAppConfig({
      SECURITY_PREVIEW_SESSION_MODE: 'packaged-preview-explicit',
    });

    render(<SecurityPreviewLaunchGuard />, { wrapper: IntlTestWrapper });

    expect(
      screen.queryByText('This local preview was opened outside the supported launcher.')
    ).not.toBeInTheDocument();
  });
});
