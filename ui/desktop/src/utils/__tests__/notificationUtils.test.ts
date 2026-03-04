import { describe, expect, it } from 'vitest';

import {
  getGooseActivityFields,
  getNotificationMethod,
  getNotificationParams,
} from '@/utils/notificationUtils';

describe('notificationUtils', () => {
  it('reads method/params from direct shape', () => {
    const msg = { method: 'goose/activity', params: { phase: 'render', text: 'X' } };
    expect(getNotificationMethod(msg)).toBe('goose/activity');
    expect(getNotificationParams(msg)).toEqual({ phase: 'render', text: 'X' });
    expect(getGooseActivityFields(msg)).toEqual({ phase: 'render', text: 'X' });
  });

  it('reads method/params from nested `custom` shape', () => {
    const msg = { custom: { method: 'goose/activity', params: { phase: 'routing', text: 'Y' } } };
    expect(getNotificationMethod(msg)).toBe('goose/activity');
    expect(getNotificationParams(msg)).toEqual({ phase: 'routing', text: 'Y' });
    expect(getGooseActivityFields(msg)).toEqual({ phase: 'routing', text: 'Y' });
  });

  it('reads method/params from nested `CustomNotification` shape', () => {
    const msg = {
      CustomNotification: { method: 'goose/activity', params: { phase: 'subagent', text: 'Z' } },
    };
    expect(getNotificationMethod(msg)).toBe('goose/activity');
    expect(getNotificationParams(msg)).toEqual({ phase: 'subagent', text: 'Z' });
    expect(getGooseActivityFields(msg)).toEqual({ phase: 'subagent', text: 'Z' });
  });

  it('returns null when text is missing/empty', () => {
    const msg = { method: 'goose/activity', params: { phase: 'render', text: '' } };
    expect(getGooseActivityFields(msg)).toBeNull();
  });
});
