export async function safeJsonParse<T>(
  response: Response,
  errorMessage: string = 'Failed to parse server response'
): Promise<T> {
  try {
    return (await response.json()) as T;
  } catch (error) {
    if (error instanceof SyntaxError) {
      throw new Error(errorMessage);
    }
    throw error;
  }
}

export function errorMessage(err: Error | unknown, default_value?: string) {
  if (err instanceof Error) {
    return err.message;
  } else if (typeof err === 'object' && err !== null && 'message' in err) {
    return String(err.message);
  } else {
    return default_value || String(err);
  }
}
