from dataclasses import dataclass
from uuid import UUID

from sqlalchemy.orm import Session

from aci.common.db import crud
from aci.common.db.sql_models import ConnectedAccount, MCPServerBundle, MCPServerConfiguration, User
from aci.common.enums import ConnectedAccountOwnership
from aci.common.logging_setup import get_logger
from aci.control_plane import access_control

logger = get_logger(__name__)


@dataclass
class OrphanConnectedAccount:
    id: UUID


@dataclass
class OrphanMCPBundle:
    id: UUID


@dataclass
class OrphanMCPServerConfiguration:
    id: UUID


@dataclass
class OrphanMCPServerConfigurationInBundle:
    bundle_id: UUID
    configuration_id: UUID


@dataclass
class OrphanRecordsRemoval:
    connected_accounts: list[OrphanConnectedAccount] | None = None
    mcp_bundles: list[OrphanMCPBundle] | None = None
    mcp_configurations: list[OrphanMCPServerConfiguration] | None = None
    mcp_configurations_in_bundles: list[OrphanMCPServerConfigurationInBundle] | None = None


class OrphanRecordsRemover:
    """
    This class is a centralised place to remove orphan records from the database.

    Orphan records are records that are not associated with any other records and not accessible by
    users after parent / linked entities are deleted.

    **Whether or not a user is accessible to a MCP Server Configuration** is defined in
    `check_mcp_server_config_accessibility()` in `aci/control_plane/access_control.py`.

    Events that may cause orphan records:
    - MCP Server deleted
    - MCP Server Configuration's allowed_teams updated
    - MCP Server Configuration deleted
    - User removed from a team
    - Team deleted
    - User removed from an organization

    (See docs: https://www.notion.so/Orphan-Records-Removal-List-27d8378d6a4780c1a1e5cc3b9b1cc812?source=copy_link)
    """

    def __init__(self, db_session: Session):
        self.db_session = db_session

    def on_mcp_server_configuration_allowed_teams_updated(
        self, mcp_server_configuration: MCPServerConfiguration
    ) -> OrphanRecordsRemoval:
        """
        Orphan records removal function for when a MCP Server Configuration's allowed_teams is
        updated.

        Delete the Connected Accounts that became inaccessible to the MCP Server Configuration
        if the user of the Connected Account no longer accessible to the MCP Server Configuration.

        Iterate through all MCP Bundles in the organization:
            - Remove the MCP Server Configuration from the MCP Bundle if the user of the MCP Bundle
            no longer accessible to the MCP Server Configuration
        """
        removal = OrphanRecordsRemoval()

        # Delete the Connected Accounts that became inaccessible to the MCP Server Configuration
        # if the user of the Connected Account no longer accessible to the MCP Server Configuration.
        connected_accounts = (
            crud.connected_accounts.get_connected_accounts_by_mcp_server_configuration_id(
                db_session=self.db_session,
                mcp_server_configuration_id=mcp_server_configuration.id,
            )
        )
        removal.connected_accounts = self._clean_orphan_connected_accounts(connected_accounts)

        # Remove the MCP Server Configuration from the MCP Bundles if the user of the MCP Bundle
        # no longer accessible to the MCP Server Configuration
        removal.mcp_configurations_in_bundles = []
        mcp_server_bundles = crud.mcp_server_bundles.get_mcp_server_bundles_by_organization_id_and_contains_mcp_server_configuration_id(  # noqa: E501
            db_session=self.db_session,
            organization_id=mcp_server_configuration.organization_id,
            mcp_server_configuration_id=mcp_server_configuration.id,
        )
        for mcp_server_bundle in mcp_server_bundles:
            removal.mcp_configurations_in_bundles.extend(
                self._clean_orphan_configurations_in_bundles(
                    mcp_server_bundle=mcp_server_bundle,
                )
            )
        return removal

    def on_mcp_server_configuration_deleted(
        self, organization_id: UUID, mcp_server_configuration_id: UUID
    ) -> OrphanRecordsRemoval:
        """
        Orphan records removal function for when a MCP Server Configuration is deleted

        - Delete all Connected Accounts associated with the MCP Server Configuration
        - Iterate through all MCP Bundles in the organization:
            - Remove the MCP Server Configuration in the MCP Bundles if the MCP Bundle contains it
        """
        # Remove all ConnectedAccount under this MCP server configuration
        # The ConnectedAccount is deleted automatically by the CASCADE DELETE when the MCP Server
        # Configuration is deleted, so we do not need to delete it here
        remaining = crud.connected_accounts.get_connected_accounts_by_mcp_server_configuration_id(
            db_session=self.db_session,
            mcp_server_configuration_id=mcp_server_configuration_id,
        )
        if len(remaining) > 0:
            # This should not happen normally if cascade delete is working properly
            logger.error(
                f"Connected Accounts remained after deletion of MCP Server Configuration "
                f"{mcp_server_configuration_id}. Connected Accounts: {[ca.id for ca in remaining]}"
            )

        # Remove the MCP server configuration from all MCPServerBundle in the organization
        orphan_mcp_configurations_in_bundles = []
        mcp_server_bundles = crud.mcp_server_bundles.get_mcp_server_bundles_by_organization_id_and_contains_mcp_server_configuration_id(  # noqa: E501
            db_session=self.db_session,
            organization_id=organization_id,
            mcp_server_configuration_id=mcp_server_configuration_id,
        )
        for mcp_server_bundle in mcp_server_bundles:
            orphan_mcp_configurations_in_bundles.append(
                OrphanMCPServerConfigurationInBundle(
                    bundle_id=mcp_server_bundle.id,
                    configuration_id=mcp_server_configuration_id,
                )
            )
            self._remove_configuration_id_from_bundle(
                mcp_server_bundle=mcp_server_bundle,
                mcp_server_configuration_id=mcp_server_configuration_id,
            )

        return OrphanRecordsRemoval(
            mcp_configurations_in_bundles=orphan_mcp_configurations_in_bundles,
        )

    def on_user_removed_from_team(
        self, user_id: UUID, organization_id: UUID
    ) -> OrphanRecordsRemoval:
        """
        Orphan records removal function for when a user is removed from a team

        **Team ID is not needed because we will check all records the user are associated with**

        Iterate through all Connected Accounts of the user:
            - Delete the Connected Account if the MCP Server Configuration of the Connected Account
              is no longer accessible by the user

        Remove the MCP Server Configuration from the MCP Bundles if the user is no longer
        accessible to the MCP Server Configuration
        """
        removal = OrphanRecordsRemoval()

        # Delete the Connected Account if the MCP Server Configuration of the Connected Account
        # is no longer accessible by the user
        connected_accounts = crud.connected_accounts.get_connected_accounts_by_user_id(
            db_session=self.db_session,
            user_id=user_id,
        )
        removal.connected_accounts = self._clean_orphan_connected_accounts(connected_accounts)

        # Remove the MCP Server Configuration from the MCP Bundles if the user is no longer
        # accessible to the MCP Server Configuration
        removal.mcp_configurations_in_bundles = []
        mcp_server_bundles = (
            crud.mcp_server_bundles.get_mcp_server_bundles_by_user_id_and_organization_id(
                db_session=self.db_session,
                user_id=user_id,
                organization_id=organization_id,
            )
        )
        for mcp_server_bundle in mcp_server_bundles:
            removal.mcp_configurations_in_bundles.extend(
                self._clean_orphan_configurations_in_bundles(
                    mcp_server_bundle=mcp_server_bundle,
                )
            )

        return removal

    def on_team_deleted(
        self, organization_id: UUID, team_members: list[User]
    ) -> OrphanRecordsRemoval:
        """
        Orphan records removal function for when a team is deleted

        This function bascially applies the same orphan removal logic as if users are removed from
        the team one by one
        """
        # orphan_mcp_configurations_in_bundles = []

        # for user in team_members:
        #     orphan_mcp_configurations_in_bundles.extend(
        #         self._remove_orphan_configurations_from_bundles_on_user_removed_from_team(
        #             user.id, organization_id
        #         )
        #     )
        #     self.on_user_removed_from_team(user.id, organization_id)

        # return OrphanRecordsRemoval(
        #     mcp_configurations_in_bundles=orphan_mcp_configurations_in_bundles,
        # )
        raise NotImplementedError("Not implemented")

    def on_user_removed_from_organization(self, user_id: UUID, organization_id: UUID) -> None:
        """
        Orphan records removal function for when a user is removed from an organization

        - Delete all connected accounts associated with the user in the organization
        - Delete all MCP Bundles associated with the user in the organization
        """
        raise NotImplementedError("Not implemented")

    def on_mcp_server_deleted(
        self, organization_id: UUID, mcp_server_id: UUID
    ) -> OrphanRecordsRemoval:
        """
        Orphan records removal function for when a MCP Server is deleted

        - Delete all MCP Server Configurations under the MCP Server
        - Delete all Connected Accounts that connects with any of the MCP Server Configurations
          under the MCP Server
        - Remove the MCP Server Configuration from any MCP Bundles in the organization that contains
          a MCP Server Configuration using this MCP Server
        - Delete all MCP Tools records under the MCP Server
        """

        orphan_mcp_configurations_in_bundles = []

        # Delete all MCP Server Configurations under the MCP Server
        # This is done automatically by the CASCADE DELETE when deleting MCP Server, defined in
        # `sql_models.py`, so we do not need to do anything here.
        remaining_configs = crud.mcp_server_configurations.get_mcp_server_configurations(
            db_session=self.db_session,
            organization_id=organization_id,
            mcp_server_id=mcp_server_id,
        )
        if len(remaining_configs) > 0:
            # This should not happen normally if cascade delete is working properly
            logger.error(
                f"MCP Server Configurations remained after deletion of MCP Server {mcp_server_id}. "
                f"MCP Server Configurations: {[config.id for config in remaining_configs]}"
            )

        # Delete all Connected Accounts that connects with any of the MCP Server Configurations
        # This is done automatically by the CASCADE DELETE when deleting MCP Server Configurations,
        # defined in `sql_models.py`, so we do not need to do anything here.
        remaining_connected_accounts = (
            crud.connected_accounts.get_connected_accounts_by_mcp_server_id(
                db_session=self.db_session,
                mcp_server_id=mcp_server_id,
            )
        )
        if len(remaining_connected_accounts) > 0:
            # This should not happen normally if cascade delete is working properly
            logger.error(
                f"Connected Accounts remained after deletion of MCP Server {mcp_server_id}. "
                f"Connected Accounts: {[ca.id for ca in remaining_connected_accounts]}"
            )

        # Remove the MCP Server Configuration from any MCP Bundles in the organization containing it
        # Since we don't have the MCP Server Configuration, we need to iterate through all MCP
        # Server Bundles and check if it contains any non-existence MCP Server Configuration.
        mcp_server_bundles = crud.mcp_server_bundles.get_mcp_server_bundles_by_organization_id(
            db_session=self.db_session, organization_id=organization_id
        )
        for mcp_server_bundle in mcp_server_bundles:
            orphan_mcp_configurations_in_bundles.extend(
                self._clean_orphan_configurations_in_bundles(
                    mcp_server_bundle=mcp_server_bundle,
                )
            )

        # Delete all MCP Tools records under the MCP Server is done automatically by the CASCADE
        # DELETE when deleting MCP Server, defined in `sql_models.py`, so we do not need to do
        # anything here
        remaining_mcp_tools = crud.mcp_tools.get_mcp_tools_by_mcp_server_id(
            db_session=self.db_session,
            mcp_server_id=mcp_server_id,
        )
        if len(remaining_mcp_tools) > 0:
            # This should not happen normally if cascade delete is working properly
            logger.error(
                f"MCP Tools remained after deletion of MCP Server {mcp_server_id}. "
                f"MCP Tools: {[tool.id for tool in remaining_mcp_tools]}"
            )

        return OrphanRecordsRemoval(
            mcp_configurations_in_bundles=orphan_mcp_configurations_in_bundles,
        )

    def _remove_configuration_id_from_bundle(
        self, mcp_server_bundle: MCPServerBundle, mcp_server_configuration_id: UUID
    ) -> None:
        """
        Helper function to remove a configuration id from a MCP Server Bundle
        """
        updated_config_ids = list(dict.fromkeys(mcp_server_bundle.mcp_server_configuration_ids))
        updated_config_ids.remove(mcp_server_configuration_id)

        crud.mcp_server_bundles.update_mcp_server_bundle_configuration_ids(
            db_session=self.db_session,
            mcp_server_bundle_id=mcp_server_bundle.id,
            update_mcp_server_bundle_configuration_ids=updated_config_ids,
        )

    def _clean_orphan_connected_accounts(
        self,
        connected_accounts: list[ConnectedAccount],
    ) -> list[OrphanConnectedAccount]:
        """
        Helper function to clean up orphan Connected Accounts

        Check for the given connected accounts, delete them if:
        - if the owner of the connected account has no access to the MCP Server Configuration the
        connected account is associated with
        """
        orphan_connected_accounts = []
        for connected_account in connected_accounts:
            # If the connected_account_ownership is not individual type, the connected account will
            # be shared, it will not be orphaned by user losing access to the MCP Server
            # Configuration.
            if connected_account.ownership != ConnectedAccountOwnership.INDIVIDUAL:
                continue

            accessible = access_control.check_mcp_server_config_accessibility(
                db_session=self.db_session,
                user_id=connected_account.user_id,
                mcp_server_configuration_id=connected_account.mcp_server_configuration_id,
                throw_error_if_not_permitted=False,
            )
            if not accessible:
                logger.info(f"Deleting Connected Account {connected_account.id}")
                orphan_connected_accounts.append(OrphanConnectedAccount(id=connected_account.id))
                crud.connected_accounts.delete_connected_account(
                    db_session=self.db_session,
                    connected_account_id=connected_account.id,
                )
        return orphan_connected_accounts

    def _clean_orphan_configurations_in_bundles(
        self, mcp_server_bundle: MCPServerBundle
    ) -> list[OrphanMCPServerConfigurationInBundle]:
        """
        Helper function to clean up orphan MCP Server Configurations in Bundles

        Check every MCP Server Configuration IDs of the bundle, remove it from the bundle if:
        - if the MCP Server Configuration does not exists
        - if the owner of the bundle has no access to the MCP Server Configuration
        """
        orphan_mcp_configurations_in_bundles = []
        for mcp_configuration_id in mcp_server_bundle.mcp_server_configuration_ids:
            should_remove = False
            if not crud.mcp_server_configurations.get_mcp_server_configuration_by_id(
                self.db_session,
                mcp_configuration_id,
                throw_error_if_not_found=False,
            ):
                should_remove = True
            elif not access_control.check_mcp_server_config_accessibility(
                db_session=self.db_session,
                user_id=mcp_server_bundle.user_id,
                mcp_server_configuration_id=mcp_configuration_id,
                throw_error_if_not_permitted=False,
            ):
                should_remove = True

            if should_remove:
                logger.info(
                    f"Removing MCP Server Configuration {mcp_configuration_id} from bundle {mcp_server_bundle.id}"  # noqa: E501
                )
                orphan_mcp_configurations_in_bundles.append(
                    OrphanMCPServerConfigurationInBundle(
                        bundle_id=mcp_server_bundle.id,
                        configuration_id=mcp_configuration_id,
                    )
                )
                self._remove_configuration_id_from_bundle(
                    mcp_server_bundle=mcp_server_bundle,
                    mcp_server_configuration_id=mcp_configuration_id,
                )
        return orphan_mcp_configurations_in_bundles
