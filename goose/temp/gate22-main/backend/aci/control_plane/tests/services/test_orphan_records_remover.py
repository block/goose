import enum
from uuid import UUID, uuid4

import pytest
from sqlalchemy.orm import Session

from aci.common.db import crud
from aci.common.db.sql_models import (
    ConnectedAccount,
    MCPServer,
    MCPServerBundle,
    MCPServerConfiguration,
    Organization,
    Team,
    User,
)
from aci.common.enums import ConnectedAccountOwnership
from aci.common.logging_setup import get_logger
from aci.common.schemas.mcp_server_bundle import MCPServerBundleCreate
from aci.control_plane import access_control
from aci.control_plane.services.orphan_records_remover import (
    OrphanRecordsRemoval,
    OrphanRecordsRemover,
)

logger = get_logger(__name__)


def _create_team_with_members(
    db_session: Session, organization_id: UUID, user_ids: list[UUID]
) -> Team:
    random_name = f"Team {uuid4()}"
    team = crud.teams.create_team(
        db_session=db_session,
        organization_id=organization_id,
        name=f"Team {random_name}",
        description="Team Description",
    )
    for user_id in user_ids:
        crud.teams.add_team_member(
            db_session=db_session,
            organization_id=organization_id,
            team_id=team.id,
            user_id=user_id,
        )
    db_session.commit()
    return team


def _assert_users_configuration_accessibilities(
    db_session: Session,
    accessibilities: list[tuple[UUID, UUID, bool]],
) -> None:
    for user_id, mcp_server_configuration_id, accessibility in accessibilities:
        assert (
            access_control.check_mcp_server_config_accessibility(
                db_session=db_session,
                user_id=user_id,
                mcp_server_configuration_id=mcp_server_configuration_id,
                throw_error_if_not_permitted=False,
            )
            is accessibility
        )


def _assert_connected_accounts_removal(
    db_session: Session,
    removal_result: OrphanRecordsRemoval,
    expected_removed_connected_accounts: list[UUID],
    expected_retained_connected_accounts: list[UUID],
) -> None:
    assert removal_result.connected_accounts is not None
    for connected_account_id in expected_removed_connected_accounts:
        assert (
            crud.connected_accounts.get_connected_account_by_id(
                db_session=db_session,
                connected_account_id=connected_account_id,
                throw_error_if_not_found=False,
            )
            is None
        )
        assert connected_account_id in [a.id for a in removal_result.connected_accounts]
    for connected_account_id in expected_retained_connected_accounts:
        assert (
            crud.connected_accounts.get_connected_account_by_id(
                db_session=db_session,
                connected_account_id=connected_account_id,
                throw_error_if_not_found=False,
            )
            is not None
        )
        assert connected_account_id not in [a.id for a in removal_result.connected_accounts]


def _assert_mcp_configurations_in_bundles_removal(
    db_session: Session,
    removal_result: OrphanRecordsRemoval,
    expected_removed_mcp_configurations_in_bundles: list[tuple[UUID, list[UUID]]]
    | None = None,  # list of (bundle_id, configuration_ids)[]
    expected_retained_mcp_configurations_in_bundles: list[tuple[UUID, list[UUID]]]
    | None = None,  # list of (bundle_id, configuration_ids)[]
) -> None:
    assert removal_result.mcp_configurations_in_bundles is not None

    if expected_removed_mcp_configurations_in_bundles is not None:
        for bundle_id, configuration_ids in expected_removed_mcp_configurations_in_bundles:
            # Check if the records are removed from the database
            bundle = crud.mcp_server_bundles.get_mcp_server_bundle_by_id(
                db_session=db_session,
                mcp_server_bundle_id=bundle_id,
            )
            assert bundle is not None
            for configuration_id in configuration_ids:
                assert configuration_id not in bundle.mcp_server_configuration_ids

            # Check if the removal results contain the removal records
            for configuration_id in configuration_ids:
                assert (bundle_id, configuration_id) in [
                    (a.bundle_id, a.configuration_id)
                    for a in removal_result.mcp_configurations_in_bundles
                ]

    if expected_retained_mcp_configurations_in_bundles is not None:
        for bundle_id, configuration_ids in expected_retained_mcp_configurations_in_bundles:
            # Check if the records are retained in the database
            bundle = crud.mcp_server_bundles.get_mcp_server_bundle_by_id(
                db_session=db_session,
                mcp_server_bundle_id=bundle_id,
            )
            assert bundle is not None
            for configuration_id in configuration_ids:
                assert configuration_id in bundle.mcp_server_configuration_ids

            # Check if the removal results do not contain the retention records
            for configuration_id in configuration_ids:
                assert (bundle_id, configuration_id) not in [
                    (a.bundle_id, a.configuration_id)
                    for a in removal_result.mcp_configurations_in_bundles
                ]


