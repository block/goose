from __future__ import annotations

from datetime import datetime
from uuid import UUID, uuid4

from pgvector.sqlalchemy import Vector
from sqlalchemy import (
    Boolean,
    DateTime,
    ForeignKey,
    ForeignKeyConstraint,
    Index,
    Integer,
    String,
    Text,
    UniqueConstraint,
    false,
    func,
)
from sqlalchemy import Enum as SQLEnum
from sqlalchemy.dialects.postgresql import ARRAY, JSONB
from sqlalchemy.dialects.postgresql import UUID as PGUUID
from sqlalchemy.orm import DeclarativeBase, Mapped, MappedAsDataclass, mapped_column, relationship
from sqlalchemy.types import TEXT

from aci.common.enums import (
    AuthType,
    ConnectedAccountOwnership,
    MCPServerTransportType,
    MCPToolCallStatus,
    OrganizationInvitationStatus,
    OrganizationRole,
    TeamRole,
    UserIdentityProvider,
    UserVerificationType,
)

EMBEDDING_DIMENSION = 1024
MAX_STRING_LENGTH = 512
MAX_ENUM_LENGTH = 50

BUNDLE_KEY_LENGTH = 36


class Base(MappedAsDataclass, DeclarativeBase):
    pass


class User(Base):
    __tablename__ = "users"

    id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), primary_key=True, default_factory=uuid4, init=False
    )
    # TODO: should the same email but from different providers be considered the same user
    # (e.g., google and github)?
    identity_provider: Mapped[UserIdentityProvider] = mapped_column(
        SQLEnum(UserIdentityProvider, native_enum=False, length=MAX_ENUM_LENGTH), nullable=False
    )
    email: Mapped[str] = mapped_column(String(MAX_STRING_LENGTH), unique=True, nullable=False)
    email_verified: Mapped[bool] = mapped_column(Boolean, server_default=false(), nullable=False)
    name: Mapped[str] = mapped_column(String(MAX_STRING_LENGTH), nullable=False)
    password_hash: Mapped[str | None] = mapped_column(String(MAX_STRING_LENGTH), nullable=True)
    last_login_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), nullable=False, init=False
    )

    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), nullable=False, init=False
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True),
        server_default=func.now(),
        onupdate=func.now(),
        nullable=False,
        init=False,
    )
    deleted_at: Mapped[datetime | None] = mapped_column(
        DateTime(timezone=True), nullable=True, init=False
    )

    organization_memberships: Mapped[list[OrganizationMembership]] = relationship(
        back_populates="user", cascade="all", passive_deletes=True, init=False
    )
    team_memberships: Mapped[list[TeamMembership]] = relationship(
        back_populates="user", cascade="all", passive_deletes=True, init=False
    )
    refresh_tokens: Mapped[list[UserRefreshToken]] = relationship(
        back_populates="user", cascade="all, delete-orphan", init=False
    )


class UserRefreshToken(Base):
    __tablename__ = "user_refresh_tokens"
    id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), primary_key=True, default_factory=uuid4, init=False
    )
    user_id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), ForeignKey("users.id", ondelete="CASCADE"), nullable=False
    )
    token_hash: Mapped[str] = mapped_column(String(MAX_STRING_LENGTH), nullable=True)
    expires_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), nullable=False)
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), nullable=False, init=False
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True),
        server_default=func.now(),
        onupdate=func.now(),
        nullable=False,
        init=False,
    )
    deleted_at: Mapped[datetime | None] = mapped_column(
        DateTime(timezone=True), nullable=True, init=False
    )
    user: Mapped[User] = relationship(back_populates="refresh_tokens", init=False)


class UserVerification(Base):
    __tablename__ = "user_verifications"

    id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), primary_key=True, default_factory=uuid4, init=False
    )
    user_id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), ForeignKey("users.id", ondelete="CASCADE"), nullable=False
    )
    type: Mapped[UserVerificationType] = mapped_column(
        SQLEnum(UserVerificationType, native_enum=False, length=MAX_ENUM_LENGTH), nullable=False
    )
    token_hash: Mapped[str] = mapped_column(
        String(MAX_STRING_LENGTH), unique=True, nullable=False
    )  # HMAC-SHA256(secret, token)
    email_metadata: Mapped[dict | None] = mapped_column(
        JSONB, nullable=True
    )  # email provider, send time, reference id
    expires_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), nullable=False)
    used_at: Mapped[datetime | None] = mapped_column(
        DateTime(timezone=True), nullable=True, init=False
    )
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), nullable=False, init=False
    )

    # No relationship needed - only using user_id foreign key directly


