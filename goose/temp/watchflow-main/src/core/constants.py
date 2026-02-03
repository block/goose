"""
Application-wide constants.
"""

# Default team memberships for rule validation
# TODO: In production, these should be fetched from an external provider or DB
DEFAULT_TEAM_MEMBERSHIPS: dict[str, list[str]] = {
    "devops": ["devops-user", "admin-user"],
    "codeowners": ["senior-dev", "tech-lead"],
}
