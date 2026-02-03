from typing import Annotated
from uuid import UUID

from fastapi import APIRouter, Depends, HTTPException, status

from aci.common import utils
from aci.common.db import crud
from aci.common.db.sql_models import BUNDLE_KEY_LENGTH
from aci.common.enums import OrganizationRole
from aci.common.logging_setup import get_logger
from aci.common.schemas.mcp_server_bundle import (
    MCPServerBundleCreate,
    MCPServerBundlePublic,
    MCPServerBundlePublicWithBundleKey,
)
from aci.common.schemas.pagination import PaginationParams, PaginationResponse
from aci.control_plane import access_control, schema_utils
from aci.control_plane import dependencies as deps
from aci.control_plane.exceptions import NotPermittedError

logger = get_logger(__name__)
router = APIRouter()


@router.post("", response_model=MCPServerBundlePublic, response_model_exclude_none=True)
async def create_mcp_server_bundle(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    body: MCPServerBundleCreate,
) -> MCPServerBundlePublic:
    # TODO: acl control
    # Only member can create MCP server bundle
    if context.act_as.role == OrganizationRole.ADMIN:
        raise HTTPException(status_code=403, detail="Forbidden")

    for mcp_server_configuration_id in body.mcp_server_configuration_ids:
        # make sure mcp_server_configuration_ids are actually in the org and user belongs
        # to a team that has access to the mcp server configuration
        access_control.check_mcp_server_config_accessibility(
            db_session=context.db_session,
            user_id=context.user_id,
            mcp_server_configuration_id=mcp_server_configuration_id,
            throw_error_if_not_permitted=True,
        )

        connected_account = crud.connected_accounts.get_connected_account_by_user_id_and_mcp_server_configuration_id(  # noqa: E501
            context.db_session,
            context.user_id,
            mcp_server_configuration_id,
        )
        if connected_account is None:
            logger.error(
                f"Connected account not found for user {context.user_id} and mcp "
                f"configuration {mcp_server_configuration_id}",
            )
            raise HTTPException(
                status_code=404,
                detail=f"Connected account not found for user {context.user_id} and "
                f"mcp server configuration {mcp_server_configuration_id}",
            )

    # Generate bundle key
    bundle_key = utils.generate_alphanumeric_string(BUNDLE_KEY_LENGTH)

    mcp_server_bundle = crud.mcp_server_bundles.create_mcp_server_bundle(
        context.db_session,
        context.user_id,
        context.act_as.organization_id,
        body,
        bundle_key,
    )

    context.db_session.commit()

    # Here the bundle key would not include in the response, because the user who created the
    # bundle is an admin and would not be able to see the bundle key
    return schema_utils.construct_mcp_server_bundle_public(context.db_session, mcp_server_bundle)


@router.get(
    "",
    response_model=PaginationResponse[MCPServerBundlePublic | MCPServerBundlePublicWithBundleKey],
    description="Only admin can see all MCP server bundles, member can only see their own MCP "
    "server bundles, bundle key is only visible to the member themselves",
)
async def list_mcp_server_bundles(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    pagination_params: Annotated[PaginationParams, Depends()],
) -> PaginationResponse[MCPServerBundlePublic | MCPServerBundlePublicWithBundleKey]:
    if context.act_as.role == OrganizationRole.ADMIN:
        mcp_server_bundles = crud.mcp_server_bundles.get_mcp_server_bundles_by_organization_id(
            context.db_session,
            context.act_as.organization_id,
            offset=pagination_params.offset,
            limit=pagination_params.limit,
        )
    else:
        mcp_server_bundles = (
            crud.mcp_server_bundles.get_mcp_server_bundles_by_user_id_and_organization_id(
                context.db_session,
                context.user_id,
                context.act_as.organization_id,
                offset=pagination_params.offset,
                limit=pagination_params.limit,
            )
        )

    # Only member can see the bundle key of their own MCP server bundles
    should_include_bundle_key = context.act_as.role == OrganizationRole.MEMBER

    return PaginationResponse[MCPServerBundlePublic | MCPServerBundlePublicWithBundleKey](
        data=[
            schema_utils.construct_mcp_server_bundle_public(
                context.db_session, mcp_server_bundle, should_include_bundle_key
            )
            for mcp_server_bundle in mcp_server_bundles
        ],
        offset=pagination_params.offset,
    )


@router.get(
    "/{mcp_server_bundle_id}",
    response_model=MCPServerBundlePublic | MCPServerBundlePublicWithBundleKey,
    description="Bundle key is only visible to member themselves",
)
async def get_mcp_server_bundle(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    mcp_server_bundle_id: UUID,
) -> MCPServerBundlePublic | MCPServerBundlePublicWithBundleKey:
    mcp_server_bundle = crud.mcp_server_bundles.get_mcp_server_bundle_by_id(
        context.db_session, mcp_server_bundle_id
    )

    if mcp_server_bundle is None:
        raise HTTPException(status_code=404, detail="MCP server bundle not found")

    # check if the MCP server bundle is under the user's org
    access_control.check_act_as_organization_role(
        context.act_as,
        requested_organization_id=mcp_server_bundle.organization_id,
        throw_error_if_not_permitted=True,
    )

    if context.act_as.role == OrganizationRole.MEMBER:
        # If user is member, check if the MCP server bundle is belongs to the member
        if mcp_server_bundle.user_id != context.user_id:
            logger.error(
                f"MCP server bundle {mcp_server_bundle_id} is not belongs to the member {context.user_id}"  # noqa: E501
            )
            raise NotPermittedError(message="Cannot access MCP server bundle")

    # Only member can see the bundle key of their own MCP server bundles
    should_include_bundle_key = context.act_as.role == OrganizationRole.MEMBER

    return schema_utils.construct_mcp_server_bundle_public(
        context.db_session, mcp_server_bundle, should_include_bundle_key
    )


@router.delete("/{mcp_server_bundle_id}", status_code=status.HTTP_200_OK)
async def delete_mcp_server_bundle(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    mcp_server_bundle_id: UUID,
) -> None:
    mcp_server_bundle = crud.mcp_server_bundles.get_mcp_server_bundle_by_id(
        context.db_session, mcp_server_bundle_id
    )
    if not mcp_server_bundle:
        raise HTTPException(status_code=404, detail="MCP server bundle not found")

    # check if the MCP server bundle is under the user's org
    access_control.check_act_as_organization_role(
        context.act_as,
        requested_organization_id=mcp_server_bundle.organization_id,
        throw_error_if_not_permitted=True,
    )

    # Member can only delete their own connected accounts
    if context.act_as.role == OrganizationRole.MEMBER:
        if mcp_server_bundle.user_id != context.user_id:
            logger.error(
                f"MCP server bundle {mcp_server_bundle_id} is not belongs to the member {context.user_id}"  # noqa: E501
            )
            raise NotPermittedError(message="Cannot delete MCP server bundle")

    # Admin can delete any connected account
    if context.act_as.role == OrganizationRole.ADMIN:
        pass

    crud.mcp_server_bundles.delete_mcp_server_bundle(context.db_session, mcp_server_bundle_id)
    context.db_session.commit()
