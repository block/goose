/**
 * JSON-RPC 2.0 type definitions for internal use.
 */
export type AnyMessage = AnyRequest | AnyResponse | AnyNotification;
export type AnyRequest = {
    jsonrpc: "2.0";
    id: string | number | null;
    method: string;
    params?: unknown;
};
export type AnyResponse = {
    jsonrpc: "2.0";
    id: string | number | null;
} & Result<unknown>;
export type AnyNotification = {
    jsonrpc: "2.0";
    method: string;
    params?: unknown;
};
export type Result<T> = {
    result: T;
} | {
    error: ErrorResponse;
};
export type ErrorResponse = {
    code: number;
    message: string;
    data?: unknown;
};
export type PendingResponse = {
    resolve: (response: unknown) => void;
    reject: (error: ErrorResponse) => void;
};
export type RequestHandler = (method: string, params: unknown) => Promise<unknown>;
export type NotificationHandler = (method: string, params: unknown) => Promise<void>;
