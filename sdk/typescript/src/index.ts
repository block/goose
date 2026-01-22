/**
 * goosed-sdk - TypeScript SDK for goosed API
 */

export { GoosedClient } from './client.js';
export {
    GoosedException,
    GoosedAuthError,
    GoosedNotFoundError,
    GoosedAgentNotInitializedError,
    GoosedServerError,
    GoosedConnectionError,
} from './client.js';
export type {
    Session,
    Message,
    MessageContent,
    MessageMetadata,
    ToolInfo,
    CallToolResponse,
    SSEEvent,
    SSEEventType,
    TokenState,
    SystemInfo,
    ExtensionResult,
    GoosedClientOptions,
} from './types.js';