class Organization(Base):
    __tablename__ = "organizations"

    id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), primary_key=True, default_factory=uuid4, init=False
    )
    name: Mapped[str] = mapped_column(String(MAX_STRING_LENGTH), unique=True, nullable=False)
    description: Mapped[str | None] = mapped_column(String(MAX_STRING_LENGTH), nullable=True)

    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), nullable=False, init=False
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True),
        server_default=func.now(),
        onupdate=func.now(),
        nullable=False,
        init=False,
    )
    deleted_at: Mapped[datetime | None] = mapped_column(
        DateTime(timezone=True), nullable=True, init=False
    )

    # TODO: consider lazy loading for these relationships if we have a lot of data
    memberships: Mapped[list[OrganizationMembership]] = relationship(
        back_populates="organization", cascade="all, delete-orphan", single_parent=True, init=False
    )
    teams: Mapped[list[Team]] = relationship(
        back_populates="organization", cascade="all, delete-orphan", init=False
    )
    invitations: Mapped[list[OrganizationInvitation]] = relationship(
        back_populates="organization", cascade="all, delete-orphan", single_parent=True, init=False
    )
    stripe_customer_id: Mapped[str | None] = mapped_column(
        String(MAX_STRING_LENGTH), nullable=True, server_default=None, init=False
    )
    # One organization has maximum one subscription
    subscription: Mapped[OrganizationSubscription | None] = relationship(
        back_populates="organization", init=False
    )

    __table_args__ = (UniqueConstraint("stripe_customer_id", name="uc_org_stripe_customer_id"),)


class OrganizationMembership(Base):
    __tablename__ = "organization_memberships"

    id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), primary_key=True, default_factory=uuid4, init=False
    )
    organization_id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), ForeignKey("organizations.id", ondelete="CASCADE"), nullable=False
    )
    user_id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), ForeignKey("users.id", ondelete="CASCADE"), nullable=False
    )
    role: Mapped[OrganizationRole] = mapped_column(
        SQLEnum(OrganizationRole, native_enum=False, length=MAX_ENUM_LENGTH), nullable=False
    )

    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), nullable=False, init=False
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True),
        server_default=func.now(),
        onupdate=func.now(),
        nullable=False,
        init=False,
    )

    organization: Mapped[Organization] = relationship(back_populates="memberships", init=False)
    user: Mapped[User] = relationship(back_populates="organization_memberships", init=False)

    # NOTE: user can belong to multiple organizations, but not the same organization multiple times
    __table_args__ = (UniqueConstraint("organization_id", "user_id", name="uc_org_user"),)


class OrganizationInvitation(Base):
    __tablename__ = "organization_invitations"

    id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), primary_key=True, default_factory=uuid4, init=False
    )
    organization_id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), ForeignKey("organizations.id", ondelete="CASCADE"), nullable=False
    )
    email: Mapped[str] = mapped_column(String(MAX_STRING_LENGTH), nullable=False)
    inviter_user_id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), ForeignKey("users.id", ondelete="CASCADE"), nullable=False
    )
    role: Mapped[OrganizationRole] = mapped_column(
        SQLEnum(OrganizationRole, native_enum=False, length=MAX_ENUM_LENGTH), nullable=False
    )
    token_hash: Mapped[str] = mapped_column(
        String(MAX_STRING_LENGTH), unique=True, nullable=False
    )  # HMAC-SHA256(secret, token)
    expires_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), nullable=False)
    email_metadata: Mapped[dict | None] = mapped_column(
        JSONB, nullable=True, default=None
    )  # email provider, send time, reference id
    status: Mapped[OrganizationInvitationStatus] = mapped_column(
        SQLEnum(OrganizationInvitationStatus, native_enum=False, length=MAX_ENUM_LENGTH),
        nullable=False,
        default=OrganizationInvitationStatus.PENDING,
    )
    used_at: Mapped[datetime | None] = mapped_column(
        DateTime(timezone=True), nullable=True, init=False
    )  # when the invitation was accepted(only for accepted or reject invitations)
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), nullable=False, init=False
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True),
        server_default=func.now(),
        onupdate=func.now(),
        nullable=False,
        init=False,
    )

    organization: Mapped[Organization] = relationship(back_populates="invitations", init=False)
    inviter: Mapped[User] = relationship(foreign_keys=[inviter_user_id], init=False)

    __table_args__ = (UniqueConstraint("organization_id", "email", name="uc_org_invitation_email"),)


