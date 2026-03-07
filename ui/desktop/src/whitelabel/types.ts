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

export interface WhiteLabelSkill {
  /** Display name shown to the agent */
  name: string;
  /** When to activate this skill */
  description: string;
  /** Path to the skill directory containing SKILL.md and supporting files */
  path: string;
}

export interface WhiteLabelTool {
  /** Tool name (used as command name) */
  name: string;
  /** What the tool does */
  description: string;
  /** Path to the CLI binary */
  path: string;
  /** Environment variables the tool needs */
  env?: Record<string, string>;
  /** Inline help text to include in the system prompt (e.g. usage docs).
   *  If omitted, the system will run `<path> --help` at build time. */
  helpText?: string;
}

/** Inline custom provider definition — registered automatically on first launch */
export interface WhiteLabelProvider {
  /** Provider ID (e.g. "custom_kgoose"). Must start with "custom_" */
  id: string;
  /** Display name in settings UI */
  displayName: string;
  /** Engine type: "openai", "anthropic", etc. */
  engine: string;
  /** Base URL for the API */
  apiUrl: string;
  /** Model names this provider supports */
  models: string[];
  /** Whether the provider requires an API key */
  requiresAuth?: boolean;
  /** Whether the provider supports streaming */
  supportsStreaming?: boolean;
  /** Custom headers to send with requests */
  headers?: Record<string, string>;
  /** Base path override (e.g. "/v1") */
  basePath?: string;
}

export interface WhiteLabelDefaults {
  /** Provider ID to use (must match providerDefinition.id if defined) */
  provider?: string;
  model?: string;
  /** Full provider definition — auto-registered on startup */
  providerDefinition?: WhiteLabelProvider;
  extensions?: WhiteLabelExtensionDefault[];
  workingDir?: string;
  systemPrompt?: string;
  goosehints?: string;
  /** Skill directories — each contains a SKILL.md with frontmatter + instructions */
  skills?: WhiteLabelSkill[];
  /** CLI tools to make available to the agent */
  tools?: WhiteLabelTool[];
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
  /** URL to call (POST) after the process is ready. The JSON response
   *  is a string→string map of environment variables to set on
   *  process.env so the agent's shell commands can use them. */
  envFromUrl?: string;
  /** Timeout in ms for the envFromUrl call (default: 120000 — 2 minutes,
   *  to allow time for interactive auth flows like OAuth). */
  envFromUrlTimeoutMs?: number;
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
