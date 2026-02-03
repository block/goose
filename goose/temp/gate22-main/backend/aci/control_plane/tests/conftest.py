from collections.abc import Generator
from typing import cast

import pytest
from fastapi.testclient import TestClient
from sqlalchemy import Inspector, inspect
from sqlalchemy.orm import Session

from aci.common import utils
from aci.common.db import crud
from aci.common.db.sql_models import (
    Base,
    ConnectedAccount,
    MCPServer,
    MCPServerBundle,
    MCPServerConfiguration,
    Organization,
    Team,
    User,
)
from aci.common.enums import (
    AuthType,
    ConnectedAccountOwnership,
    HttpLocation,
    MCPServerTransportType,
    OrganizationRole,
    UserIdentityProvider,
)
from aci.common.logging_setup import get_logger
from aci.common.openai_client import init_openai_client
from aci.common.schemas.auth import ActAsInfo
from aci.common.schemas.mcp_auth import APIKeyConfig, AuthConfig, NoAuthConfig
from aci.common.schemas.mcp_server import (
    MCPServerMetadata,
    MCPServerUpsert,
)
from aci.common.schemas.mcp_server_bundle import MCPServerBundleCreate
from aci.common.schemas.mcp_server_configuration import MCPServerConfigurationCreate
from aci.common.test_utils import clear_database
from aci.control_plane import config
from aci.control_plane import dependencies as deps
from aci.control_plane.main import app as fastapi_app
from aci.control_plane.tests import helper

logger = get_logger(__name__)

init_openai_client(config.OPENAI_API_KEY)

test_jwt_signing_key = config.JWT_SIGNING_KEY
test_jwt_algorithm = config.JWT_ALGORITHM
test_jwt_access_token_expire_minutes = config.JWT_ACCESS_TOKEN_EXPIRE_MINUTES


# call this one time for entire tests because it's slow and costs money (negligible) as it needs
# to generate embeddings using OpenAI for each app and function
dummy_mcp_servers_to_be_inserted = helper.prepare_mcp_servers()
DUMMY_MCP_SERVER_NAME_NOTION = "NOTION"
DUMMY_MCP_SERVER_NAME_GITHUB = "GITHUB"
DUMMY_MCP_SERVER_NAME_GMAIL = "GMAIL"


@pytest.fixture(scope="function")
def test_client(db_session: Session) -> Generator[TestClient, None, None]:
    fastapi_app.dependency_overrides[deps.yield_db_session] = lambda: db_session
    # disable following redirects for testing login
    # NOTE: need to set base_url to http://localhost because we set TrustedHostMiddleware in main.py
    with TestClient(fastapi_app, base_url="http://localhost", follow_redirects=False) as c:
        yield c


# ------------------------------------------------------------
# Dummy Access Tokens for testing
# - dummy_access_token_admin
# - dummy_access_token_admin_act_as_member
# - dummy_access_token_member
# - dummy_access_token_no_orgs (without act as)
# - dummy_access_token_another_org (act as other organization)
# ------------------------------------------------------------
@pytest.fixture(scope="function")
def dummy_access_token_admin(dummy_admin: User) -> str:
    """
    Access token of `dummy_user` with `admin` role in `dummy_organization`
    """
    org_membership = dummy_admin.organization_memberships[0]
    return utils.sign_token(
        user=dummy_admin,
        act_as=ActAsInfo(
            organization_id=org_membership.organization_id, role=OrganizationRole.ADMIN
        ),
        jwt_signing_key=test_jwt_signing_key,
        jwt_algorithm=test_jwt_algorithm,
        jwt_access_token_expire_minutes=test_jwt_access_token_expire_minutes,
    )


@pytest.fixture(scope="function")
def dummy_access_token_admin_act_as_member(dummy_admin: User) -> str:
    """
    Access token of `dummy_user` with `admin` role in `dummy_organization`, but act as `member` role
    """
    org_membership = dummy_admin.organization_memberships[0]
    return utils.sign_token(
        user=dummy_admin,
        act_as=ActAsInfo(
            organization_id=org_membership.organization_id, role=OrganizationRole.MEMBER
        ),
        jwt_signing_key=test_jwt_signing_key,
        jwt_algorithm=test_jwt_algorithm,
        jwt_access_token_expire_minutes=test_jwt_access_token_expire_minutes,
    )


