import os
import string
from datetime import UTC, datetime, timedelta
from typing import Annotated, Literal
from urllib.parse import urlparse
from uuid import UUID

import favicon  # type: ignore[import-untyped]
import tldextract
from fastapi import APIRouter, Depends, HTTPException, Request
from pydantic import AnyUrl, HttpUrl
from sqlalchemy.orm import Session

from aci.common import embeddings, entitlement_utils, utils
from aci.common.db import crud
from aci.common.enums import ConnectedAccountOwnership, MCPServerTransportType, OrganizationRole
from aci.common.logging_setup import get_logger
from aci.common.openai_client import get_openai_client
from aci.common.schemas.mcp_server import (
    CustomMCPServerCreateRequest,
    MCPServerEmbeddingFields,
    MCPServerOAuth2DCRRequest,
    MCPServerOAuth2DCRResponse,
    MCPServerOAuth2DiscoveryRequest,
    MCPServerOAuth2DiscoveryResponse,
    MCPServerPartialUpdate,
    MCPServerPartialUpdateRequest,
    MCPServerPublic,
    MCPServerUpsert,
)
from aci.common.schemas.mcp_server_configuration import MCPServerConfigurationCreate
from aci.common.schemas.pagination import PaginationParams, PaginationResponse
from aci.control_plane import access_control, config, schema_utils
from aci.control_plane import dependencies as deps
from aci.control_plane.exceptions import (
    MCPServerNotFoundError,
    MCPToolsRefreshTooFrequent,
    NotPermittedError,
    OAuth2MetadataDiscoveryError,
    UsageExceeded,
)
from aci.control_plane.routes.connected_accounts import (
    CONNECTED_ACCOUNTS_OAUTH2_CALLBACK_ROUTE_NAME,
)
from aci.control_plane.services.mcp_tools.mcp_tools_manager import MCPToolsDiff, MCPToolsManager
from aci.control_plane.services.oauth2_client import (
    ClientRegistrator,
    MetadataFetcher,
    OAuthClientMetadata,
)
from aci.control_plane.services.orphan_records_remover import OrphanRecordsRemover

logger = get_logger(__name__)
router = APIRouter()


# TODO: support both query by mcp server id and name
@router.get("/{mcp_server_id}", response_model=MCPServerPublic)
async def get_mcp_server(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    mcp_server_id: UUID,
) -> MCPServerPublic:
    access_control.check_mcp_server_accessibility(
        db_session=context.db_session,
        act_as=context.act_as,
        user_id=context.user_id,
        mcp_server_id=mcp_server_id,
        throw_error_if_not_permitted=True,
    )

    mcp_server = crud.mcp_servers.get_mcp_server_by_id(
        context.db_session, mcp_server_id, throw_error_if_not_found=False
    )
    if not mcp_server:
        # TODO: should we only use custom error class here, e.g, MCPServerNotFoundError?
        raise HTTPException(status_code=404, detail="MCP server not found")

    return schema_utils.construct_mcp_server_public(mcp_server)


@router.get("", response_model=PaginationResponse[MCPServerPublic])
async def list_mcp_servers(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    pagination_params: Annotated[PaginationParams, Depends()],
) -> PaginationResponse[MCPServerPublic]:
    mcp_servers = crud.mcp_servers.list_mcp_servers(
        context.db_session,
        organization_id=context.act_as.organization_id,
        offset=pagination_params.offset,
        limit=pagination_params.limit,
    )

    return PaginationResponse(
        data=[schema_utils.construct_mcp_server_public(mcp_server) for mcp_server in mcp_servers],
        offset=pagination_params.offset,
    )


def _generate_unique_mcp_server_canonical_name(
    db_session: Session, name: str, max_trials: int = 10
) -> str:
    """
    Generate a unique MCP server canonical name. If collision happens, try max. max_trials times.
    Return None if failed.
    """
    for _ in range(max_trials):
        random_id = utils.generate_alphanumeric_string(
            8, character_pool=string.ascii_uppercase + string.digits
        )
        canonical_name = f"{name}_{random_id}"
        if not crud.mcp_servers.get_mcp_server_by_name(
            db_session, canonical_name, throw_error_if_not_found=False
        ):
            return canonical_name

    logger.error(
        f"Failed to generate a unique MCP server canonical name for {name} after {max_trials} tries"
    )
    raise Exception(
        f"Failed to generate a unique MCP server canonical name for {name} after {max_trials} tries"
    )


