import React from 'react';
import { IntlProvider } from 'react-intl';

interface IntlTestWrapperProps {
  children: React.ReactNode;
  locale?: string;
  defaultLocale?: string;
  messages?: Record<string, string>;
}

/**
 * Wraps a component tree with IntlProvider for tests.
 * Uses English locale with no messages (defaultMessage values are used).
 */
export function IntlTestWrapper({
  children,
  locale = 'en',
  defaultLocale = 'en',
  messages = {},
}: IntlTestWrapperProps) {
  return (
    <IntlProvider locale={locale} defaultLocale={defaultLocale} messages={messages}>
      {children}
    </IntlProvider>
  );
}