@pytest.fixture(scope="function")
def dummy_access_token_member(dummy_member: User) -> str:
    """
    Access token of `dummy_user` with `member` role in `dummy_organization`
    """
    org_membership = dummy_member.organization_memberships[0]
    return utils.sign_token(
        user=dummy_member,
        act_as=ActAsInfo(
            organization_id=org_membership.organization_id, role=OrganizationRole.MEMBER
        ),
        jwt_signing_key=test_jwt_signing_key,
        jwt_algorithm=test_jwt_algorithm,
        jwt_access_token_expire_minutes=test_jwt_access_token_expire_minutes,
    )


@pytest.fixture(scope="function")
def dummy_access_token_member_2(dummy_member_2: User) -> str:
    """
    Access token of `dummy_user` with `member` role in `dummy_organization`
    """
    org_membership = dummy_member_2.organization_memberships[0]
    return utils.sign_token(
        user=dummy_member_2,
        act_as=ActAsInfo(
            organization_id=org_membership.organization_id, role=OrganizationRole.MEMBER
        ),
        jwt_signing_key=test_jwt_signing_key,
        jwt_algorithm=test_jwt_algorithm,
        jwt_access_token_expire_minutes=test_jwt_access_token_expire_minutes,
    )


@pytest.fixture(scope="function")
def dummy_access_token_no_orgs(dummy_user_without_org: User) -> str:
    """
    Access token of a user without any organization
    """
    return utils.sign_token(
        user=dummy_user_without_org,
        act_as=None,
        jwt_signing_key=test_jwt_signing_key,
        jwt_algorithm=test_jwt_algorithm,
        jwt_access_token_expire_minutes=test_jwt_access_token_expire_minutes,
    )


@pytest.fixture(scope="function")
def dummy_access_token_another_org(db_session: Session) -> str:
    """
    Access token of a user acted as other organization. This user has no membership in
    `dummy_organization`.
    """
    dummy_other_user = crud.users.create_user(
        db_session=db_session,
        name="Dummy Other User",
        email="dummy_other@example.com",
        password_hash=None,
        identity_provider=UserIdentityProvider.EMAIL,
        email_verified=True,
    )

    dummy_other_organization = crud.organizations.create_organization(
        db_session=db_session,
        name="Dummy Other Organization",
        description="Dummy Other Organization Description",
    )

    crud.organizations.add_user_to_organization(
        db_session=db_session,
        organization_id=dummy_other_organization.id,
        user_id=dummy_other_user.id,
        role=OrganizationRole.ADMIN,
    )
    db_session.commit()

    return utils.sign_token(
        user=dummy_other_user,
        act_as=ActAsInfo(organization_id=dummy_other_organization.id, role=OrganizationRole.ADMIN),
        jwt_signing_key=test_jwt_signing_key,
        jwt_algorithm=test_jwt_algorithm,
        jwt_access_token_expire_minutes=test_jwt_access_token_expire_minutes,
    )


# ------------------------------------------------------------
# Dummy organization and user
# - dummy_organization (with dummy_user as org admin, and a dummy team with dummy_user inside)
# - dummy_user
# - dummy_admin (same as dummy_user, but is admin in the dummy_organization)
# - dummy_member (same as dummy_user, but is member in the dummy_organization)
# ------------------------------------------------------------


@pytest.fixture(scope="function")
def dummy_organization(db_session: Session) -> Organization:
    dummy_organization = crud.organizations.create_organization(
        db_session=db_session,
        name="Dummy Organization",
        description="Dummy Organization Description",
    )
    dummy_user = crud.users.create_user(
        db_session=db_session,
        name="Dummy User",
        email="dummy@example.com",
        password_hash=None,
        identity_provider=UserIdentityProvider.EMAIL,
        email_verified=True,
    )
    crud.organizations.add_user_to_organization(
        db_session=db_session,
        organization_id=dummy_organization.id,
        user_id=dummy_user.id,
        role=OrganizationRole.ADMIN,
    )
    dummy_team = crud.teams.create_team(
        db_session=db_session,
        organization_id=dummy_organization.id,
        name="Dummy Team",
        description="Dummy Team Description",
    )
    crud.teams.add_team_member(
        db_session=db_session,
        organization_id=dummy_organization.id,
        team_id=dummy_team.id,
        user_id=dummy_user.id,
    )
    db_session.commit()
    return dummy_organization