def _discover_mcp_server_logo(url: str) -> str:
    """
    Discover the MCP server logo from the given URL.
    """
    # It is quite common for MCP Servers that the endpoint is not returning a HTML, the url is in a
    # subdomain that does not have a favicon. We also attemp to fallback to look for root domain
    # for a relevant icon.
    parsed = urlparse(url)
    ext = tldextract.extract(url)
    root_domain = f"{parsed.scheme}://{ext.domain}.{ext.suffix}"

    user_agent = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36"  # noqa: E501

    for attempt_url in [url, root_domain]:
        try:
            icons = favicon.get(attempt_url, headers={"User-Agent": user_agent}, timeout=5.0)

            if len(icons) > 0:
                # The icons returned sometimes contain irrelvant image. Try to match using filename.
                for icon in icons:
                    filename = os.path.basename(urlparse(icon.url).path)
                    if (
                        filename.endswith(".ico")
                        or "icon" in filename.lower()
                        or "logo" in filename.lower()
                    ):
                        return str(icon.url)

                # Otherwise use the first icon returned.
                return str(icons[0].url)
        except Exception:
            logger.info(f"Failed to discover MCP server logo, url={attempt_url}")

    return config.DEFAULT_MCP_SERVER_LOGO


@router.post("", response_model=MCPServerPublic)
async def create_custom_mcp_server(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    mcp_server_data: CustomMCPServerCreateRequest,
) -> MCPServerPublic:
    access_control.check_act_as_organization_role(
        context.act_as, required_role=OrganizationRole.ADMIN
    )

    # Check if the usage exceeds the entitlement after creating the new MCP server bundle
    if config.SUBSCRIPTION_ENABLED:
        entitlement = entitlement_utils.get_organization_entitlement(
            context.db_session, context.act_as.organization_id
        )
        usage = entitlement_utils.get_organization_usage(
            context.db_session, context.act_as.organization_id
        )
        usage.custom_mcp_servers_count += 1
        if not entitlement_utils.is_entitlement_fulfilling_usage(
            entitlement=entitlement, usage=usage
        ):
            raise UsageExceeded(
                message="organization has reached usage limit. Please upgrade your subscription to continue.",  # noqa: E501
            )

    mcp_server_embedding = embeddings.generate_mcp_server_embedding(
        get_openai_client(),
        MCPServerEmbeddingFields(
            name=mcp_server_data.name,
            url=mcp_server_data.url,
            description=mcp_server_data.description,
            categories=mcp_server_data.categories,
        ),
    )

    canonical_name = _generate_unique_mcp_server_canonical_name(
        context.db_session, mcp_server_data.name
    )

    mcp_server_data.name = canonical_name

    # Logo will be attempted to be discovered in API logic if not provided
    if not mcp_server_data.logo:
        mcp_server_data.logo = _discover_mcp_server_logo(mcp_server_data.url)

    # Transport Type will be inferred from the url if not provided
    if not mcp_server_data.transport_type:
        if mcp_server_data.url.endswith("sse") and (
            mcp_server_data.url.startswith("http") or mcp_server_data.url.startswith("https")
        ):
            mcp_server_data.transport_type = MCPServerTransportType.SSE
        else:
            mcp_server_data.transport_type = MCPServerTransportType.STREAMABLE_HTTP

    mcp_server = crud.mcp_servers.create_mcp_server(
        context.db_session,
        organization_id=context.act_as.organization_id,
        mcp_server_upsert=MCPServerUpsert.model_validate(mcp_server_data.model_dump()),
        embedding=mcp_server_embedding,
    )

    # We would need an operational MCPServerConfiguration for each custom MCP server
    crud.mcp_server_configurations.create_mcp_server_configuration(
        context.db_session,
        organization_id=context.act_as.organization_id,
        mcp_server_configuration=MCPServerConfigurationCreate(
            mcp_server_id=mcp_server.id,
            name=f"Operational_Configuration_{mcp_server.name}",
            description=f"Operational MCP Server Configuration for {mcp_server.name}",
            auth_type=mcp_server_data.operational_account_auth_type,
            connected_account_ownership=ConnectedAccountOwnership.OPERATIONAL,
            all_tools_enabled=True,  # Does not matter for operational account
            enabled_tools=[],  # Does not matter for operational account
            allowed_teams=[],  # Does not matter for operational account
        ),
    )

    mcp_server_public = schema_utils.construct_mcp_server_public(mcp_server)

    context.db_session.commit()
    return mcp_server_public


