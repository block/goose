import type { GoslingMcpHostCapabilities } from "./mcp-apps.js";

export interface GoslingClientCapabilitiesMeta {
  gosling?: {
    mcpHostCapabilities?: GoslingMcpHostCapabilities;
    customNotifications?: boolean;
  };
}
