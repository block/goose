import pytest
from fastapi.testclient import TestClient
from sqlalchemy.orm import Session

from aci.common.db import crud
from aci.common.db.sql_models import MCPServerBundle, User
from aci.common.schemas.mcp_server_bundle import (
    MCPServerBundlePublic,
    MCPServerBundlePublicWithBundleKey,
)
from aci.common.schemas.pagination import PaginationResponse
from aci.control_plane import config


@pytest.mark.parametrize(
    "access_token_fixture",
    [
        "dummy_access_token_no_orgs",
        "dummy_access_token_admin",
        "dummy_access_token_member",
        "dummy_access_token_admin_act_as_member",
    ],
)
@pytest.mark.parametrize("offset", [None, 0, 10])
def test_list_mcp_server_bundles(
    test_client: TestClient,
    request: pytest.FixtureRequest,
    access_token_fixture: str,
    dummy_user: User,
    dummy_mcp_server_bundles: list[MCPServerBundle],
    offset: int | None,
) -> None:
    access_token = request.getfixturevalue(access_token_fixture)

    params = {}
    if offset is not None:
        params["offset"] = offset

    response = test_client.get(
        config.ROUTER_PREFIX_MCP_SERVER_BUNDLES,
        headers={"Authorization": f"Bearer {access_token}"},
        params=params,
    )

    if access_token_fixture == "dummy_access_token_no_orgs":
        assert response.status_code == 403
        return

    if offset is None or offset == 0:
        if access_token_fixture == "dummy_access_token_admin":
            # Should see all the MCP server bundles in the organization
            assert response.status_code == 200

            paginated_response = PaginationResponse[MCPServerBundlePublic].model_validate(
                response.json(),
            )

            assert len(paginated_response.data) == len(dummy_mcp_server_bundles)
            assert all(
                not hasattr(response_item, "bundle_key")
                for response_item in paginated_response.data
            )
        elif access_token_fixture in [
            "dummy_access_token_member",
            "dummy_access_token_admin_act_as_member",
        ]:
            # Should only see the MCP server bundles that the user has
            assert response.status_code == 200

            paginated_response_with_key = PaginationResponse[
                MCPServerBundlePublicWithBundleKey
            ].model_validate(
                response.json(),
            )

            assert len(paginated_response_with_key.data) == 2
            assert all(
                response_item.user_id == dummy_user.id
                for response_item in paginated_response_with_key.data
            )
            assert all(
                hasattr(response_item, "bundle_key")
                for response_item in paginated_response_with_key.data
            )
        else:
            raise Exception("Untested access token fixture")
    else:
        # shows nothing because offset should be larger than the total test MCP server bundles
        assert len(response.json()["data"]) == 0


