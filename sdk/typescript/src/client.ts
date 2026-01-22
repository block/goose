/**
 * goosed-sdk client
 */

import type {
    Session,
    ToolInfo,
    CallToolResponse,
    SSEEvent,
    SystemInfo,
    ExtensionResult,
    GoosedClientOptions,
} from './types.js';

export class GoosedException extends Error {
    statusCode?: number;

    constructor(message: string, statusCode?: number) {
        super(message);
        this.name = 'GoosedException';
        this.statusCode = statusCode;
    }
}

export class GoosedAuthError extends GoosedException {
    constructor(message = 'Authentication failed') {
        super(message, 401);
        this.name = 'GoosedAuthError';
    }
}

export class GoosedNotFoundError extends GoosedException {
    constructor(message = 'Resource not found') {
        super(message, 404);
        this.name = 'GoosedNotFoundError';
    }
}

export class GoosedAgentNotInitializedError extends GoosedException {
    constructor(message = 'Agent not initialized') {
        super(message, 424);
        this.name = 'GoosedAgentNotInitializedError';
    }
}

export class GoosedServerError extends GoosedException {
    constructor(message = 'Server error') {
        super(message, 500);
        this.name = 'GoosedServerError';
    }
}

export class GoosedConnectionError extends GoosedException {
    constructor(message = 'Connection error') {
        super(message);
        this.name = 'GoosedConnectionError';
    }
}

export class GoosedClient {
    private baseUrl: string;
    private secretKey: string;
    private timeout: number;

    constructor(options: GoosedClientOptions = {}) {
        this.baseUrl = (options.baseUrl ?? 'http://127.0.0.1:3000').replace(/\/$/, '');
        this.secretKey = options.secretKey ?? 'test';
        this.timeout = options.timeout ?? 30000;
    }

    private headers(): Record<string, string> {
        return {
            'Content-Type': 'application/json',
            'x-secret-key': this.secretKey,
        };
    }

    private async handleResponse<T>(response: Response): Promise<T> {
        if (response.ok) {
            const contentType = response.headers.get('content-type') ?? '';
            if (contentType.includes('application/json')) {
                const text = await response.text();
                if (text === '') {
                    return undefined as T;
                }
                return JSON.parse(text) as T;
            }
            const text = await response.text();
            if (text === '') {
                return undefined as T;
            }
            return text as unknown as T;
        }

        const text = await response.text();
        switch (response.status) {
            case 401:
                throw new GoosedAuthError();
            case 404:
                throw new GoosedNotFoundError();
            case 424:
                throw new GoosedAgentNotInitializedError();
            default:
                if (response.status >= 500) {
                    throw new GoosedServerError(text);
                }
                throw new GoosedException(`HTTP ${response.status}: ${text}`, response.status);
        }
    }

    private async get<T>(path: string, params?: Record<string, string>): Promise<T> {
        const url = new URL(`${this.baseUrl}${path}`);
        if (params) {
            Object.entries(params).forEach(([key, value]) => {
                url.searchParams.append(key, value);
            });
        }

        try {
            const response = await fetch(url.toString(), {
                method: 'GET',
                headers: this.headers(),
                signal: AbortSignal.timeout(this.timeout),
            });
            return this.handleResponse<T>(response);
        } catch (error) {
            if (error instanceof TypeError) {
                throw new GoosedConnectionError(error.message);
            }
            if (error instanceof DOMException && error.name === 'AbortError') {
                throw new GoosedConnectionError('Request timed out');
            }
            throw error;
        }
    }

    private async post<T>(path: string, body?: Record<string, unknown>): Promise<T> {
        try {
            const response = await fetch(`${this.baseUrl}${path}`, {
                method: 'POST',
                headers: this.headers(),
                body: body ? JSON.stringify(body) : undefined,
                signal: AbortSignal.timeout(this.timeout),
            });
            return this.handleResponse<T>(response);
        } catch (error) {
            if (error instanceof TypeError) {
                throw new GoosedConnectionError(error.message);
            }
            if (error instanceof DOMException && error.name === 'AbortError') {
                throw new GoosedConnectionError('Request timed out');
            }
            throw error;
        }
    }

    private async put<T>(path: string, body?: Record<string, unknown>): Promise<T> {
        try {
            const response = await fetch(`${this.baseUrl}${path}`, {
                method: 'PUT',
                headers: this.headers(),
                body: body ? JSON.stringify(body) : undefined,
                signal: AbortSignal.timeout(this.timeout),
            });
            return this.handleResponse<T>(response);
        } catch (error) {
            if (error instanceof TypeError) {
                throw new GoosedConnectionError(error.message);
            }
            if (error instanceof DOMException && error.name === 'AbortError') {
                throw new GoosedConnectionError('Request timed out');
            }
            throw error;
        }
    }

    private async delete<T>(path: string): Promise<T> {
        try {
            const response = await fetch(`${this.baseUrl}${path}`, {
                method: 'DELETE',
                headers: this.headers(),
                signal: AbortSignal.timeout(this.timeout),
            });
            return this.handleResponse<T>(response);
        } catch (error) {
            if (error instanceof TypeError) {
                throw new GoosedConnectionError(error.message);
            }
            if (error instanceof DOMException && error.name === 'AbortError') {
                throw new GoosedConnectionError('Request timed out');
            }
            throw error;
        }
    }

