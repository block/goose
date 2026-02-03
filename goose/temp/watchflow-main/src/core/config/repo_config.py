"""
Repository configuration.
"""

from dataclasses import dataclass


@dataclass
class RepoConfig:
    """Repository configuration."""

    base_path: str = ".watchflow"
    rules_file: str = "rules.yaml"
