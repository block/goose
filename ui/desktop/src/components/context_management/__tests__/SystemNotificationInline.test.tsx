import { describe, expect, it, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import i18n from '../../../i18n';
import { SystemNotificationInline } from '../SystemNotificationInline';

describe('SystemNotificationInline', () => {
  beforeEach(async () => {
    await i18n.changeLanguage('en');
  });

  it('localizes compaction completion message and marks success', async () => {
    await i18n.changeLanguage('zh-Hans');

    render(
      <SystemNotificationInline
        notification={{ notificationType: 'inlineMessage', msg: 'Compaction complete' }}
      />
    );

    const marker = screen.getByTestId('compaction-success-marker');
    expect(marker).toHaveTextContent('压缩完成');
  });

  it('localizes auto-compact threshold interpolation', async () => {
    await i18n.changeLanguage('zh-Hant');

    render(
      <SystemNotificationInline
        notification={{
          notificationType: 'inlineMessage',
          msg: 'Exceeded auto-compact threshold of 70%. Performing auto-compaction...',
        }}
      />
    );

    expect(screen.getByText('已超過自動壓縮門檻 70%，正在自動壓縮...')).toBeInTheDocument();
  });

  it('keeps unknown notifications as-is', () => {
    render(
      <SystemNotificationInline
        notification={{ notificationType: 'inlineMessage', msg: 'Some unknown backend message' }}
      />
    );

    const marker = screen.getByTestId('system-inline-notification');
    expect(marker).toHaveTextContent('Some unknown backend message');
  });

  it('marks compaction failures with error test id', () => {
    render(
      <SystemNotificationInline
        notification={{ notificationType: 'inlineMessage', msg: 'Compaction failed: boom' }}
      />
    );

    const marker = screen.getByTestId('compaction-error-marker');
    expect(marker).toHaveTextContent('Compaction failed: boom');
  });
});