    // === Status APIs ===

    async status(): Promise<string> {
        return this.get<string>('/status');
    }

    async systemInfo(): Promise<SystemInfo> {
        return this.get<SystemInfo>('/system_info');
    }

    // === Agent APIs ===

    async startSession(workingDir: string): Promise<Session> {
        return this.post<Session>('/agent/start', { working_dir: workingDir });
    }

    async resumeSession(
        sessionId: string,
        loadModelAndExtensions = true
    ): Promise<{ session: Session; extensionResults: ExtensionResult[] }> {
        const data = await this.post<{ session: Session; extension_results: ExtensionResult[] }>(
            '/agent/resume',
            { session_id: sessionId, load_model_and_extensions: loadModelAndExtensions }
        );
        return {
            session: data.session,
            extensionResults: data.extension_results ?? [],
        };
    }

    async restartSession(sessionId: string): Promise<ExtensionResult[]> {
        const data = await this.post<{ extension_results: ExtensionResult[] }>(
            '/agent/restart',
            { session_id: sessionId }
        );
        return data.extension_results ?? [];
    }

    async stopSession(sessionId: string): Promise<void> {
        await this.post('/agent/stop', { session_id: sessionId });
    }

    async getTools(sessionId: string, extensionName?: string): Promise<ToolInfo[]> {
        const params: Record<string, string> = { session_id: sessionId };
        if (extensionName) {
            params.extension_name = extensionName;
        }
        return this.get<ToolInfo[]>('/agent/tools', params);
    }

    async callTool(
        sessionId: string,
        name: string,
        args: Record<string, unknown>
    ): Promise<CallToolResponse> {
        return this.post<CallToolResponse>('/agent/call_tool', {
            session_id: sessionId,
            name,
            arguments: args,
        });
    }

    // === Chat APIs ===

    async *sendMessage(sessionId: string, text: string): AsyncGenerator<SSEEvent> {
        const message = {
            role: 'user',
            created: Math.floor(Date.now() / 1000),
            content: [{ type: 'text', text }],
            metadata: { userVisible: true, agentVisible: true },
        };

        let response: Response;
        try {
            response = await fetch(`${this.baseUrl}/reply`, {
                method: 'POST',
                headers: this.headers(),
                body: JSON.stringify({
                    session_id: sessionId,
                    user_message: message,
                }),
                signal: AbortSignal.timeout(this.timeout),
            });
        } catch (error) {
            if (error instanceof TypeError) {
                throw new GoosedConnectionError(error.message);
            }
            if (error instanceof DOMException && error.name === 'AbortError') {
                throw new GoosedConnectionError('Request timed out');
            }
            throw error;
        }

        if (!response.ok) {
            await this.handleResponse(response);
        }

        const reader = response.body?.getReader();
        if (!reader) {
            throw new GoosedException('No response body');
        }

        const decoder = new TextDecoder();
        let buffer = '';
        let dataLines: string[] = [];

        while (true) {
            const { done, value } = await reader.read();
            if (done) break;

            buffer += decoder.decode(value, { stream: true });
            const lines = buffer.split('\n');
            buffer = lines.pop() ?? '';

            for (const line of lines) {
                const trimmed = line.replace(/\r$/, '');
                if (trimmed === '') {
                    if (dataLines.length > 0) {
                        const data = JSON.parse(dataLines.join('\n')) as SSEEvent;
                        dataLines = [];
                        yield data;
                    }
                    continue;
                }
                if (trimmed.startsWith('data:')) {
                    dataLines.push(trimmed.slice(5).trimStart());
                }
            }
        }

        if (dataLines.length > 0) {
            const data = JSON.parse(dataLines.join('\n')) as SSEEvent;
            yield data;
        }
    }

    async chat(sessionId: string, text: string): Promise<string> {
        let responseText = '';
        for await (const event of this.sendMessage(sessionId, text)) {
            if (event.type === 'Message' && event.message) {
                const content = event.message.content as Array<{ type: string; text?: string }>;
                for (const c of content ?? []) {
                    if (c.type === 'text' && c.text) {
                        responseText += c.text;
                    }
                }
            } else if (event.type === 'Error') {
                throw new GoosedException(event.error ?? 'Unknown error');
            }
        }
        return responseText;
    }

    // === Session APIs ===

    async listSessions(): Promise<Session[]> {
        const data = await this.get<{ sessions: Session[] }>('/sessions');
        return data.sessions ?? [];
    }

    async getSession(sessionId: string): Promise<Session> {
        return this.get<Session>(`/sessions/${sessionId}`);
    }

    async updateSessionName(sessionId: string, name: string): Promise<void> {
        await this.put(`/sessions/${sessionId}/name`, { name });
    }

    async deleteSession(sessionId: string): Promise<void> {
        await this.delete(`/sessions/${sessionId}`);
    }

    async exportSession(sessionId: string): Promise<string> {
        return this.get<string>(`/sessions/${sessionId}/export`);
    }


}