# NOTE: team belongs to exactly one organization, so no need for a join table
class Team(Base):
    __tablename__ = "teams"

    id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), primary_key=True, default_factory=uuid4, init=False
    )
    organization_id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), ForeignKey("organizations.id", ondelete="CASCADE"), nullable=False
    )
    name: Mapped[str] = mapped_column(String(MAX_STRING_LENGTH), nullable=False)
    description: Mapped[str | None] = mapped_column(String(MAX_STRING_LENGTH), nullable=True)

    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), nullable=False, init=False
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True),
        server_default=func.now(),
        onupdate=func.now(),
        nullable=False,
        init=False,
    )

    organization: Mapped[Organization] = relationship(back_populates="teams", init=False)
    memberships: Mapped[list[TeamMembership]] = relationship(
        back_populates="team",
        cascade="all, delete-orphan",
        single_parent=True,
        foreign_keys="TeamMembership.team_id",
        init=False,
    )

    # TODO: team name should be unique within an organization?
    __table_args__ = (
        UniqueConstraint("organization_id", "name", name="uc_org_team"),
        # Add unique constraint for the compound foreign key: to be used in the TeamMembership
        # table's ForeignKeyConstraint
        UniqueConstraint("id", "organization_id", name="uc_team_org"),
    )


class TeamMembership(Base):
    __tablename__ = "team_memberships"

    id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), primary_key=True, default_factory=uuid4, init=False
    )

    team_id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), ForeignKey("teams.id", ondelete="CASCADE"), nullable=False
    )
    # NOTE: organization_id is added here for the ForeignKeyConstraint below
    organization_id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), ForeignKey("organizations.id"), nullable=False
    )
    user_id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), ForeignKey("users.id", ondelete="CASCADE"), nullable=False
    )

    role: Mapped[TeamRole] = mapped_column(
        SQLEnum(TeamRole, native_enum=False, length=MAX_ENUM_LENGTH), nullable=False
    )

    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), nullable=False, init=False
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True),
        server_default=func.now(),
        onupdate=func.now(),
        nullable=False,
        init=False,
    )

    team: Mapped[Team] = relationship(
        back_populates="memberships", init=False, foreign_keys=[team_id]
    )
    user: Mapped[User] = relationship(back_populates="team_memberships", init=False)

    # TODO: should probably have test coverage for these constraints
    __table_args__ = (
        UniqueConstraint("team_id", "user_id", name="uc_team_user"),
        # Foreign key to ensure organization_id matches the team's organization
        ForeignKeyConstraint(
            ["team_id", "organization_id"],
            ["teams.id", "teams.organization_id"],
            name="fk_team_org_consistency",
            ondelete="CASCADE",  # Team deletion cascades to its memberships
        ),
        # Foreign key to ensure user is a member of the organization
        ForeignKeyConstraint(
            ["organization_id", "user_id"],
            ["organization_memberships.organization_id", "organization_memberships.user_id"],
            name="fk_user_org_membership",
            ondelete="CASCADE",  # Org membership removal cascades to team memberships
        ),
    )


