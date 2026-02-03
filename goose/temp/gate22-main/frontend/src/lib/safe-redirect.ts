const FALLBACK_ORIGIN = "http://localhost";

export function sanitizeRedirectPath(path: string | null | undefined): string | null {
  if (!path) {
    return null;
  }

  if (!path.startsWith("/") || path.startsWith("//")) {
    return null;
  }

  try {
    const url = new URL(path, FALLBACK_ORIGIN);
    return `${url.pathname}${url.search}${url.hash}`;
  } catch (error) {
    console.error("Failed to sanitize redirect path", error);
    return null;
  }
}
