export const UPDATES_ENABLED = window.appConfig?.get('DISTRO_UPDATES_ENABLED') ?? true;
export const COST_TRACKING_ENABLED = window.appConfig?.get('DISTRO_COST_TRACKING_ENABLED') ?? true;
export const ANNOUNCEMENTS_ENABLED =
  window.appConfig?.get('DISTRO_ANNOUNCEMENTS_ENABLED') ?? false;
export const CONFIGURATION_ENABLED =
  window.appConfig?.get('DISTRO_CONFIGURATION_ENABLED') ?? true;
export const TELEMETRY_UI_ENABLED = window.appConfig?.get('DISTRO_TELEMETRY_UI_ENABLED') ?? true;
export const DICTATION_ALLOWED_PROVIDERS: string[] | null =
  (window.appConfig?.get('DISTRO_DICTATION_ALLOWED_PROVIDERS') as string[] | null) ?? null;
