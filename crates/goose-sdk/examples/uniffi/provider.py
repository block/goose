#!/usr/bin/env -S uv run --script
"""Goose SDK demo: build a declarative provider and stream a completion."""
import asyncio
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent.parent / "generated"))

from goose import (  # noqa: E402
    MessageRole,
    ProviderMessage,
    ProviderModelConfig,
    declarative_provider_from_json,
)


async def main() -> None:
    provider = declarative_provider_from_json((HERE.parent / "deepseek.json").read_text())
    model = ProviderModelConfig(model_name="deepseek-v4-flash")
    messages = [ProviderMessage(role=MessageRole.USER, text="what is the capital of France?")]
    stream = await provider.stream(
        model,
        "You are a knowledgable geography expert",
        messages,
    )

    while chunk := await stream.next():
        if chunk.text:
            print(chunk.text, end="")
        if chunk.usage_json:
            usage = json.loads(chunk.usage_json)
            print(f"\nusage: {usage}")
    print()


if __name__ == "__main__":
    asyncio.run(main())
