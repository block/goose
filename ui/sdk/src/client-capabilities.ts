import type { GooseMcpHostCapabilities } from "./mcp-apps.js";

export interface GooseTerminalShellCapabilities {
  executable: string;
  argsPrefix: string[];
}

export interface GooseClientCapabilitiesMeta {
  goose?: {
    mcpHostCapabilities?: GooseMcpHostCapabilities;
    terminalShell?: GooseTerminalShellCapabilities;
    customNotifications?: boolean;
  };
}
