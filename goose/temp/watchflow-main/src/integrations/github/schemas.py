from typing import Any

from pydantic import BaseModel, ConfigDict, Field


class GitHubRepository(BaseModel):
    """Schema for GitHub repository response."""

    model_config = ConfigDict(populate_by_name=True)

    id: int
    name: str
    full_name: str
    private: bool
    owner: dict[str, Any]
    description: str | None = None
    default_branch: str = Field(default="main")
    language: str | None = None
    size: int = 0
    stargazers_count: int = Field(default=0, alias="stargazers_count")
    watchers_count: int = Field(default=0, alias="watchers_count")
    forks_count: int = Field(default=0, alias="forks_count")
    open_issues_count: int = Field(default=0, alias="open_issues_count")
    created_at: str | None = None
    updated_at: str | None = None
    pushed_at: str | None = None
