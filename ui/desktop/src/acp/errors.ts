export interface AcpCreditsExhaustedError {
  message: string;
  url?: string;
}

const CREDITS_EXHAUSTED_REASON = 'credits_exhausted';

export function parseAcpCreditsExhaustedError(error: unknown): AcpCreditsExhaustedError | null {
  const jsonRpcError = findJsonRpcError(error);
  if (!jsonRpcError || !isRecord(jsonRpcError.data)) {
    return null;
  }

  if (jsonRpcError.data.reason !== CREDITS_EXHAUSTED_REASON) {
    return null;
  }

  const url = typeof jsonRpcError.data.url === 'string' ? jsonRpcError.data.url : undefined;

  return {
    message: jsonRpcError.message,
    ...(url ? { url } : {}),
  };
}

interface JsonRpcErrorLike {
  message: string;
  data?: unknown;
}

function findJsonRpcError(error: unknown, depth = 0): JsonRpcErrorLike | null {
  if (depth > 3 || !isRecord(error)) {
    return null;
  }

  if (typeof error.message === 'string' && 'data' in error) {
    return {
      message: error.message,
      data: error.data,
    };
  }

  return findJsonRpcError(error.error, depth + 1) ?? findJsonRpcError(error.cause, depth + 1);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}