@pytest.fixture(scope="function")
def dummy_user(dummy_organization: Organization) -> User:
    """
    `dummy_user` with in `dummy_organization`
    """
    # Find the membership for "dummy@example.com"
    membership = next(
        m for m in dummy_organization.memberships if m.user.email == "dummy@example.com"
    )
    return membership.user


@pytest.fixture(scope="function")
def dummy_admin(db_session: Session, dummy_organization: Organization) -> User:
    """
    `dummy_user` with `admin` role in `dummy_organization`
    """
    # Find the membership for "dummy@example.com" and set it to ADMIN
    membership = next(
        m for m in dummy_organization.memberships if m.user.email == "dummy@example.com"
    )
    membership.role = OrganizationRole.ADMIN
    db_session.commit()
    return membership.user


@pytest.fixture(scope="function")
def dummy_member(db_session: Session, dummy_organization: Organization) -> User:
    """
    `dummy_user` with `member` role in `dummy_organization`
    """
    # Find the membership for "dummy@example.com" and set it to MEMBER
    membership = next(
        m for m in dummy_organization.memberships if m.user.email == "dummy@example.com"
    )
    membership.role = OrganizationRole.MEMBER
    db_session.commit()
    return membership.user


@pytest.fixture(scope="function")
def dummy_member_2(db_session: Session, dummy_organization: Organization) -> User:
    """
    This will add another member into dummy_organization.
    """
    dummy_user_2 = crud.users.create_user(
        db_session=db_session,
        name="Dummy User 2",
        email="dummy_2@example.com",
        password_hash=None,
        identity_provider=UserIdentityProvider.EMAIL,
        email_verified=True,
    )
    crud.organizations.add_user_to_organization(
        db_session=db_session,
        organization_id=dummy_organization.id,
        user_id=dummy_user_2.id,
        role=OrganizationRole.MEMBER,
    )
    db_session.commit()
    return dummy_user_2


@pytest.fixture(scope="function")
def dummy_member_3(db_session: Session, dummy_organization: Organization) -> User:
    """
    This will add another member into dummy_organization.
    """
    dummy_user_3 = crud.users.create_user(
        db_session=db_session,
        name="Dummy User 3",
        email="dummy_3@example.com",
        password_hash=None,
        identity_provider=UserIdentityProvider.EMAIL,
        email_verified=True,
    )
    crud.organizations.add_user_to_organization(
        db_session=db_session,
        organization_id=dummy_organization.id,
        user_id=dummy_user_3.id,
        role=OrganizationRole.MEMBER,
    )
    db_session.commit()
    return dummy_user_3


@pytest.fixture(scope="function")
def dummy_team(dummy_organization: Organization) -> Team:
    return dummy_organization.teams[0]


@pytest.fixture(scope="function")
def dummy_user_without_org(db_session: Session) -> User:
    user = crud.users.create_user(
        db_session=db_session,
        name="Dummy User Without Org",
        email="dummy_without_org@example.com",
        password_hash=None,
        identity_provider=UserIdentityProvider.EMAIL,
        email_verified=True,
    )
    db_session.commit()
    return user


# ------------------------------------------------------------
#
# Dummy MCP Servers
#
# ------------------------------------------------------------


@pytest.fixture(scope="function")
def dummy_mcp_servers(db_session: Session, dummy_custom_mcp_server: MCPServer) -> list[MCPServer]:
    dummy_mcp_servers = []
    for (
        mcp_server_upsert,
        tools_upsert,
        embedding,
        mcp_tool_embeddings,
    ) in dummy_mcp_servers_to_be_inserted:
        mcp_server = crud.mcp_servers.create_mcp_server(
            db_session=db_session,
            organization_id=None,
            mcp_server_upsert=mcp_server_upsert,
            embedding=embedding,
        )
        crud.mcp_tools.create_mcp_tools(
            db_session=db_session,
            mcp_tool_upserts=tools_upsert,
            mcp_tool_embeddings=mcp_tool_embeddings,
        )
        dummy_mcp_servers.append(mcp_server)

    # Add a dummy custom MCP server
    dummy_mcp_servers.append(dummy_custom_mcp_server)
    db_session.commit()

    return dummy_mcp_servers


