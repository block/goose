export * from "./generated/types.gen.js";
export * from "./generated/zod.gen.js";
export {
  GOOSE_EXT_AGENT_REQUESTS,
  GOOSE_EXT_NOTIFICATIONS,
} from "./generated/index.js";
export {
  GooseExtClient,
  type GooseClientCallbacks,
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
