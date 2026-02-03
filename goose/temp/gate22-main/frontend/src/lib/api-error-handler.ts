/**
 * Centralized API error handler to reduce repetitive error parsing
 */
export async function parseApiError(response: Response, defaultMessage: string): Promise<string> {
  if (response.ok) {
    return defaultMessage;
  }

  try {
    const cloned = response.clone();
    const errorData: unknown = await cloned.json();

    if (typeof errorData === "string") {
      return errorData;
    }
    if (errorData && typeof errorData === "object") {
      const anyData = errorData as Record<string, unknown>;
      if (typeof anyData.detail === "string") return anyData.detail;
      if (typeof anyData.message === "string") return anyData.message;
      if (typeof anyData.error === "string") return anyData.error;
    }
    return defaultMessage;
  } catch {
    // If JSON parse fails, fall back to raw text from the original response
    try {
      const errorText = await response.text();
      return errorText.trim() || defaultMessage;
    } catch {
      return defaultMessage;
    }
  }
}

/**
 * Throws an error with parsed API error message
 */
export async function throwApiError(response: Response, defaultMessage: string): Promise<never> {
  let errorMessage: string = defaultMessage;

  // Read response body once as text
  let responseText: string;
  try {
    responseText = await response.text();
  } catch {
    // If reading the body fails, use default message
    throw new Error(errorMessage);
  }

  // Only process if we have non-empty text
  if (responseText && responseText.trim()) {
    try {
      // Try to parse as JSON
      const errorData: unknown = JSON.parse(responseText);

      // Extract error message from various possible fields
      if (typeof errorData === "string" && errorData.trim()) {
        errorMessage = errorData;
      } else if (errorData && typeof errorData === "object") {
        const anyData = errorData as Record<string, unknown>;

        // Check common error field names in order of preference
        if (typeof anyData.error === "string" && anyData.error.trim()) {
          errorMessage = anyData.error;
        } else if (typeof anyData.detail === "string" && anyData.detail.trim()) {
          errorMessage = anyData.detail;
        } else if (typeof anyData.message === "string" && anyData.message.trim()) {
          errorMessage = anyData.message;
        } else if (typeof anyData.msg === "string" && anyData.msg.trim()) {
          errorMessage = anyData.msg;
        } else if (typeof anyData.reason === "string" && anyData.reason.trim()) {
          errorMessage = anyData.reason;
        } else if (typeof anyData.description === "string" && anyData.description.trim()) {
          errorMessage = anyData.description;
        }

        // Handle nested error objects
        if (typeof anyData.error === "object" && anyData.error !== null) {
          const nestedError = anyData.error as Record<string, unknown>;
          if (typeof nestedError.message === "string" && nestedError.message.trim()) {
            errorMessage = nestedError.message;
          } else if (typeof nestedError.detail === "string" && nestedError.detail.trim()) {
            errorMessage = nestedError.detail;
          }
        }
      }
    } catch {
      // JSON parsing failed, use the raw text as error message
      errorMessage = responseText.trim();
    }
  }

  throw new Error(errorMessage);
}