@pytest.fixture(scope="function")
def dummy_custom_mcp_server(db_session: Session, dummy_organization: Organization) -> MCPServer:
    mcp_server = crud.mcp_servers.create_mcp_server(
        db_session=db_session,
        organization_id=dummy_organization.id,
        mcp_server_upsert=MCPServerUpsert(
            name="TEST_MCP_SERVER",
            url="https://test-mcp-server.com",
            description="Test MCP server",
            categories=["test"],
            transport_type=MCPServerTransportType.STREAMABLE_HTTP,
            auth_configs=[
                AuthConfig.model_validate(
                    APIKeyConfig(
                        type=AuthType.API_KEY, location=HttpLocation.HEADER, name="X-API-Key"
                    )
                ),
                AuthConfig.model_validate(NoAuthConfig(type=AuthType.NO_AUTH)),
            ],
            logo="https://test-mcp-server.com/logo.png",
            server_metadata=MCPServerMetadata(),
        ),
        embedding=[0.1] * 1024,
    )
    db_session.commit()
    return mcp_server


@pytest.fixture(scope="function")
def dummy_mcp_server_notion(dummy_mcp_servers: list[MCPServer]) -> MCPServer:
    dummy_mcp_server_notion = next(
        dummy_mcp_server
        for dummy_mcp_server in dummy_mcp_servers
        if dummy_mcp_server.name == DUMMY_MCP_SERVER_NAME_NOTION
    )
    assert dummy_mcp_server_notion is not None
    return dummy_mcp_server_notion


@pytest.fixture(scope="function")
def dummy_mcp_server_github(dummy_mcp_servers: list[MCPServer]) -> MCPServer:
    dummy_mcp_server_notion = next(
        dummy_mcp_server
        for dummy_mcp_server in dummy_mcp_servers
        if dummy_mcp_server.name == DUMMY_MCP_SERVER_NAME_GITHUB
    )
    assert dummy_mcp_server_notion is not None
    return dummy_mcp_server_notion


@pytest.fixture(scope="function")
def dummy_mcp_server(dummy_mcp_server_notion: MCPServer) -> MCPServer:
    """
    alias for dummy_mcp_server_notion
    """
    return dummy_mcp_server_notion


# ------------------------------------------------------------
#
# Dummy MCP Servers Configurations
#
# ------------------------------------------------------------


@pytest.fixture(scope="function")
def dummy_mcp_server_configurations(
    db_session: Session,
    dummy_organization: Organization,
    dummy_mcp_servers: list[MCPServer],
    dummy_team: Team,
) -> list[MCPServerConfiguration]:
    dummy_mcp_server_configurations = []
    for dummy_mcp_server in dummy_mcp_servers:
        dummy_mcp_server_configuration = crud.mcp_server_configurations.create_mcp_server_configuration(  # noqa: E501
            db_session=db_session,
            organization_id=dummy_organization.id,
            mcp_server_configuration=MCPServerConfigurationCreate(
                name=f"Dummy MCP Server Configuration {dummy_mcp_server.name}",
                description=f"Dummy MCP Server Configuration {dummy_mcp_server.name} Description",
                mcp_server_id=dummy_mcp_server.id,
                auth_type=dummy_mcp_server.auth_configs[0]["type"],
                connected_account_ownership=ConnectedAccountOwnership.INDIVIDUAL,
                all_tools_enabled=True,
                enabled_tools=[],
                allowed_teams=[dummy_team.id],
            ),
        )

        # set GMAIL configuration to shared for testing.
        if dummy_mcp_server.name == DUMMY_MCP_SERVER_NAME_GMAIL:
            dummy_mcp_server_configuration.connected_account_ownership = (
                ConnectedAccountOwnership.SHARED
            )
            db_session.commit()

        dummy_mcp_server_configurations.append(dummy_mcp_server_configuration)

    return dummy_mcp_server_configurations