class MCPServer(Base):
    __tablename__ = "mcp_servers"

    id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), primary_key=True, default_factory=uuid4, init=False
    )
    name: Mapped[str] = mapped_column(String(MAX_STRING_LENGTH), unique=True, nullable=False)
    # e.g., https://example.com/mcp
    url: Mapped[str] = mapped_column(Text, nullable=False)

    # Custom MCP Server, null if it is a public MCP Server
    organization_id: Mapped[UUID | None] = mapped_column(
        PGUUID(as_uuid=True),
        ForeignKey("organizations.id", ondelete="CASCADE"),
        nullable=True,
        server_default=None,
    )

    # Last time the MCP Server was synced for the tool list.
    last_synced_at: Mapped[datetime | None] = mapped_column(
        DateTime(timezone=True), nullable=True, server_default=None
    )

    description: Mapped[str] = mapped_column(Text, nullable=False)
    transport_type: Mapped[MCPServerTransportType] = mapped_column(
        SQLEnum(MCPServerTransportType, native_enum=False, length=MAX_ENUM_LENGTH), nullable=False
    )
    logo: Mapped[str] = mapped_column(Text, nullable=False)
    # TODO: consider adding a category table
    categories: Mapped[list[str]] = mapped_column(ARRAY(String), nullable=False)
    # NOTE: a mcp server might support multiple auth types, e.g., both oauth2 and api key
    auth_configs: Mapped[list[dict]] = mapped_column(ARRAY(JSONB), nullable=False)
    server_metadata: Mapped[dict] = mapped_column(JSONB, nullable=False)
    embedding: Mapped[list[float]] = mapped_column(Vector(EMBEDDING_DIMENSION), nullable=False)

    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), nullable=False, init=False
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True),
        server_default=func.now(),
        onupdate=func.now(),
        nullable=False,
        init=False,
    )

    tools: Mapped[list[MCPTool]] = relationship(
        back_populates="mcp_server", cascade="all, delete-orphan", init=False
    )


class MCPTool(Base):
    __tablename__ = "mcp_tools"

    id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), primary_key=True, default_factory=uuid4, init=False
    )
    mcp_server_id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), ForeignKey("mcp_servers.id", ondelete="CASCADE"), nullable=False
    )
    # NOTE: the name of the tool is not the same as the canonical tool name from the mcp server.
    # e.g., the canonical tool name is "create-pull-request" and the name of the tool
    # can be "GITHUB__CREATE_PULL_REQUEST"
    name: Mapped[str] = mapped_column(String(MAX_STRING_LENGTH), nullable=False, unique=True)
    # NOTE: the description might not be the exact same as the canonical tool description from the
    # mcp server, as some of them might be too long (e.g., openai require < 1024 characters)
    description: Mapped[str] = mapped_column(Text, nullable=False)
    input_schema: Mapped[dict] = mapped_column(JSONB, nullable=False)
    tags: Mapped[list[str]] = mapped_column(ARRAY(String), nullable=False)
    tool_metadata: Mapped[dict] = mapped_column(JSONB, nullable=False)
    embedding: Mapped[list[float]] = mapped_column(Vector(EMBEDDING_DIMENSION), nullable=False)

    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), nullable=False, init=False
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True),
        server_default=func.now(),
        onupdate=func.now(),
        nullable=False,
        init=False,
    )

    mcp_server: Mapped[MCPServer] = relationship(back_populates="tools", init=False)


