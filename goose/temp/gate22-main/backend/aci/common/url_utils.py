"""Utilities for working with URLs and redirect paths."""

from __future__ import annotations

from urllib.parse import urlparse


def sanitize_redirect_path(path: str | None) -> str | None:
    """Return a safe redirect path rooted at the current origin.

    The function mirrors the frontend logic by ensuring the path starts with a
    single forward slash, does not contain a scheme or different host, and
    preserves any query string or fragment components.
    """
    if not path:
        return None

    trimmed = path.strip()
    if not trimmed.startswith("/") or trimmed.startswith("//"):
        return None

    parsed = urlparse(trimmed)
    if parsed.scheme or parsed.netloc:
        return None

    sanitized = parsed.path or "/"
    if parsed.query:
        sanitized = f"{sanitized}?{parsed.query}"
    if parsed.fragment:
        sanitized = f"{sanitized}#{parsed.fragment}"

    return sanitized