@pytest.mark.parametrize(
    "access_token_fixture",
    [
        "dummy_access_token_no_orgs",
        "dummy_access_token_admin",
        "dummy_access_token_member",
        "dummy_access_token_admin_act_as_member",
        "dummy_access_token_another_org",
    ],
)
@pytest.mark.parametrize("is_own_mcp_server_bundle", [True, False])
def test_get_mcp_server_bundle(
    test_client: TestClient,
    db_session: Session,
    request: pytest.FixtureRequest,
    access_token_fixture: str,
    dummy_user: User,
    dummy_mcp_server_bundles: list[MCPServerBundle],
    is_own_mcp_server_bundle: bool,
) -> None:
    access_token = request.getfixturevalue(access_token_fixture)

    # Find the target MCP server bundle for testing
    if is_own_mcp_server_bundle:
        target_mcp_server_bundle = next(
            mcp_server_bundle
            for mcp_server_bundle in dummy_mcp_server_bundles
            if mcp_server_bundle.user_id == dummy_user.id
        )
    else:
        target_mcp_server_bundle = next(
            mcp_server_bundle
            for mcp_server_bundle in dummy_mcp_server_bundles
            if mcp_server_bundle.user_id != dummy_user.id
        )

    db_session.commit()

    response = test_client.get(
        f"{config.ROUTER_PREFIX_MCP_SERVER_BUNDLES}/{target_mcp_server_bundle.id}",
        headers={"Authorization": f"Bearer {access_token}"},
    )

    if access_token_fixture in ["dummy_access_token_no_orgs", "dummy_access_token_another_org"]:
        assert response.status_code == 403
        return

    # Admin can see all MCP server bundles
    elif access_token_fixture == "dummy_access_token_admin":
        assert response.status_code == 200
        mcp_server_bundle = MCPServerBundlePublic.model_validate(response.json())
        assert mcp_server_bundle.id == target_mcp_server_bundle.id
        assert not hasattr(mcp_server_bundle, "bundle_key")

    elif access_token_fixture in [
        "dummy_access_token_member",
        "dummy_access_token_admin_act_as_member",
    ]:
        if is_own_mcp_server_bundle:
            # Member can see their own MCP server bundles only
            assert response.status_code == 200
            mcp_server_bundle = MCPServerBundlePublicWithBundleKey.model_validate(response.json())
            assert mcp_server_bundle.id == target_mcp_server_bundle.id
            assert mcp_server_bundle.bundle_key is not None
        else:
            # Should not see any MCP server bundle
            assert response.status_code == 403
            assert response.json()["error"].startswith("Not permitted")
    else:
        raise Exception("Untested access token fixture")


@pytest.mark.parametrize(
    "access_token_fixture",
    [
        "dummy_access_token_no_orgs",
        "dummy_access_token_admin",
        "dummy_access_token_member",
        "dummy_access_token_admin_act_as_member",
        "dummy_access_token_another_org",
    ],
)
@pytest.mark.parametrize("is_own_mcp_server_bundle", [True, False])
def test_delete_mcp_server_bundle(
    test_client: TestClient,
    db_session: Session,
    request: pytest.FixtureRequest,
    access_token_fixture: str,
    dummy_user: User,
    dummy_mcp_server_bundles: list[MCPServerBundle],
    is_own_mcp_server_bundle: bool,
) -> None:
    access_token = request.getfixturevalue(access_token_fixture)

    # Find the target MCP server bundle for testing
    if is_own_mcp_server_bundle:
        target_mcp_server_bundle = next(
            mcp_server_bundle
            for mcp_server_bundle in dummy_mcp_server_bundles
            if mcp_server_bundle.user_id == dummy_user.id
        )
    else:
        target_mcp_server_bundle = next(
            mcp_server_bundle
            for mcp_server_bundle in dummy_mcp_server_bundles
            if mcp_server_bundle.user_id != dummy_user.id
        )

    db_session.commit()

    response = test_client.delete(
        f"{config.ROUTER_PREFIX_MCP_SERVER_BUNDLES}/{target_mcp_server_bundle.id}",
        headers={"Authorization": f"Bearer {access_token}"},
    )

    if access_token_fixture in ["dummy_access_token_no_orgs", "dummy_access_token_another_org"]:
        assert response.status_code == 403
        return

    # Admin can delete any MCP server bundles in the organization
    elif access_token_fixture == "dummy_access_token_admin":
        assert response.status_code == 200

        # Check if the MCP server bundle is deleted in db
        assert (
            crud.mcp_server_bundles.get_mcp_server_bundle_by_id(
                db_session, target_mcp_server_bundle.id
            )
            is None
        )

    elif access_token_fixture in [
        "dummy_access_token_member",
        "dummy_access_token_admin_act_as_member",
    ]:
        if is_own_mcp_server_bundle:
            # Member can see their own MCP server bundles only
            assert response.status_code == 200
            assert (
                crud.mcp_server_bundles.get_mcp_server_bundle_by_id(
                    db_session, target_mcp_server_bundle.id
                )
                is None
            )
        else:
            # Should not see any MCP server bundle
            assert response.status_code == 403
            assert response.json()["error"].startswith("Not permitted")
    else:
        raise Exception("Untested access token fixture")
