export const PERMISSIONS = {
  // MCP Server permissions
  MCP_SERVER_PAGE_VIEW: "mcp_server:page_view",

  CUSTOM_MCP_SERVER_CREATE: "custom_mcp_server:create",
  CUSTOM_MCP_SERVER_SYNC: "custom_mcp_server:sync_tools",
  CUSTOM_MCP_SERVER_DELETE: "custom_mcp_server:delete",
  CUSTOM_MCP_SERVER_UPDATE: "custom_mcp_server:update",

  // MCP Configuration permissions
  MCP_CONFIGURATION_PAGE_VIEW: "mcp_configuration:page_view",
  MCP_CONFIGURATION_CREATE: "mcp_configuration:create",
  MCP_CONFIGURATION_DELETE: "mcp_configuration:delete",

  // Connected Accounts permissions
  CONNECTED_ACCOUNT_PAGE_VIEW: "connected_account:page_view",
  CONNECTED_ACCOUNT_CREATE_OWN: "connected_account:create_own",
  CONNECTED_ACCOUNT_CREATE_SHARED: "connected_account:create_shared",
  CONNECTED_ACCOUNT_CREATE_OPERATIONAL: "connected_account:create_operational",
  CONNECTED_ACCOUNT_DELETE_OWN: "connected_account:delete_own",
  CONNECTED_ACCOUNT_DELETE_SHARED: "connected_account:delete_shared",
  CONNECTED_ACCOUNT_DELETE_OPERATIONAL: "connected_account:delete_operational",

  // Bundle permissions
  BUNDLE_PAGE_VIEW: "bundle:page_view",
  BUNDLE_CREATE: "bundle:create",
  BUNDLE_DELETE_OWN: "bundle:delete_own",
  BUNDLE_DELETE_ALL: "bundle:delete_all",
  BUNDLE_MCP_URL_VIEW: "bundle:mcp_url_view",

  // Subscription permissions
  SUBSCRIPTION_PAGE_VIEW: "subscription:page_view",
} as const;

export type Permission = (typeof PERMISSIONS)[keyof typeof PERMISSIONS];

export const ROLE_PERMISSIONS: Record<string, readonly Permission[]> = {
  admin: [
    // MCP Server - Admins can see all
    PERMISSIONS.MCP_SERVER_PAGE_VIEW,

    PERMISSIONS.CUSTOM_MCP_SERVER_CREATE,
    PERMISSIONS.CUSTOM_MCP_SERVER_SYNC,
    PERMISSIONS.CUSTOM_MCP_SERVER_DELETE,
    PERMISSIONS.CUSTOM_MCP_SERVER_UPDATE,

    // MCP Configuration - Admins can view, create and delete configurations
    PERMISSIONS.MCP_CONFIGURATION_PAGE_VIEW,
    PERMISSIONS.MCP_CONFIGURATION_CREATE,
    PERMISSIONS.MCP_CONFIGURATION_DELETE,

    // Connected Accounts - Admins can see all accounts
    PERMISSIONS.CONNECTED_ACCOUNT_PAGE_VIEW,
    PERMISSIONS.CONNECTED_ACCOUNT_CREATE_SHARED,
    PERMISSIONS.CONNECTED_ACCOUNT_CREATE_OPERATIONAL,
    PERMISSIONS.CONNECTED_ACCOUNT_CREATE_OWN,
    PERMISSIONS.CONNECTED_ACCOUNT_DELETE_OWN,
    PERMISSIONS.CONNECTED_ACCOUNT_DELETE_SHARED,
    PERMISSIONS.CONNECTED_ACCOUNT_DELETE_OPERATIONAL,

    // Bundles - Admins can see all and delete any bundle
    PERMISSIONS.BUNDLE_PAGE_VIEW,
    PERMISSIONS.BUNDLE_DELETE_ALL,
    // Note: Admins cannot create bundles
    // Note: Admins cannot view MCP URLs of the members

    // Subscription - Only admins can view subscription settings
    PERMISSIONS.SUBSCRIPTION_PAGE_VIEW,
  ],
  member: [
    // MCP Server - Members can only view accessible servers
    PERMISSIONS.MCP_SERVER_PAGE_VIEW,
    // Note: Members cannot configure MCP servers

    // Connected Accounts - Members see own and shared only
    PERMISSIONS.CONNECTED_ACCOUNT_PAGE_VIEW,
    PERMISSIONS.CONNECTED_ACCOUNT_CREATE_OWN,
    PERMISSIONS.CONNECTED_ACCOUNT_DELETE_OWN,

    // Bundles - Members can create and see own bundles
    PERMISSIONS.BUNDLE_PAGE_VIEW,
    PERMISSIONS.BUNDLE_CREATE,
    PERMISSIONS.BUNDLE_DELETE_OWN,
    PERMISSIONS.BUNDLE_MCP_URL_VIEW,
  ],
} as const;

export type Role = keyof typeof ROLE_PERMISSIONS;