@router.patch("/{mcp_server_id}", response_model=MCPServerPublic)
async def update_mcp_server(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    mcp_server_id: UUID,
    body: MCPServerPartialUpdateRequest,
) -> MCPServerPublic:
    mcp_server = crud.mcp_servers.get_mcp_server_by_id(
        context.db_session, mcp_server_id, throw_error_if_not_found=False
    )
    if not mcp_server:
        logger.error(f"MCP server {mcp_server_id} not found")
        raise MCPServerNotFoundError("MCP server not found")

    if mcp_server.organization_id is None:
        logger.error(f"MCP server {mcp_server_id} is public")
        raise NotPermittedError("MCP server is public")

    # Enforce only admin to perform this action
    access_control.check_act_as_organization_role(
        context.act_as,
        requested_organization_id=mcp_server.organization_id,
        required_role=OrganizationRole.ADMIN,
        throw_error_if_not_permitted=True,
    )

    # Re-embed if any fields that are set is part of embedding fields
    # Note: we don't check if a field "is None" because user might want to explicitly set any
    # fields to None
    embedding_fields = set(MCPServerEmbeddingFields.model_fields.keys())
    updated_embedding_fields = body.model_fields_set.intersection(embedding_fields)
    if len(updated_embedding_fields) > 0:
        # Load Embedding Fields with original mcp server data
        mcp_server_data = MCPServerEmbeddingFields.model_validate(mcp_server, from_attributes=True)

        # Replace fields that are set (instead of checking if a field is None)
        for field in updated_embedding_fields:
            setattr(mcp_server_data, field, getattr(body, field))

        mcp_server_embedding = embeddings.generate_mcp_server_embedding(
            get_openai_client(),
            mcp_server_data,
        )
    else:
        mcp_server_embedding = None

    mcp_server = crud.mcp_servers.update_mcp_server(
        context.db_session,
        mcp_server,
        # Here the `body` is validated by the `MCPServerPartialUpdateRequest` model, should not
        # contain non-nullable fields set to None
        MCPServerPartialUpdate.model_validate(body.model_dump(exclude_unset=True)),
        mcp_server_embedding,
    )

    context.db_session.commit()
    return schema_utils.construct_mcp_server_public(mcp_server)


@router.delete("/{mcp_server_id}")
async def delete_mcp_server(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    mcp_server_id: UUID,
) -> None:
    mcp_server = crud.mcp_servers.get_mcp_server_by_id(
        context.db_session, mcp_server_id, throw_error_if_not_found=False
    )
    if not mcp_server:
        logger.error(f"MCP server {mcp_server_id} not found")
        raise MCPServerNotFoundError("MCP server not found")

    if mcp_server.organization_id is None:
        logger.error(f"MCP server {mcp_server_id} is public")
        raise NotPermittedError("MCP server is public")

    # Enforce only admin to perform this action
    access_control.check_act_as_organization_role(
        context.act_as,
        requested_organization_id=mcp_server.organization_id,
        required_role=OrganizationRole.ADMIN,
        throw_error_if_not_permitted=True,
    )

    crud.mcp_servers.delete_mcp_server(context.db_session, mcp_server_id)

    removal_result = OrphanRecordsRemover(context.db_session).on_mcp_server_deleted(
        organization_id=mcp_server.organization_id,
        mcp_server_id=mcp_server_id,
    )
    logger.info(f"Orphan records removal: {removal_result}")

    context.db_session.commit()


@router.post(
    "/oauth2-discovery",
    response_model=MCPServerOAuth2DiscoveryResponse,
    description=(
        "Discover OAuth2 Metadata and optionally perform dynamic client registration (DCR) for MCP "
        "server. Note that this does not result in any record creation or update in Control Plane."
    ),
)
async def mcp_server_oauth2_discovery(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    body: MCPServerOAuth2DiscoveryRequest,
) -> MCPServerOAuth2DiscoveryResponse:
    # Enforce only admin to perform this action
    access_control.check_act_as_organization_role(
        context.act_as, required_role=OrganizationRole.ADMIN
    )

    try:
        oauth2_metadata_fetcher = MetadataFetcher(str(body.mcp_server_url))
        oauth2_metadata = oauth2_metadata_fetcher.metadata_discovery()

        # Since we only support none and client_secret_post, only return these two
        if oauth2_metadata.token_endpoint_auth_methods_supported:
            methods_supported = {"none", "client_secret_post"}.intersection(
                oauth2_metadata.token_endpoint_auth_methods_supported
            )
        else:
            methods_supported = {"none"}

        return MCPServerOAuth2DiscoveryResponse(
            authorize_url=oauth2_metadata.authorization_endpoint,
            access_token_url=oauth2_metadata.token_endpoint,
            refresh_token_url=oauth2_metadata.token_endpoint,
            registration_url=oauth2_metadata.registration_endpoint,
            token_endpoint_auth_method_supported=list(methods_supported),
        )
    except OAuth2MetadataDiscoveryError as e:
        # Return 200 with empty fields, as the action is actually success, but the discovery failed
        logger.error(
            f"Failed to discover OAuth2 metadata, mcp_server_url={body.mcp_server_url}, error={e}"
        )
        return MCPServerOAuth2DiscoveryResponse(
            authorize_url=None,
            access_token_url=None,
            refresh_token_url=None,
            registration_url=None,
            token_endpoint_auth_method_supported=[],
        )