@pytest.fixture(scope="function")
def dummy_mcp_server_configuration(
    dummy_mcp_server_configuration_notion: MCPServerConfiguration,
) -> MCPServerConfiguration:
    """
    alias for dummy_mcp_server_configuration_notion
    """
    return dummy_mcp_server_configuration_notion


@pytest.fixture(scope="function")
def dummy_mcp_server_configuration_notion(
    dummy_mcp_server_configurations: list[MCPServerConfiguration],
    dummy_mcp_server_notion: MCPServer,
) -> MCPServerConfiguration:
    """
    A dummy MCP server configuration under dummy_organization, allowed [dummy_team]
    """
    dummy_mcp_server_configuration = next(
        dummy_mcp_server_configuration
        for dummy_mcp_server_configuration in dummy_mcp_server_configurations
        if dummy_mcp_server_configuration.mcp_server_id == dummy_mcp_server_notion.id
    )
    assert dummy_mcp_server_configuration is not None
    return dummy_mcp_server_configuration


@pytest.fixture(scope="function")
def dummy_mcp_server_configuration_github(
    dummy_mcp_server_configurations: list[MCPServerConfiguration],
    dummy_mcp_server_github: MCPServer,
) -> MCPServerConfiguration:
    dummy_mcp_server_configuration = next(
        dummy_mcp_server_configuration
        for dummy_mcp_server_configuration in dummy_mcp_server_configurations
        if dummy_mcp_server_configuration.mcp_server_id == dummy_mcp_server_github.id
    )
    assert dummy_mcp_server_configuration is not None
    return dummy_mcp_server_configuration


@pytest.fixture(scope="function")
def dummy_mcp_server_configuration_gmail_shared(
    dummy_mcp_server_configurations: list[MCPServerConfiguration],
) -> MCPServerConfiguration:
    dummy_mcp_server_configuration = next(
        dummy_mcp_server_configuration
        for dummy_mcp_server_configuration in dummy_mcp_server_configurations
        if dummy_mcp_server_configuration.connected_account_ownership
        == ConnectedAccountOwnership.SHARED
    )
    assert dummy_mcp_server_configuration is not None
    return dummy_mcp_server_configuration


# ------------------------------------------------------------
#
# Dummy Connected Accounts
#
# ------------------------------------------------------------


@pytest.fixture(scope="function")
def dummy_connected_accounts(
    db_session: Session,
    dummy_member: User,
    dummy_member_2: User,
    dummy_mcp_server_configuration_github: MCPServerConfiguration,
    dummy_mcp_server_configuration_notion: MCPServerConfiguration,
    dummy_mcp_server_configuration_gmail_shared: MCPServerConfiguration,
) -> list[ConnectedAccount]:
    """
    Test connection graph:

    dummy_member ──via individual account──> dummy_mcp_server_configuration_github
               ──via individual account──> dummy_mcp_server_configuration_notion

    dummy_member_2 ──via individual account──> dummy_mcp_server_configuration_github
                 ──via shared account───> dummy_mcp_server_configuration_gmail_shared

    Legend:
    - Users connect to MCP server configurations through connected accounts
    - Personal accounts: individual user connections
    - Shared accounts: organization-wide shared connections
    """

    connected_accounts = []

    connected_accounts.append(
        crud.connected_accounts.create_connected_account(
            db_session=db_session,
            user_id=dummy_member.id,
            mcp_server_configuration_id=dummy_mcp_server_configuration_github.id,
            auth_credentials={},
            ownership=ConnectedAccountOwnership.INDIVIDUAL,
        )
    )
    connected_accounts.append(
        crud.connected_accounts.create_connected_account(
            db_session=db_session,
            user_id=dummy_member.id,
            mcp_server_configuration_id=dummy_mcp_server_configuration_notion.id,
            auth_credentials={},
            ownership=ConnectedAccountOwnership.INDIVIDUAL,
        )
    )
    connected_accounts.append(
        crud.connected_accounts.create_connected_account(
            db_session=db_session,
            user_id=dummy_member_2.id,
            mcp_server_configuration_id=dummy_mcp_server_configuration_github.id,
            auth_credentials={},
            ownership=ConnectedAccountOwnership.INDIVIDUAL,
        )
    )
    connected_accounts.append(
        crud.connected_accounts.create_connected_account(
            db_session=db_session,
            user_id=dummy_member_2.id,
            mcp_server_configuration_id=dummy_mcp_server_configuration_gmail_shared.id,
            auth_credentials={},
            ownership=ConnectedAccountOwnership.SHARED,
        )
    )
    return connected_accounts