OrphanConnectedAccountTestCase = enum.Enum(
    "OrphanConnectedAccountTestCase",
    [
        "remove_team_none",
        "remove_team_team_1",
        "remove_team_team_2",
        "remove_team_all",
        "remove_user_from_team_1",
        "remove_user_2_from_team_1",
        "remove_user_2_from_team_2",
    ],
)


@pytest.fixture(scope="function")
def dummy_test_setting_1(
    db_session: Session,
    dummy_organization: Organization,
    dummy_member: User,
    dummy_member_2: User,
    dummy_mcp_server_configuration_github: MCPServerConfiguration,
    dummy_mcp_server_configuration: MCPServerConfiguration,
    dummy_mcp_server_configuration_gmail_shared: MCPServerConfiguration,
) -> tuple[MCPServerConfiguration, list[Team], list[ConnectedAccount], list[MCPServerBundle]]:
    """
    Connection Visualization:
    ┌─────────────────────────────────────────────┐
    │ dummy_mcp_server_configuration (individual) │
    └─────────────────────────────────────────────┘
        │
    (allowed_teams)
        │                      ┌─────────────┐   ┌────────────────────────┐
        │   ┌────────┐   ┌────>│ dummy_member  │──>│ connected_account_user │
        ├──>│ team_1 │───┤     └─────────────┘   └────────────────────────┘
        │   └────────┘   │
        │   ┌────────┐   └────>┌──────────────┐   ┌──────────────────────────┐
        └──>│ team_2 │────────>│ dummy_member_2 │──>│ connected_account_user_2 │
            └────────┘         └──────────────┘   └──────────────────────────┘

    Bundles:
    - Bundle #1:
        dummy_member [dummy_mcp_server_configuration, dummy_mcp_server_configuration_github]
    - Bundle #2:
        dummy_member_2 [dummy_mcp_server_configuration, dummy_mcp_server_configuration_gmail_shared]
    """

    team_1 = _create_team_with_members(
        db_session, dummy_organization.id, [dummy_member.id, dummy_member_2.id]
    )
    team_2 = _create_team_with_members(db_session, dummy_organization.id, [dummy_member_2.id])
    dummy_mcp_server_configuration.allowed_teams = [team_1.id, team_2.id]

    dummy_mcp_server_configuration.connected_account_ownership = (
        ConnectedAccountOwnership.INDIVIDUAL
    )

    # Create connected accounts for both users
    connected_account_user = crud.connected_accounts.create_connected_account(
        db_session=db_session,
        user_id=dummy_member.id,
        mcp_server_configuration_id=dummy_mcp_server_configuration.id,
        auth_credentials={},
        ownership=ConnectedAccountOwnership.INDIVIDUAL,
    )
    connected_account_user_2 = crud.connected_accounts.create_connected_account(
        db_session=db_session,
        user_id=dummy_member_2.id,
        mcp_server_configuration_id=dummy_mcp_server_configuration.id,
        auth_credentials={},
        ownership=ConnectedAccountOwnership.INDIVIDUAL,
    )

    # Creating others records for Connected Accounts and Bundle's Configuration
    crud.connected_accounts.create_connected_account(
        db_session=db_session,
        user_id=dummy_member.id,
        mcp_server_configuration_id=dummy_mcp_server_configuration_github.id,
        auth_credentials={},
        ownership=ConnectedAccountOwnership.INDIVIDUAL,
    )
    dummy_mcp_server_configuration_github.allowed_teams = [team_1.id, team_2.id]
    crud.connected_accounts.create_connected_account(
        db_session=db_session,
        user_id=dummy_member_2.id,
        mcp_server_configuration_id=dummy_mcp_server_configuration_gmail_shared.id,
        auth_credentials={},
        ownership=ConnectedAccountOwnership.INDIVIDUAL,
    )
    dummy_mcp_server_configuration_gmail_shared.allowed_teams = [team_1.id, team_2.id]

    # Create Bundles for both users, both contains the same MCP Server Configuration
    bundle_user = crud.mcp_server_bundles.create_mcp_server_bundle(
        db_session=db_session,
        user_id=dummy_member.id,
        organization_id=dummy_organization.id,
        mcp_server_bundle_create=MCPServerBundleCreate(
            name="Test Bundle 1",
            description="Test Bundle 1 Description",
            mcp_server_configuration_ids=[
                dummy_mcp_server_configuration.id,
                dummy_mcp_server_configuration_github.id,
            ],
        ),
        bundle_key="test_bundle_1_key",
    )
    bundle_user_2 = crud.mcp_server_bundles.create_mcp_server_bundle(
        db_session=db_session,
        user_id=dummy_member_2.id,
        organization_id=dummy_organization.id,
        mcp_server_bundle_create=MCPServerBundleCreate(
            name="Test Bundle 2",
            description="Test Bundle 2 Description",
            mcp_server_configuration_ids=[
                dummy_mcp_server_configuration.id,
                dummy_mcp_server_configuration_gmail_shared.id,
            ],
        ),
        bundle_key="test_bundle_2_key",
    )

    db_session.commit()

    return (
        dummy_mcp_server_configuration,
        [team_1, team_2],
        [connected_account_user, connected_account_user_2],
        [bundle_user, bundle_user_2],
    )