# NOTE:
# - each org can configure the same mcp server multiple times
# - if the connected account ownership is OPERATIONAL, then this MCP server configuration is only
# for MCP Server operational use (e.g. for fetching info for MCP servers info). In that case it
# should be invisible to users, only use for system operational purpose.
class MCPServerConfiguration(Base):
    __tablename__ = "mcp_server_configurations"

    id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), primary_key=True, default_factory=uuid4, init=False
    )
    name: Mapped[str] = mapped_column(String(MAX_STRING_LENGTH), nullable=False)
    description: Mapped[str | None] = mapped_column(String(MAX_STRING_LENGTH), nullable=True)
    mcp_server_id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), ForeignKey("mcp_servers.id", ondelete="CASCADE"), nullable=False
    )
    organization_id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), ForeignKey("organizations.id", ondelete="CASCADE"), nullable=False
    )
    auth_type: Mapped[AuthType] = mapped_column(
        SQLEnum(AuthType, native_enum=False, length=MAX_ENUM_LENGTH), nullable=False
    )
    connected_account_ownership: Mapped[ConnectedAccountOwnership] = mapped_column(
        SQLEnum(ConnectedAccountOwnership, native_enum=False, length=MAX_ENUM_LENGTH),
        nullable=False,
    )

    # TODO: add whitelabel overrides?
    all_tools_enabled: Mapped[bool] = mapped_column(Boolean, nullable=False)
    # A list of tool ids
    enabled_tools: Mapped[list[UUID]] = mapped_column(ARRAY(PGUUID(as_uuid=True)), nullable=False)

    # TODO: need to check teams actually belongs to the org on app layer
    # whitelisted teams that can use this mcp server configuration
    allowed_teams: Mapped[list[UUID]] = mapped_column(ARRAY(PGUUID(as_uuid=True)), nullable=False)

    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), nullable=False, init=False
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True),
        server_default=func.now(),
        onupdate=func.now(),
        nullable=False,
        init=False,
    )

    # one way relationship to the mcp server
    mcp_server: Mapped[MCPServer] = relationship("MCPServer", init=False)

    __table_args__ = (
        # One OPERATIONAL config per (mcp_server_id)
        # In SQLAlchemy, UniqueConstraint itself does not accept postgresql_where so we use Index
        # for Partial unique index instead
        Index(
            "ux_mcp_server_config_per_server_org",
            "mcp_server_id",
            unique=True,
            postgresql_where=(connected_account_ownership == ConnectedAccountOwnership.OPERATIONAL),
        ),
    )


# TODO:
# - for now, connected account is tied to mcp server configuration, not mcp server
# - for simplicity, we only support one connected account per user per mcp server configuration
# - we might need to support multiple connected accounts per user per mcp server (configuration)
class ConnectedAccount(Base):
    __tablename__ = "connected_accounts"

    id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), primary_key=True, default_factory=uuid4, init=False
    )
    user_id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), ForeignKey("users.id", ondelete="CASCADE"), nullable=False
    )
    mcp_server_configuration_id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True),
        ForeignKey("mcp_server_configurations.id", ondelete="CASCADE"),
        nullable=False,
    )
    auth_credentials: Mapped[dict] = mapped_column(JSONB, nullable=False)

    ownership: Mapped[ConnectedAccountOwnership] = mapped_column(
        SQLEnum(ConnectedAccountOwnership, native_enum=False, length=MAX_ENUM_LENGTH),
        nullable=False,
    )

    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), nullable=False, init=False
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True),
        server_default=func.now(),
        onupdate=func.now(),
        nullable=False,
        init=False,
    )

    mcp_server_configuration: Mapped[MCPServerConfiguration] = relationship(
        "MCPServerConfiguration", init=False
    )

    user: Mapped[User] = relationship("User", init=False)

    # TODO: consider composite key instead
    __table_args__ = (
        UniqueConstraint(
            "user_id",
            "mcp_server_configuration_id",
            name="uc_connected_accounts_one_per_user_per_mcp_server_config",
        ),
        # One Shared Connected Account per (mcp_server_configuration_id)
        # In SQLAlchemy, UniqueConstraint itself does not accept postgresql_where so we use Index
        # for Partial unique index instead
        Index(
            "ux_shared_connected_accounts_one_per_mcp_server_config",
            "mcp_server_configuration_id",
            unique=True,
            postgresql_where=(ownership == ConnectedAccountOwnership.SHARED),
        ),
        # One OPERATIONAL Connected Account per (mcp_server_configuration_id)
        # In SQLAlchemy, UniqueConstraint itself does not accept postgresql_where so we use Index
        # for Partial unique index instead
        Index(
            "ux_operational_connected_accounts_one_per_mcp_server_config",
            "mcp_server_configuration_id",
            unique=True,
            postgresql_where=(ownership == ConnectedAccountOwnership.OPERATIONAL),
        ),
    )


