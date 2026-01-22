/**
 * goosed-sdk types
 */

export interface MessageMetadata {
    userVisible: boolean;
    agentVisible: boolean;
}

export interface TextContent {
    type: 'text';
    text: string;
}

export interface MessageContent {
    type: string;
    text?: string;
    data?: string;
    mimeType?: string;
    id?: string;
    toolCall?: Record<string, unknown>;
    toolResult?: Record<string, unknown>;
}

export interface Message {
    id?: string;
    role: 'user' | 'assistant';
    created: number;
    content: MessageContent[];
    metadata: MessageMetadata;
}

export interface TokenState {
    inputTokens: number;
    outputTokens: number;
    totalTokens: number;
    accumulatedInputTokens: number;
    accumulatedOutputTokens: number;
    accumulatedTotalTokens: number;
}

export interface ExtensionConfig {
    type: string;
    name: string;
    description?: string;
    bundled?: boolean;
}

export interface Session {
    id: string;
    name: string;
    working_dir: string;
    session_type: string;
    created_at: string;
    updated_at: string;
    user_set_name?: boolean;
    message_count?: number;
    total_tokens?: number | null;
    input_tokens?: number | null;
    output_tokens?: number | null;
    provider_name?: string | null;
    conversation?: Record<string, unknown>[] | null;
}

export interface ToolInfo {
    name: string;
    description: string;
    parameters: string[];
    permission?: string | null;
}

export interface CallToolResponse {
    content: Record<string, unknown>[];
    is_error: boolean;
}

export interface ExtensionResult {
    name: string;
    success: boolean;
}

export interface SystemInfo {
    app_version: string;
    os: string;
    os_version: string;
    architecture: string;
    provider: string;
    model: string;
    enabled_extensions: string[];
}

export type SSEEventType = 'Ping' | 'Message' | 'Finish' | 'Error' | 'ModelChange' | 'Notification';

export interface SSEEvent {
    type: SSEEventType;
    message?: Record<string, unknown>;
    token_state?: TokenState;
    reason?: string;
    error?: string;
}

export interface GoosedClientOptions {
    baseUrl?: string;
    secretKey?: string;
    timeout?: number;
}

export interface SetProviderRequest {
    provider: string;
    model: string;
}
