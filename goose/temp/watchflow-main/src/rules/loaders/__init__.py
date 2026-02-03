"""
Rule loaders package.

This package contains implementations of the RuleLoader interface
for loading rules from different sources (GitHub, database, etc.).
"""

from src.rules.loaders.github_loader import (
    GitHubRuleLoader,
    RulesFileNotFoundError,
    github_rule_loader,
)

__all__ = [
    "GitHubRuleLoader",
    "RulesFileNotFoundError",
    "github_rule_loader",
]