class MCPServerBundle(Base):
    __tablename__ = "mcp_server_bundles"

    id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), primary_key=True, default_factory=uuid4, init=False
    )
    name: Mapped[str] = mapped_column(String(MAX_STRING_LENGTH), nullable=False)
    description: Mapped[str | None] = mapped_column(Text, nullable=True)
    # user who created the mcp server bundle
    user_id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), ForeignKey("users.id", ondelete="CASCADE"), nullable=False
    )
    organization_id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), ForeignKey("organizations.id", ondelete="CASCADE"), nullable=False
    )
    # Opaque bundle key, not to use hash because we would display the bundle key in the UI
    bundle_key: Mapped[str] = mapped_column(String(MAX_STRING_LENGTH), unique=True, nullable=False)

    # a list of mcp server configuration ids the bundle contains
    # TODO: should only allow mcp server configurations of the same mcp server once
    # TODO: should probably only allow mcp server configurations that the user has connected to
    mcp_server_configuration_ids: Mapped[list[UUID]] = mapped_column(
        ARRAY(PGUUID(as_uuid=True)), nullable=False
    )
    user: Mapped[User] = relationship("User", init=False)

    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), nullable=False, init=False
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True),
        server_default=func.now(),
        onupdate=func.now(),
        nullable=False,
        init=False,
    )


class MCPSession(Base):
    """
    Sessions table for maintaining the session state among:
    mcp client (user) - gate22 mcp server - external mcp servers (e.g., RENDER, GITHUB, etc.)
    """

    __tablename__ = "mcp_sessions"
    # the id is also the session id of the gate22 mcp server (ACI.dev controlled)
    id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), primary_key=True, default_factory=uuid4, init=False
    )
    # which mcp server bundle this session is used for
    bundle_id: Mapped[UUID] = mapped_column(PGUUID(as_uuid=True), nullable=False)
    # a map between mcp server id and its session id, e.g.,
    # {"0000-0000-0000-0000": "session_id_1", "0000-0000-0000-0001": "session_id_2"}
    # NOTE: not every mcp server requires and/or supports sessions
    external_mcp_sessions: Mapped[dict[str, str]] = mapped_column(JSONB, nullable=False)

    deleted: Mapped[bool] = mapped_column(
        Boolean, nullable=False, server_default=false(), init=False
    )

    last_accessed_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), nullable=False, init=False
    )

    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), nullable=False, init=False
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True),
        server_default=func.now(),
        onupdate=func.now(),
        nullable=False,
        init=False,
    )


class MCPToolCallLog(Base):
    """
    Logs for tool calls handled by the "MCP" service.
    NOTE: no foreign key constraints are added so that even if other tables' records are deleted,
    we can still keep the logs for reference.
    # we store the ids of the bundle, mcp server, and mcp tool for ambiguity avoidance since
    # the names can change. But we still only display names stored here to user due to:
    # 1. the records can be deleted, so we can't always get the latest names
    # 2. slow to join each record to get latest names
    # 3. for audit logs, we expect logs to reflect the state at the time of the action.
    NOTE: mcp_server_id/name, mcp_tool_id/name and mcp_server_configuration_id/name are nullable
    because user might send a tool call that doesn't have a valid value in the database.
    NOTE: arguments are stored as string because they can be other types than dict due to
    LLM tool call mistakes.
    """

    __tablename__ = "mcp_tool_call_logs"

    id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), primary_key=True, default_factory=uuid4, init=False
    )
    organization_id: Mapped[UUID] = mapped_column(PGUUID(as_uuid=True), nullable=False)
    user_id: Mapped[UUID] = mapped_column(PGUUID(as_uuid=True), nullable=False)
    request_id: Mapped[str] = mapped_column(String(MAX_STRING_LENGTH), nullable=False)
    # the session if of MCPSession table
    session_id: Mapped[UUID] = mapped_column(PGUUID(as_uuid=True), nullable=False)
    bundle_name: Mapped[str] = mapped_column(String(MAX_STRING_LENGTH), nullable=False)
    bundle_id: Mapped[UUID] = mapped_column(PGUUID(as_uuid=True), nullable=False)
    mcp_server_name: Mapped[str | None] = mapped_column(String(MAX_STRING_LENGTH), nullable=True)
    mcp_server_id: Mapped[UUID | None] = mapped_column(PGUUID(as_uuid=True), nullable=True)
    mcp_tool_name: Mapped[str | None] = mapped_column(String(MAX_STRING_LENGTH), nullable=True)
    mcp_tool_id: Mapped[UUID | None] = mapped_column(PGUUID(as_uuid=True), nullable=True)
    mcp_server_configuration_name: Mapped[str | None] = mapped_column(
        String(MAX_STRING_LENGTH), nullable=True
    )
    mcp_server_configuration_id: Mapped[UUID | None] = mapped_column(
        PGUUID(as_uuid=True), nullable=True
    )
    arguments: Mapped[str | None] = mapped_column(Text, nullable=True)
    result: Mapped[dict] = mapped_column(JSONB, nullable=False)
    status: Mapped[MCPToolCallStatus] = mapped_column(
        SQLEnum(MCPToolCallStatus, native_enum=False, length=MAX_ENUM_LENGTH), nullable=False
    )
    # whether the tool call is excecuted via the "EXECUTE_TOOL" tool
    via_execute_tool: Mapped[bool] = mapped_column(Boolean, nullable=False)
    # The full payload of the tool call: JSONRPCToolsCallRequest
    jsonrpc_payload: Mapped[dict] = mapped_column(JSONB, nullable=False)
    started_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), nullable=False)
    ended_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), nullable=False)
    duration_ms: Mapped[int] = mapped_column(Integer, nullable=False)

    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), nullable=False, init=False
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True),
        server_default=func.now(),
        onupdate=func.now(),
        nullable=False,
        init=False,
    )

    # TODO: add index for cursor based pagination


