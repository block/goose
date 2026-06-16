export * from "./generated/types.gen.js";
export * from "./generated/zod.gen.js";
export {
  type GooseClientCallbacks,
  type GooseExtNotifications,
} from "./generated/client.gen.js";
export { GooseClient } from "./goose-client.js";
export { createHttpStream } from "./http-stream.js";
export * from "./client-capabilities.js";
export * from "./mcp-apps.js";

export {
  ClientSideConnection,
  type Client,
  type Stream,
} from "@agentclientprotocol/sdk";
