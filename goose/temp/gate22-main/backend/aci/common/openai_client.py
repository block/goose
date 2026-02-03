from openai import OpenAI

_openai_client_instance = None


def init_openai_client(api_key: str) -> None:
    """Initialize the OpenAI client when the app starts."""
    global _openai_client_instance
    if _openai_client_instance is None:
        _openai_client_instance = OpenAI(api_key=api_key)


def get_openai_client() -> OpenAI:
    """Get the OpenAI client instance. Must call init_openai_client() first."""
    if _openai_client_instance is None:
        raise RuntimeError("OpenAI client not initialized. Call init_openai_client() first.")
    return _openai_client_instance