@pytest.mark.parametrize(
    "connected_account_case",
    [
        OrphanConnectedAccountTestCase.remove_team_none,
        OrphanConnectedAccountTestCase.remove_team_team_1,
        OrphanConnectedAccountTestCase.remove_team_team_2,
        OrphanConnectedAccountTestCase.remove_team_all,
    ],
)
def test_on_mcp_server_configuration_allowed_teams_updated(
    db_session: Session,
    dummy_member: User,
    dummy_member_2: User,
    connected_account_case: OrphanConnectedAccountTestCase,
    dummy_mcp_server_configuration_github: MCPServerConfiguration,
    dummy_mcp_server_configuration_gmail_shared: MCPServerConfiguration,
    dummy_test_setting_1: tuple[
        MCPServerConfiguration, list[Team], list[ConnectedAccount], list[MCPServerBundle]
    ],
) -> None:
    # obtain test entities from fixture
    dummy_mcp_server_configuration, teams, connected_accounts, bundles = dummy_test_setting_1

    # Confirm the users are accessible to the MCP Server Configuration originally
    _assert_users_configuration_accessibilities(
        db_session=db_session,
        accessibilities=[
            (dummy_member.id, dummy_mcp_server_configuration.id, True),
            (dummy_member_2.id, dummy_mcp_server_configuration.id, True),
        ],
    )

    # Now remove teams based on the test parameter
    team_1, team_2 = teams[0], teams[1]
    match connected_account_case:
        case OrphanConnectedAccountTestCase.remove_team_none:
            pass  # no change
        case OrphanConnectedAccountTestCase.remove_team_team_1:
            dummy_mcp_server_configuration.allowed_teams = [uuid4(), team_2.id]  # add a random team
        case OrphanConnectedAccountTestCase.remove_team_team_2:
            dummy_mcp_server_configuration.allowed_teams = [team_1.id, uuid4()]
        case OrphanConnectedAccountTestCase.remove_team_all:
            dummy_mcp_server_configuration.allowed_teams = []

    db_session.commit()

    # Execute the orphan records removal
    removal_result = OrphanRecordsRemover(
        db_session
    ).on_mcp_server_configuration_allowed_teams_updated(
        mcp_server_configuration=dummy_mcp_server_configuration
    )

    db_session.commit()

    # Verify the results
    match connected_account_case:
        case OrphanConnectedAccountTestCase.remove_team_none:
            # Both users still have access to the MCP Server Configuration
            _assert_connected_accounts_removal(
                db_session=db_session,
                removal_result=removal_result,
                expected_removed_connected_accounts=[],
                expected_retained_connected_accounts=[
                    connected_accounts[0].id,
                    connected_accounts[1].id,
                ],
            )
            _assert_mcp_configurations_in_bundles_removal(
                db_session=db_session,
                removal_result=removal_result,
                expected_removed_mcp_configurations_in_bundles=[],
                expected_retained_mcp_configurations_in_bundles=[
                    (
                        bundles[0].id,
                        [
                            dummy_mcp_server_configuration.id,
                            dummy_mcp_server_configuration_github.id,
                        ],
                    ),
                    (
                        bundles[1].id,
                        [
                            dummy_mcp_server_configuration.id,
                            dummy_mcp_server_configuration_gmail_shared.id,
                        ],
                    ),
                ],
            )

        case OrphanConnectedAccountTestCase.remove_team_team_1:
            # dummy_member's loses access to the MCP Server Configuration
            # dummy_member_2's still accessible via team_2
            _assert_connected_accounts_removal(
                db_session=db_session,
                removal_result=removal_result,
                expected_removed_connected_accounts=[connected_accounts[0].id],
                expected_retained_connected_accounts=[connected_accounts[1].id],
            )

            _assert_mcp_configurations_in_bundles_removal(
                db_session=db_session,
                removal_result=removal_result,
                expected_removed_mcp_configurations_in_bundles=[
                    (bundles[0].id, [dummy_mcp_server_configuration.id]),
                ],
                expected_retained_mcp_configurations_in_bundles=[
                    (bundles[0].id, [dummy_mcp_server_configuration_github.id]),
                    (
                        bundles[1].id,
                        [
                            dummy_mcp_server_configuration.id,
                            dummy_mcp_server_configuration_gmail_shared.id,
                        ],
                    ),
                ],
            )

        case OrphanConnectedAccountTestCase.remove_team_team_2:
            # Both users still have access to the MCP Server Configuration
            _assert_connected_accounts_removal(
                db_session=db_session,
                removal_result=removal_result,
                expected_removed_connected_accounts=[],
                expected_retained_connected_accounts=[
                    connected_accounts[0].id,
                    connected_accounts[1].id,
                ],
            )
            _assert_mcp_configurations_in_bundles_removal(
                db_session=db_session,
                removal_result=removal_result,
                expected_removed_mcp_configurations_in_bundles=[],
                expected_retained_mcp_configurations_in_bundles=[
                    (
                        bundles[0].id,
                        [
                            dummy_mcp_server_configuration.id,
                            dummy_mcp_server_configuration_github.id,
                        ],
                    ),
                    (
                        bundles[1].id,
                        [
                            dummy_mcp_server_configuration.id,
                            dummy_mcp_server_configuration_gmail_shared.id,
                        ],
                    ),
                ],
            )

        case OrphanConnectedAccountTestCase.remove_team_all:
            # Both users lose access to the MCP Server Configuration
            _assert_connected_accounts_removal(
                db_session=db_session,
                removal_result=removal_result,
                expected_removed_connected_accounts=[
                    connected_accounts[0].id,
                    connected_accounts[1].id,
                ],
                expected_retained_connected_accounts=[],
            )
            _assert_mcp_configurations_in_bundles_removal(
                db_session=db_session,
                removal_result=removal_result,
                expected_removed_mcp_configurations_in_bundles=[
                    (bundles[0].id, [dummy_mcp_server_configuration.id]),
                    (bundles[1].id, [dummy_mcp_server_configuration.id]),
                ],
                expected_retained_mcp_configurations_in_bundles=[
                    (bundles[0].id, [dummy_mcp_server_configuration_github.id]),
                    (bundles[1].id, [dummy_mcp_server_configuration_gmail_shared.id]),
                ],
            )


