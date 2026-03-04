import * as Sentry from '@sentry/electron/renderer';

let sentryTelemetryEnabled = false;

Sentry.init({
  environment: import.meta.env.MODE === 'production' ? 'production' : 'development',
  beforeSend(event) {
    return sentryTelemetryEnabled ? event : null;
  },
  beforeSendTransaction(transaction) {
    return sentryTelemetryEnabled ? transaction : null;
  },
});

export function setSentryTelemetryEnabled(enabled: boolean) {
  sentryTelemetryEnabled = enabled;
}
