#!/usr/bin/env -S uv run --script
"""Goose SDK demo: host-driven OAuth device-code flow.

This example lists OAuth providers and starts a device-code session. It does
not open a browser and does not persist tokens — that is the host's job.

Live login is skipped unless GOOSE_OAUTH_PROVIDER is set (github_copilot or
kimi_code). CI should run without that env var.
"""
import asyncio
import os
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent.parent / "generated"))

from goose import (  # noqa: E402
    DevicePollResult,
    exchange_github_copilot_token,
    github_copilot_provider,
    kimi_code_provider,
    list_oauth_grants,
    list_oauth_providers,
    poll_device_flow,
    start_device_flow,
)


async def main() -> None:
    providers = list_oauth_providers()
    print("oauth providers:")
    for info in providers:
        print(
            f"  {info.id}: {info.name} grants={info.grants} refresh={info.supports_refresh}"
        )
    print("known grants:", list_oauth_grants())

    provider_id = os.environ.get("GOOSE_OAUTH_PROVIDER")
    if not provider_id:
        print("set GOOSE_OAUTH_PROVIDER=github_copilot|kimi_code to run a live flow")
        return

    session = await start_device_flow(provider_id)
    print(f"visit {session.verification_uri()} and enter {session.user_code()}")
    print(f"poll every {session.interval_secs()}s")
    # RFC 8628: wait `interval` before the first token poll.
    await asyncio.sleep(session.interval_secs())

    while True:
        result = await poll_device_flow(session)
        if isinstance(result, DevicePollResult.Pending):
            await asyncio.sleep(result.interval_secs)
            continue
        if isinstance(result, DevicePollResult.SlowDown):
            await asyncio.sleep(result.interval_secs)
            continue
        if isinstance(result, DevicePollResult.Success):
            print("authorized; host should persist tokens (not printed)")
            if provider_id == "github_copilot":
                copilot = await exchange_github_copilot_token(result.tokens.access_token())
                _ = github_copilot_provider(copilot.api_endpoint(), copilot.token())
                print("constructed github_copilot provider")
            elif provider_id == "kimi_code":
                _ = kimi_code_provider(result.tokens.access_token())
                print("constructed kimi_code provider")
            return
        if isinstance(result, DevicePollResult.Denied):
            raise SystemExit("authorization denied")
        if isinstance(result, DevicePollResult.Expired):
            raise SystemExit("device code expired")
        raise SystemExit(f"unexpected poll result: {result!r}")


if __name__ == "__main__":
    asyncio.run(main())