def test_on_mcp_server_configuration_deleted(
    db_session: Session,
    dummy_organization: Organization,
    dummy_mcp_server_configuration_github: MCPServerConfiguration,
    dummy_mcp_server_configuration_gmail_shared: MCPServerConfiguration,
    dummy_test_setting_1: tuple[
        MCPServerConfiguration, list[Team], list[ConnectedAccount], list[MCPServerBundle]
    ],
) -> None:
    # obtain test entities from fixture
    dummy_mcp_server_configuration, _, connected_accounts, bundles = dummy_test_setting_1

    original_ids = [connected_accounts[0].id, connected_accounts[1].id]

    # Now delete the MCP Server Configuration
    crud.mcp_server_configurations.delete_mcp_server_configuration(
        db_session=db_session,
        mcp_server_configuration_id=dummy_mcp_server_configuration.id,
    )

    # Execute the orphan records removal
    removal_result = OrphanRecordsRemover(db_session).on_mcp_server_configuration_deleted(
        organization_id=dummy_organization.id,
        mcp_server_configuration_id=dummy_mcp_server_configuration.id,
    )

    db_session.commit()

    # Verify the results
    # All connected accounts should be deleted. This is done automatically by the CASCADE DELETE
    # during the MCP Server Configuration deletion, instead of deleted by the
    # OrphanRecordsRemover. So it is not returned in the removal result.
    for connected_account_id in original_ids:
        assert (
            crud.connected_accounts.get_connected_account_by_id(
                db_session=db_session,
                connected_account_id=connected_account_id,
                throw_error_if_not_found=False,
            )
            is None
        )

    _assert_mcp_configurations_in_bundles_removal(
        db_session=db_session,
        removal_result=removal_result,
        expected_removed_mcp_configurations_in_bundles=[
            (bundles[0].id, [dummy_mcp_server_configuration.id]),
            (bundles[1].id, [dummy_mcp_server_configuration.id]),
        ],
        expected_retained_mcp_configurations_in_bundles=[
            (bundles[0].id, [dummy_mcp_server_configuration_github.id]),
            (bundles[1].id, [dummy_mcp_server_configuration_gmail_shared.id]),
        ],
    )


