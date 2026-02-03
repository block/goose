from pydantic import BaseModel, Field


class WebhookSender(BaseModel):
    """GitHub webhook sender metadata."""

    login: str = Field(..., description="GitHub username of the event actor")
    id: int = Field(..., description="GitHub user ID")
    type: str = Field(..., description="Actor type: User, Organization, etc.")


class WebhookRepository(BaseModel):
    """GitHub repository metadata from webhook payload."""

    id: int = Field(..., description="GitHub repository ID")
    name: str = Field(..., description="Repository name (without owner)")
    full_name: str = Field(..., description="Owner/repo format")
    private: bool = Field(..., description="Repository visibility")
    html_url: str = Field(..., description="Public-facing URL")
    default_branch: str = Field(default="main", description="Default branch name")


class GitHubEventModel(BaseModel):
    """Standard GitHub webhook event payload structure."""

    action: str | None = Field(None, description="Event action type (e.g., 'opened', 'closed')")
    sender: WebhookSender = Field(..., description="User who triggered the event")
    repository: WebhookRepository = Field(..., description="Target repository")


class WebhookResponse(BaseModel):
    """Standardized response model for all webhook handlers."""

    status: str = Field(..., description="Processing status: success, received, error")
    detail: str | None = Field(None, description="Additional context or error message")
    event_type: str | None = Field(None, description="Normalized GitHub event type")