###################################################################################################
# Below tables are used only by the "virtual MCP" service hosting virtual MCP Servers
# see design doc:
# https://www.notion.so/Design-Doc-a-new-service-as-the-execution-engine-for-virtual-MCP-servers-integration-based-26b8378d6a4780b4a389cf302d021c49
###################################################################################################


class VirtualMCPServer(Base):
    """
    This table is close to the "App" table of the tool-calling platform but many fields removed.
    We can almost get rid of this table and combine the data with VirtualMCPTool table, but
    for now we keep it separate to follow the same design pattern we have, for a
    better forward compatibility.
    """

    __tablename__ = "virtual_mcp_servers"

    id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), primary_key=True, default_factory=uuid4, init=False
    )
    name: Mapped[str] = mapped_column(String(MAX_STRING_LENGTH), nullable=False, unique=True)
    description: Mapped[str] = mapped_column(Text, nullable=False)

    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), nullable=False, init=False
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True),
        server_default=func.now(),
        onupdate=func.now(),
        nullable=False,
        init=False,
    )

    tools: Mapped[list[VirtualMCPTool]] = relationship(
        back_populates="server", cascade="all, delete-orphan", init=False
    )


class VirtualMCPTool(Base):
    __tablename__ = "virtual_mcp_tools"

    id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), primary_key=True, default_factory=uuid4, init=False
    )
    virtual_mcp_server_id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True),
        ForeignKey("virtual_mcp_servers.id", ondelete="CASCADE"),
        nullable=False,
    )
    # NOTE: here we still prefix "app name" (e.g., "GMAIL__SEND_EMAIL"), but in the response of
    # tools/list (requested by unified mcp) we will strip off the prefix (e.g., "SEND_EMAIL")
    # we can do this because we will have the "app name" in the mcp url (as query parameter)
    # e.g., https://mcp.aci.dev/virtual/mcp?name=GMAIL
    name: Mapped[str] = mapped_column(String(MAX_STRING_LENGTH), nullable=False, unique=True)
    description: Mapped[str] = mapped_column(Text, nullable=False)
    # NOTE: this input_schema will include "visibility" field for "rest" protocol type
    # But they will be stripped off in the response of tools/list (requested by unified mcp)
    input_schema: Mapped[dict] = mapped_column(JSONB, nullable=False)
    # NOTE: tool_metadata serves similar function as the "protocol & protocol_data" field in the
    # tool-calling platform
    tool_metadata: Mapped[dict] = mapped_column(JSONB, nullable=False)

    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), nullable=False, init=False
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True),
        server_default=func.now(),
        onupdate=func.now(),
        nullable=False,
        init=False,
    )

    server: Mapped[VirtualMCPServer] = relationship(
        "VirtualMCPServer", back_populates="tools", init=False
    )


