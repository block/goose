import { createIntl, createIntlCache } from 'react-intl';
import { describe, expect, it } from 'vitest';
import zhCN from '../i18n/messages/zh-CN.json';
import { NAV_ITEMS, SETTINGS_NAV_ITEM, getNavItemLabel } from './useNavigationItems';

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

describe('useNavigationItems zh-CN labels', () => {
  it('keeps the Security Goose primary navigation terminology stable', () => {
    const intl = createIntl(
      {
        locale: 'zh-CN',
        defaultLocale: 'zh-CN',
        messages: toIntlMessages(zhCN),
      },
      createIntlCache()
    );

    expect(getNavItemLabel(NAV_ITEMS[1], intl)).toBe('任务模板');
    expect(getNavItemLabel(NAV_ITEMS[4], intl)).toBe('自动化');
    expect(getNavItemLabel(NAV_ITEMS[6], intl)).toBe('历史会话');
    expect(getNavItemLabel(SETTINGS_NAV_ITEM, intl)).toBe('设置');
  });
});
