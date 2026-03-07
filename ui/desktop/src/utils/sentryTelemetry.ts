import * as Sentry from '@sentry/electron/renderer';

let sentryInitialized = false;
let sentryTelemetryEnabled = false;

function ensureSentryInitialized() {
  if (sentryInitialized) return;
  sentryInitialized = true;
  Sentry.init({
    environment: import.meta.env.MODE === 'production' ? 'production' : 'development',
    beforeSend(event) {
      return sentryTelemetryEnabled ? event : null;
    },
    beforeSendTransaction(transaction) {
      return sentryTelemetryEnabled ? transaction : null;
    },
  });
}

export function setSentryTelemetryEnabled(enabled: boolean) {
  sentryTelemetryEnabled = enabled;
  if (enabled) {
    ensureSentryInitialized();
  }
}
