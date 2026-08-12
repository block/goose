export const UPDATES_ENABLED = true;
export const COST_TRACKING_ENABLED = true;
export const ANNOUNCEMENTS_ENABLED = false;
/** Raw config editor in Settings → App. False for locked custom distros. */
export const CONFIGURATION_ENABLED = false;
export const TELEMETRY_UI_ENABLED = true;
export const DICTATION_ALLOWED_PROVIDERS: string[] | null = null;
/**
 * Sidebar Apps gallery, standalone app windows, and app launch IPC.
 * Does NOT gate inline MCP App UIs in chat (e.g. Autovisualiser charts).
 * Set true to restore the Apps gallery.
 */
export const APPS_UI_ENABLED = false;
/**
 * When false, users cannot add/remove/configure LLM providers (CUSTOM_DISTROS lockdown).
 * Model switching among GOOSE_PREDEFINED_MODELS remains available.
 * Set true to restore full provider management UI.
 */
export const PROVIDER_MANAGEMENT_ENABLED = false;
/**
 * When true, show the Extensions / Connections page and nav so users can toggle
 * curated connectors (e.g. Google Workspace) and complete MCP OAuth.
 * Set false to hide the page entirely.
 */
export const EXTENSIONS_UI_ENABLED = true;
/**
 * When false, block marketplace install paths: "Add custom extension", browse
 * directory, and deeplink ExtensionInstallModal. Curated toggles still work.
 * Set true to restore full Extensions marketplace install UI.
 */
export const EXTENSIONS_INSTALL_ENABLED = false;
/**
 * When true, sync the hosted Google Workspace streamable_http connector from
 * bundled-extensions.json (OAuth 2.1 against https://dev.avocado.tech/google-workspace/mcp).
 * Set false to skip registering that connector for local/offline builds.
 */
export const GOOGLE_WORKSPACE_ENABLED = true;
/**
 * When true, show the Git worktrees section in the directory switcher.
 * False for non-developer consumer distros.
 */
export const GIT_WORKTREES_UI_ENABLED = false;