@pytest.mark.parametrize(
    "connected_account_case",
    [
        OrphanConnectedAccountTestCase.remove_user_from_team_1,
        # OrphanConnectedAccountTestCase.remove_user_2_from_team_1,
        # OrphanConnectedAccountTestCase.remove_user_2_from_team_2,
    ],
)
def test_on_user_removed_from_team(
    db_session: Session,
    dummy_organization: Organization,
    dummy_member: User,
    dummy_member_2: User,
    dummy_test_setting_1: tuple[
        MCPServerConfiguration, list[Team], list[ConnectedAccount], list[MCPServerBundle]
    ],
    dummy_mcp_server_configuration_github: MCPServerConfiguration,
    dummy_mcp_server_configuration_gmail_shared: MCPServerConfiguration,
    connected_account_case: OrphanConnectedAccountTestCase,
) -> None:
    # obtain test entities from fixture
    dummy_mcp_server_configuration, teams, connected_accounts, bundles = dummy_test_setting_1

    team_1, team_2 = teams[0], teams[1]

    match connected_account_case:
        case OrphanConnectedAccountTestCase.remove_user_from_team_1:
            # Now remove the user from the team
            crud.teams.remove_team_member(
                db_session=db_session,
                organization_id=dummy_organization.id,
                team_id=team_1.id,
                user_id=dummy_member.id,
            )
            removed_user = dummy_member
        case OrphanConnectedAccountTestCase.remove_user_2_from_team_1:
            crud.teams.remove_team_member(
                db_session=db_session,
                organization_id=dummy_organization.id,
                team_id=team_1.id,
                user_id=dummy_member_2.id,
            )
            removed_user = dummy_member_2
        case OrphanConnectedAccountTestCase.remove_user_2_from_team_2:
            crud.teams.remove_team_member(
                db_session=db_session,
                organization_id=dummy_organization.id,
                team_id=team_2.id,
                user_id=dummy_member_2.id,
            )
            removed_user = dummy_member_2

    db_session.commit()

    # Execute the orphan records removal
    removal_result = OrphanRecordsRemover(db_session).on_user_removed_from_team(
        user_id=removed_user.id,
        organization_id=dummy_organization.id,
    )
    db_session.commit()

    logger.info(f"dummy_mcp_server_configuration: {dummy_mcp_server_configuration.id}")
    logger.info(f"team1 members: {[m.user_id for m in team_1.memberships]}")
    logger.info(f"team2 members: {[m.user_id for m in team_2.memberships]}")

    # Verify the results
    match connected_account_case:
        case OrphanConnectedAccountTestCase.remove_user_from_team_1:
            # user loses access to both dummy_mcp_server_configuration and
            # dummy_mcp_server_configuration_github
            _assert_connected_accounts_removal(
                db_session=db_session,
                removal_result=removal_result,
                expected_removed_connected_accounts=[connected_accounts[0].id],
                expected_retained_connected_accounts=[connected_accounts[1].id],
            )
            _assert_mcp_configurations_in_bundles_removal(
                db_session=db_session,
                removal_result=removal_result,
                expected_removed_mcp_configurations_in_bundles=[
                    (
                        bundles[0].id,
                        [
                            dummy_mcp_server_configuration.id,
                            dummy_mcp_server_configuration_github.id,
                        ],
                    ),
                ],
                expected_retained_mcp_configurations_in_bundles=[
                    (
                        bundles[1].id,
                        [
                            dummy_mcp_server_configuration.id,
                            dummy_mcp_server_configuration_gmail_shared.id,
                        ],
                    ),
                ],
            )
        case OrphanConnectedAccountTestCase.remove_user_2_from_team_1:
            # user2 still has access to the MCP Server Configuration
            _assert_connected_accounts_removal(
                db_session=db_session,
                removal_result=removal_result,
                expected_removed_connected_accounts=[],
                expected_retained_connected_accounts=[
                    connected_accounts[0].id,
                    connected_accounts[1].id,
                ],
            )
            _assert_mcp_configurations_in_bundles_removal(
                db_session=db_session,
                removal_result=removal_result,
                expected_removed_mcp_configurations_in_bundles=[],
                expected_retained_mcp_configurations_in_bundles=[
                    (
                        bundles[0].id,
                        [
                            dummy_mcp_server_configuration.id,
                            dummy_mcp_server_configuration_github.id,
                        ],
                    ),
                    (
                        bundles[1].id,
                        [
                            dummy_mcp_server_configuration.id,
                            dummy_mcp_server_configuration_gmail_shared.id,
                        ],
                    ),
                ],
            )
        case OrphanConnectedAccountTestCase.remove_user_2_from_team_2:
            # dummy_member_2's connected account should be retained (still on team_1)
            _assert_connected_accounts_removal(
                db_session=db_session,
                removal_result=removal_result,
                expected_removed_connected_accounts=[],
                expected_retained_connected_accounts=[
                    connected_accounts[0].id,
                    connected_accounts[1].id,
                ],
            )
            _assert_mcp_configurations_in_bundles_removal(
                db_session=db_session,
                removal_result=removal_result,
                expected_removed_mcp_configurations_in_bundles=[],
                expected_retained_mcp_configurations_in_bundles=[
                    (
                        bundles[0].id,
                        [
                            dummy_mcp_server_configuration.id,
                            dummy_mcp_server_configuration_github.id,
                        ],
                    ),
                    (
                        bundles[1].id,
                        [
                            dummy_mcp_server_configuration.id,
                            dummy_mcp_server_configuration_gmail_shared.id,
                        ],
                    ),
                ],
            )


