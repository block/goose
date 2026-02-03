"""
CORS configuration.
"""

from dataclasses import dataclass


@dataclass
class CORSConfig:
    """CORS configuration."""

    headers: list[str]
    origins: list[str]
