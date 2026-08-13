import { describe, expect, it, vi } from 'vitest';
import { clearMacQuarantine } from './macQuarantine';

describe('clearMacQuarantine', () => {
  it('returns false when the path is empty', () => {
    const runXattr = vi.fn();
    expect(clearMacQuarantine('', runXattr)).toBe(false);
    expect(runXattr).not.toHaveBeenCalled();
  });

  it('strips quarantine from the goose binary and the .app bundle', () => {
    if (process.platform !== 'darwin') {
      expect(clearMacQuarantine('/tmp/goose')).toBe(false);
      return;
    }

    const runXattr = vi.fn().mockReturnValue({ status: 0 });
    expect(
      clearMacQuarantine(
        '/Applications/Avocado Work.app/Contents/Resources/bin/goose',
        runXattr
      )
    ).toBe(true);
    expect(runXattr).toHaveBeenCalledWith([
      '-cr',
      '/Applications/Avocado Work.app/Contents/Resources/bin/goose',
    ]);
    expect(runXattr).toHaveBeenCalledWith(['-cr', '/Applications/Avocado Work.app']);
    expect(runXattr).toHaveBeenCalledTimes(2);
  });
});
