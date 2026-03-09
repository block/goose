import type { WhiteLabelConfig } from './whitelabel/types';
import { DEFAULT_WHITELABEL_CONFIG } from './whitelabel/defaults';

function getWLConfig(): WhiteLabelConfig {
  try {
    return __WHITELABEL_CONFIG__;
  } catch {
    return DEFAULT_WHITELABEL_CONFIG;
  }
}

const _wl = getWLConfig();

export const UPDATES_ENABLED = _wl.features.updatesEnabled;
export const COST_TRACKING_ENABLED = _wl.features.costTrackingEnabled;
export const ANNOUNCEMENTS_ENABLED = _wl.features.announcementsEnabled;
export const CONFIGURATION_ENABLED = _wl.features.configurationEnabled;
export const TELEMETRY_UI_ENABLED = _wl.features.telemetryUiEnabled;
export const DICTATION_ALLOWED_PROVIDERS: string[] | null =
  _wl.features.dictationAllowedProviders ?? null;
