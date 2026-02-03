"""
Rule evaluation utilities.

This package contains utilities used by rule validators, including
CODEOWNERS parsing, contributor analysis, and rule validation.
"""

from src.rules.utils.codeowners import (
    get_file_owners,
    is_critical_file,
    load_codeowners,
    path_has_owner,
)
from src.rules.utils.contributors import (
    get_contributor_analyzer,
    get_past_contributors,
    is_new_contributor,
)
from src.rules.utils.validation import (
    _validate_rules_yaml,
    validate_rules_yaml_from_repo,
)

__all__ = [
    "get_file_owners",
    "is_critical_file",
    "load_codeowners",
    "path_has_owner",
    "get_contributor_analyzer",
    "get_past_contributors",
    "is_new_contributor",
    "_validate_rules_yaml",
    "validate_rules_yaml_from_repo",
]

# Alias for backward compatibility
get_codeowners = load_codeowners