# TODO: Add test for on_mcp_server_deleted
def test_on_mcp_server_deleted(
    db_session: Session,
    dummy_organization: Organization,
    dummy_custom_mcp_server: MCPServer,
    dummy_test_setting_1: tuple[
        MCPServerConfiguration, list[Team], list[ConnectedAccount], list[MCPServerBundle]
    ],
) -> None:
    # obtain test entities from fixture
    dummy_mcp_server_configuration, _, connected_accounts, bundles = dummy_test_setting_1

    # Store the original MCP Server Configuration ID to assert later
    original_mcp_server_configuration_id = dummy_mcp_server_configuration.id

    original_ids = [connected_accounts[0].id, connected_accounts[1].id]

    # Link the MCP Server Configuration to the dummy custom MCP Server for test setup
    dummy_mcp_server_configuration.mcp_server_id = dummy_custom_mcp_server.id

    db_session.commit()

    # Now delete the MCP Server
    crud.mcp_servers.delete_mcp_server(
        db_session=db_session,
        mcp_server_id=dummy_custom_mcp_server.id,
    )
    db_session.commit()

    # Execute the orphan records removal
    removal_result = OrphanRecordsRemover(db_session).on_mcp_server_deleted(
        organization_id=dummy_organization.id,
        mcp_server_id=dummy_custom_mcp_server.id,
    )

    # Verify the results

    # All connected accounts should be deleted. This is done automatically by the CASCADE DELETE
    # during the MCP Server Configuration deletion, instead of deleted by the
    # OrphanRecordsRemover. So it is not returned in the removal result.
    for connected_account_id in original_ids:
        assert (
            crud.connected_accounts.get_connected_account_by_id(
                db_session=db_session,
                connected_account_id=connected_account_id,
                throw_error_if_not_found=False,
            )
            is None
        )

    # All MCP Server Configurations in the bundles should be removed
    _assert_mcp_configurations_in_bundles_removal(
        db_session=db_session,
        removal_result=removal_result,
        expected_removed_mcp_configurations_in_bundles=[
            (bundles[0].id, [original_mcp_server_configuration_id]),
            (bundles[1].id, [original_mcp_server_configuration_id]),
        ],
    )

    db_session.commit()
