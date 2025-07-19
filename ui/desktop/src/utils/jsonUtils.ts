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
