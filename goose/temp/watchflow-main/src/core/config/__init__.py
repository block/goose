"""
Configuration package - unified access point.

This package provides all configuration classes and the global config instance.
"""

from src.core.config.settings import Config, config

__all__ = [
    "Config",
    "config",
]
