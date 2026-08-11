export const UPDATES_ENABLED = true;
export const COST_TRACKING_ENABLED = true;
export const ANNOUNCEMENTS_ENABLED = false;
/** Raw config editor in Settings → App. False for locked custom distros. */
export const CONFIGURATION_ENABLED = false;
/**
 * CUSTOM_DISTROS lock: packaged builds require Zitadel login and the baked
 * Avocado gateway URL. Combined with `app.isPackaged === true` this is the
 * definition of "locked" (see backendLock.ts).
 */
export const REQUIRE_ZITADEL_AUTH = true;
/** Gateway URL baked into packaged builds. Empty fails closed at startup. */
export const LOCKED_BACKEND_URL = 'https://dev.avocado.tech/agent';
/** Baked Zitadel OIDC issuer for packaged builds (no shipped .env). */
export const BAKED_ZITADEL_ISSUER = 'https://zitadel.avcd.ai';
/** Baked native PKCE client id for the Avocado Work desktop app. */
export const BAKED_ZITADEL_CLIENT_ID = '385574574122598405';
/** Baked project id (audience / role claims). */
export const BAKED_ZITADEL_PROJECT_ID = '385574573904494597';
/** Baked org id for org-scoped scopes. */
export const BAKED_ZITADEL_ORG_ID = '378278744818778119';
/** Baked Google IdP hint used during authorize. */
export const BAKED_ZITADEL_GOOGLE_IDP_ID = '382483250657951749';
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