@pytest.fixture(scope="function")
def dummy_mcp_server_bundles(
    dummy_organization: Organization,
    db_session: Session,
    dummy_member: User,
    dummy_member_2: User,
    dummy_mcp_server_configuration_github: MCPServerConfiguration,
    dummy_mcp_server_configuration_notion: MCPServerConfiguration,
) -> list[MCPServerBundle]:
    """
    Test bundle graph:

    dummy_member ──owns──> Bundle #1 [github + notion]
               ──owns──> Bundle #2 [github only]

    dummy_member_2 ──owns──> Bundle #3 [github only]

    Legend:
    - Users own MCP server bundles
    - Each bundle contains one or more MCP server configurations
    - Bundles group related MCP servers for easier management
    """
    mcp_server_bundles = []
    mcp_server_bundles.append(
        crud.mcp_server_bundles.create_mcp_server_bundle(
            db_session=db_session,
            user_id=dummy_member.id,
            organization_id=dummy_organization.id,
            mcp_server_bundle_create=MCPServerBundleCreate(
                mcp_server_configuration_ids=[
                    dummy_mcp_server_configuration_github.id,
                    dummy_mcp_server_configuration_notion.id,
                ],
                name="Dummy MCPServerBundle 1 Github + Notion",
                description="Dummy MCPServerBundle 1 Github + Notion Description",
            ),
            bundle_key="Dummy MCPServerBundle 1 Bundle Key",
        )
    )
    mcp_server_bundles.append(
        crud.mcp_server_bundles.create_mcp_server_bundle(
            db_session=db_session,
            user_id=dummy_member.id,
            organization_id=dummy_organization.id,
            mcp_server_bundle_create=MCPServerBundleCreate(
                mcp_server_configuration_ids=[dummy_mcp_server_configuration_github.id],
                name="Dummy MCPServerBundle Github",
                description="Dummy MCPServerBundle 2 Github Description",
            ),
            bundle_key="Dummy MCPServerBundle 2 Bundle Key",
        )
    )
    mcp_server_bundles.append(
        crud.mcp_server_bundles.create_mcp_server_bundle(
            db_session=db_session,
            user_id=dummy_member_2.id,
            organization_id=dummy_organization.id,
            mcp_server_bundle_create=MCPServerBundleCreate(
                mcp_server_configuration_ids=[dummy_mcp_server_configuration_github.id],
                name="Dummy MCPServerBundle 3 Github",
                description="Dummy MCPServerBundle 3 Github Description",
            ),
            bundle_key="Dummy MCPServerBundle 3 Bundle Key",
        )
    )
    db_session.commit()
    return mcp_server_bundles


# ------------------------------------------------------------
#
# Database session setup and cleanup
#
# ------------------------------------------------------------
@pytest.fixture(scope="function")
def db_session() -> Generator[Session, None, None]:
    """
    Create a database session for testing.
    Ensures we're using the test database.
    Each test gets its own database session for better isolation.
    """
    assert config.DB_HOST == "test-db", "Must use test-db for tests"
    assert "test" in config.DB_FULL_URL.lower(), "Database URL must contain 'test' for safety"

    with utils.create_db_session(config.DB_FULL_URL) as session:
        yield session


@pytest.fixture(scope="function", autouse=True)
def database_setup_and_cleanup(db_session: Session) -> Generator[None, None, None]:
    """
    Setup and cleanup the database for each test case.
    """
    inspector = cast(Inspector, inspect(db_session.bind))

    # Check if all tables defined in models are created in the db
    for table in Base.metadata.tables.values():
        if not inspector.has_table(table.name):
            pytest.exit(f"Table {table} does not exist in the database.")

    clear_database(db_session)
    yield  # This allows the test to run
    clear_database(db_session)
