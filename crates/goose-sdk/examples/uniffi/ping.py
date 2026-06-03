"""Minimal Goose SDK demo: ping the SDK and print the pong."""

from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent.parent / "generated"))

from aaif_goose import Client  # noqa: E402


def main() -> None:
    client = Client()
    pong = client.ping("aaif.io")
    print(pong.message)


if __name__ == "__main__":
    main()