###################################################################################################
# Below tables are used related to "subscription"
###################################################################################################
class SubscriptionPlan(Base):
    __tablename__ = "subscription_plans"

    id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), primary_key=True, default_factory=uuid4, init=False
    )
    plan_code: Mapped[str] = mapped_column(String(MAX_STRING_LENGTH), unique=True, nullable=False)
    display_name: Mapped[str] = mapped_column(String(MAX_STRING_LENGTH), nullable=False)
    is_public: Mapped[bool] = mapped_column(Boolean, nullable=False)
    # This may be null when the plan is not available for self-served subscription. For example,
    # Free plan or custom offline deals.
    stripe_price_id: Mapped[str | None] = mapped_column(TEXT, nullable=True)
    max_seats_for_subscription: Mapped[int | None] = mapped_column(Integer, nullable=True)
    max_custom_mcp_servers: Mapped[int | None] = mapped_column(Integer, nullable=True)
    log_retention_days: Mapped[int | None] = mapped_column(Integer, nullable=True)
    archived_at: Mapped[datetime | None] = mapped_column(
        DateTime(timezone=True), nullable=True, init=False
    )
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), nullable=False, init=False
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True),
        server_default=func.now(),
        onupdate=func.now(),
        nullable=False,
        init=False,
    )
    subscriptions: Mapped[list[OrganizationSubscription]] = relationship(
        back_populates="subscription_plan", init=False
    )


class OrganizationSubscription(Base):
    __tablename__ = "organization_subscriptions"

    id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), primary_key=True, default_factory=uuid4, init=False
    )
    organization_id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True),
        ForeignKey("organizations.id", ondelete="CASCADE"),
        nullable=False,
        unique=True,  # One organization can have only one active subscription
    )
    subscription_plan_id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True),
        # Deleting Subscription plan should be restricted if used by any organization.
        # We should rather mark as archived instead.
        ForeignKey("subscription_plans.id", ondelete="RESTRICT"),
        nullable=False,
    )
    seat_count: Mapped[int] = mapped_column(Integer, nullable=False)
    stripe_subscription_status: Mapped[str] = mapped_column(
        String(MAX_STRING_LENGTH), nullable=False
    )
    stripe_subscription_id: Mapped[str] = mapped_column(
        TEXT,
        nullable=False,
        unique=True,  # One stripe subscription id can only be used by one organization
    )
    stripe_subscription_item_id: Mapped[str] = mapped_column(TEXT, nullable=False)
    current_period_start: Mapped[datetime] = mapped_column(DateTime(timezone=True), nullable=False)
    current_period_end: Mapped[datetime] = mapped_column(DateTime(timezone=True), nullable=False)
    cancel_at_period_end: Mapped[bool] = mapped_column(Boolean, nullable=False)
    subscription_start_date: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), nullable=False
    )
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), nullable=False, init=False
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True),
        server_default=func.now(),
        onupdate=func.now(),
        nullable=False,
        init=False,
    )

    organization: Mapped[Organization] = relationship(back_populates="subscription", init=False)
    subscription_plan: Mapped[SubscriptionPlan] = relationship(
        back_populates="subscriptions", init=False
    )


class SubscriptionStripeEventLogs(Base):
    __tablename__ = "subscription_stripe_event_logs"

    id: Mapped[UUID] = mapped_column(
        PGUUID(as_uuid=True), primary_key=True, default_factory=uuid4, init=False
    )
    stripe_event_id: Mapped[str] = mapped_column(TEXT, nullable=False, unique=True)
    type: Mapped[str] = mapped_column(String(MAX_STRING_LENGTH), nullable=False)
    payload: Mapped[dict] = mapped_column(JSONB, nullable=False)
    received_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), nullable=False)
    processed_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True), nullable=True)
    process_error: Mapped[str | None] = mapped_column(String(MAX_STRING_LENGTH), nullable=True)
    process_attempts: Mapped[int] = mapped_column(Integer, nullable=False)
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), nullable=False, init=False
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True),
        server_default=func.now(),
        onupdate=func.now(),
        nullable=False,
        init=False,
    )