@router.post(
    "/oauth2-dcr",
    response_model=MCPServerOAuth2DCRResponse,
    description=(
        "Perform dynamic client registration (DCR) for MCP server. Note that this does not result "
        "in any record creation or update in Control Plane."
    ),
)
async def mcp_server_oauth2_dcr(
    request: Request,
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    body: MCPServerOAuth2DCRRequest,
) -> MCPServerOAuth2DCRResponse:
    # Enforce only admin to perform this action
    access_control.check_act_as_organization_role(
        context.act_as, required_role=OrganizationRole.ADMIN
    )

    # For whitelabeling purposes, we allow user to provide custom callback URL for their MCP OAuth2
    # flow. If not provided, we use the default callback URL in our API.
    path = request.url_for(CONNECTED_ACCOUNTS_OAUTH2_CALLBACK_ROUTE_NAME).path
    redirect_uris: list[AnyUrl] = [HttpUrl(f"{config.CONTROL_PLANE_BASE_URL}{path}")]

    # We currently only support none and client_secret_post.
    # TODO: support `client_secret_basic`
    token_endpoint_auth_method: Literal["none", "client_secret_post"]
    if "client_secret_post" in (body.token_endpoint_auth_method_supported or []):
        token_endpoint_auth_method = "client_secret_post"
    else:
        token_endpoint_auth_method = "none"

    oauth2_client_registrator = ClientRegistrator(
        body.mcp_server_url,
        client_metadata=OAuthClientMetadata(
            redirect_uris=redirect_uris,
            token_endpoint_auth_method=token_endpoint_auth_method,
            grant_types=["authorization_code", "refresh_token"],
            response_types=["code"],
            scope="",  # TODO: discover default scope
        ),
        registration_endpoint=body.registration_url,
    )
    client_info = oauth2_client_registrator.dynamic_client_registration()
    return MCPServerOAuth2DCRResponse(
        token_endpoint_auth_method=token_endpoint_auth_method,
        client_id=client_info.client_id,
        client_secret=client_info.client_secret,
    )


@router.post("/{mcp_server_id}/refresh-tools")
async def refresh_mcp_server_tools(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    mcp_server_id: UUID,
) -> MCPToolsDiff:
    mcp_server = crud.mcp_servers.get_mcp_server_by_id(
        context.db_session, mcp_server_id, throw_error_if_not_found=False
    )

    if not mcp_server:
        raise HTTPException(status_code=404, detail="MCP server not found")

    if mcp_server.organization_id is None:
        raise HTTPException(status_code=403, detail="MCP server is public")

    # Enforce only admin to perform this action
    access_control.check_act_as_organization_role(
        context.act_as,
        requested_organization_id=mcp_server.organization_id,
        required_role=OrganizationRole.ADMIN,
        throw_error_if_not_permitted=True,
    )

    # Basic rate limiting by checking the last synced at time
    # TODO: Handle case if last_synced_at is null and keeps failing, in this case this check does
    # not work.
    if mcp_server.last_synced_at is not None:
        if mcp_server.last_synced_at + timedelta(minutes=1) > datetime.now(UTC):
            logger.error(
                f"MCP server {mcp_server.name} has been refreshed too frequently, last_synced_at={mcp_server.last_synced_at}"  # noqa: E501
            )
            raise MCPToolsRefreshTooFrequent(
                f"MCP server {mcp_server.name} has been refreshed too frequently"
            )

    mcp_tools_diff = await MCPToolsManager(mcp_server).refresh_mcp_tools(context.db_session)

    context.db_session.commit()

    return mcp_tools_diff
