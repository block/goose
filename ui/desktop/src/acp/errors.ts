export interface AcpCreditsExhaustedError {
  message: string;
  url?: string;
}

const CREDITS_EXHAUSTED_REASON = 'credits_exhausted';

// Kept in sync with RECIPE_PARAMS_CANCELLED_REASON in crates/goose/src/acp/server/recipe.rs.
const RECIPE_PARAMS_CANCELLED_REASON = 'recipe_params_cancelled';

export function isRecipeParamsCancelled(error: unknown): boolean {
  return asAcpJsonRpcError(error)?.data?.reason === RECIPE_PARAMS_CANCELLED_REASON;
}

export function parseAcpCreditsExhaustedError(error: unknown): AcpCreditsExhaustedError | null {
  const jsonRpcError = asAcpJsonRpcError(error);
  if (jsonRpcError?.data?.reason !== CREDITS_EXHAUSTED_REASON) {
    return null;
  }

  const url = typeof jsonRpcError.data.url === 'string' ? jsonRpcError.data.url : undefined;

  return {
    message: jsonRpcError.message,
    ...(url ? { url } : {}),
  };
}

interface AcpJsonRpcError {
  message: string;
  data: Record<string, unknown>;
}

function asAcpJsonRpcError(error: unknown): AcpJsonRpcError | null {
  if (!isRecord(error)) {
    return null;
  }

  const candidate = isRecord(error.error) ? error.error : error;
  if (typeof candidate.message !== 'string' || !isRecord(candidate.data)) {
    return null;
  }

  return {
    message: candidate.message,
    data: candidate.data,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}
