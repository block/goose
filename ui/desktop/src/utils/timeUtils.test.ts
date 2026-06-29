import { afterEach, describe, expect, it, vi } from 'vitest';
import { formatMessageTimestamp, getMessageTimeFormatOptions } from './timeUtils';

const originalDateTimeFormat = Intl.DateTimeFormat;

function mockSystemHourCycle(hourCycle?: Intl.DateTimeFormatOptions['hourCycle']) {
  vi.spyOn(Intl, 'DateTimeFormat').mockImplementation(function (
    locales?: Intl.LocalesArgument,
    options?: Intl.DateTimeFormatOptions
  ) {
    if (locales === undefined && options?.hour === 'numeric') {
      return {
        resolvedOptions: () => ({ hourCycle }),
      } as unknown as Intl.DateTimeFormat;
    }

    return new originalDateTimeFormat(locales, options);
  } as typeof Intl.DateTimeFormat);
}

describe('timeUtils', () => {
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('uses the system hour cycle for message time options', () => {
    mockSystemHourCycle('h23');

    expect(getMessageTimeFormatOptions()).toEqual({
      hour: 'numeric',
      minute: '2-digit',
      hourCycle: 'h23',
    });
  });

  it('omits hourCycle when Intl does not expose a system preference', () => {
    mockSystemHourCycle();

    expect(getMessageTimeFormatOptions()).toEqual({
      hour: 'numeric',
      minute: '2-digit',
    });
  });

  it('formats timestamps with the resolved system hour cycle', () => {
    mockSystemHourCycle('h24');
    const toLocaleTimeString = vi
      .spyOn(Date.prototype, 'toLocaleTimeString')
      .mockReturnValue('13:05');

    expect(formatMessageTimestamp(Date.now() / 1000)).toBe('13:05');
    expect(toLocaleTimeString).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({
        hour: 'numeric',
        minute: '2-digit',
        hourCycle: 'h24',
      })
    );
  });
});
