/**
 * White-label configuration types.
 * These define the shape of whitelabel.yaml and are available at both
 * build time (Vite plugin) and runtime (React context).
 */

export interface WhiteLabelStarterPrompt {
  icon: string;
  label: string;
  prompt: string;
}

export interface WhiteLabelBranding {
  appName: string;
  tagline?: string;
  logo?: string;
  logoSmall?: string;
  trayIcon?: string;
  greetings: string[];
  starterPrompts?: WhiteLabelStarterPrompt[];
}

export interface WhiteLabelFeatures {
  updatesEnabled: boolean;
  costTrackingEnabled: boolean;
  announcementsEnabled: boolean;
  configurationEnabled: boolean;
  telemetryUiEnabled: boolean;
  navigation: string[];
  settingsTabs: string[];
  hiddenSettingSections?: string[];
  allowedProviders?: string[];
  dictationAllowedProviders?: string[] | null;
}

export interface WhiteLabelExtensionDefault {
  name: string;
  type: string;
  uri?: string;
  cmd?: string;
  args?: string[];
  enabled: boolean;
  envVars?: Record<string, string>;
}

export interface WhiteLabelDefaults {
  provider?: string;
  model?: string;
  extensions?: WhiteLabelExtensionDefault[];
  workingDir?: string;
  systemPrompt?: string;
  goosehints?: string;
}

export interface WhiteLabelProcess {
  name: string;
  command: string;
  args?: string[];
  cwd?: string;
  env?: Record<string, string>;
  restartOnCrash?: boolean;
  waitForPort?: number;
  waitTimeoutMs?: number;
}

export interface WhiteLabelWindow {
  width: number;
  height: number;
  minWidth: number;
  alwaysOnTop?: boolean;
  resizable?: boolean;
}

export interface WhiteLabelConfig {
  branding: WhiteLabelBranding;
  features: WhiteLabelFeatures;
  defaults: WhiteLabelDefaults;
  processes?: WhiteLabelProcess[];
  window: WhiteLabelWindow;
}
