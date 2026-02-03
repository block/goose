from unittest.mock import AsyncMock, MagicMock, patch

import pytest
from aiohttp import ClientResponseError

from src.integrations.github.api import GitHubClient


@pytest.fixture
def mock_aiohttp_session():
    with patch("aiohttp.ClientSession") as mock_session_cls:
        mock_session = AsyncMock()
        mock_session_cls.return_value = mock_session

        # Mocking __aenter__ and __aexit__ for context manager usage
        mock_session.__aenter__.return_value = mock_session
        mock_session.__aexit__.return_value = None

        # Mock request methods to be MagicMocks (not AsyncMocks) so they return the context manager directly
        mock_session.get = MagicMock()
        mock_session.post = MagicMock()
        mock_session.patch = MagicMock()
        mock_session.put = MagicMock()
        mock_session.delete = MagicMock()

        # Helper to create a mock response
        def create_mock_response(status, json_data=None, text_data=None):
            mock_response = AsyncMock()
            mock_response.status = status
            mock_response.ok = 200 <= status < 300

            # Mock json() awaitable
            async def mock_json():
                return json_data

            mock_response.json = mock_json

            # Mock text() awaitable
            async def mock_text():
                return text_data if text_data is not None else ""

            mock_response.text = mock_text

            # Mock release
            mock_response.release = MagicMock()

            # Mock raise_for_status
            def mock_raise_for_status():
                if not mock_response.ok:
                    raise ClientResponseError(
                        request_info=MagicMock(),
                        history=(),
                        status=status,
                        message="Error",
                        headers=None,
                    )

            mock_response.raise_for_status = mock_raise_for_status

            # Async context manager for response
            mock_response.__aenter__.return_value = mock_response
            mock_response.__aexit__.return_value = None

            return mock_response

        # Store the create_mock_response helper on the session object for tests to use
        mock_session.create_mock_response = create_mock_response

        yield mock_session


@pytest.fixture
def github_client(mock_aiohttp_session):
    with (
        patch("src.integrations.github.api.GitHubClient._decode_private_key", return_value="mock_key"),
        patch("src.integrations.github.api.GitHubClient._generate_jwt", return_value="mock_jwt_token"),
    ):
        client = GitHubClient()
        # Force the client to use our mock session
        client._session = mock_aiohttp_session
        yield client


@pytest.mark.asyncio
async def test_get_installation_access_token_success(github_client, mock_aiohttp_session):
    # Setup response
    mock_response = mock_aiohttp_session.create_mock_response(201, json_data={"token": "access_token"})
    mock_aiohttp_session.post.return_value = mock_response

    token = await github_client.get_installation_access_token(12345)

    assert token == "access_token"
    assert github_client._token_cache[12345] == "access_token"


@pytest.mark.asyncio
async def test_get_installation_access_token_cached(github_client, mock_aiohttp_session):
    github_client._token_cache[12345] = "cached_token"

    token = await github_client.get_installation_access_token(12345)

    assert token == "cached_token"
    mock_aiohttp_session.post.assert_not_called()


@pytest.mark.asyncio
async def test_get_installation_access_token_failure(github_client, mock_aiohttp_session):
    mock_response = mock_aiohttp_session.create_mock_response(403, text_data="Forbidden")
    mock_aiohttp_session.post.return_value = mock_response

    token = await github_client.get_installation_access_token(12345)

    assert token is None


@pytest.mark.asyncio
async def test_get_repository_success(github_client, mock_aiohttp_session):
    # Initial token mock
    mock_token_response = mock_aiohttp_session.create_mock_response(201, json_data={"token": "access_token"})

    # Repo response mock
    mock_repo_response = mock_aiohttp_session.create_mock_response(200, json_data={"full_name": "owner/repo"})

    # Side effect for sequential calls (POST token -> GET repo)
    # Note: get_installation_access_token uses POST, get_repository uses GET
    mock_aiohttp_session.post.return_value = mock_token_response
    mock_aiohttp_session.get.return_value = mock_repo_response

    repo = await github_client.get_repository("owner/repo", installation_id=123)

    assert repo == {"full_name": "owner/repo"}


@pytest.mark.asyncio
async def test_get_repository_failure(github_client, mock_aiohttp_session):
    mock_token_response = mock_aiohttp_session.create_mock_response(201, json_data={"token": "access_token"})
    mock_repo_response = mock_aiohttp_session.create_mock_response(404)

    mock_aiohttp_session.post.return_value = mock_token_response
    mock_aiohttp_session.get.return_value = mock_repo_response

    repo = await github_client.get_repository("owner/repo", installation_id=123)

    assert repo is None


@pytest.mark.asyncio
async def test_list_directory_any_auth_success(github_client, mock_aiohttp_session):
    mock_token_response = mock_aiohttp_session.create_mock_response(201, json_data={"token": "access_token"})
    mock_files_response = mock_aiohttp_session.create_mock_response(200, json_data=[{"name": "file.txt"}])

    mock_aiohttp_session.post.return_value = mock_token_response
    mock_aiohttp_session.get.return_value = mock_files_response

    files = await github_client.list_directory_any_auth("owner/repo", "path", installation_id=123)

    assert files == [{"name": "file.txt"}]


@pytest.mark.asyncio
async def test_get_file_content_success(github_client, mock_aiohttp_session):
    mock_token_response = mock_aiohttp_session.create_mock_response(201, json_data={"token": "access_token"})
    mock_content_response = mock_aiohttp_session.create_mock_response(200, text_data="content")

    mock_aiohttp_session.post.return_value = mock_token_response
    mock_aiohttp_session.get.return_value = mock_content_response

    content = await github_client.get_file_content("owner/repo", "file.txt", installation_id=123)

    assert content == "content"


@pytest.mark.asyncio
async def test_get_file_content_not_found(github_client, mock_aiohttp_session):
    mock_token_response = mock_aiohttp_session.create_mock_response(201, json_data={"token": "access_token"})
    mock_not_found_response = mock_aiohttp_session.create_mock_response(404)

    mock_aiohttp_session.post.return_value = mock_token_response
    mock_aiohttp_session.get.return_value = mock_not_found_response

    content = await github_client.get_file_content("owner/repo", "file.txt", installation_id=123)

    assert content is None


@pytest.mark.asyncio
async def test_create_check_run_success(github_client, mock_aiohttp_session):
    mock_token_response = mock_aiohttp_session.create_mock_response(201, json_data={"token": "access_token"})
    mock_check_response = mock_aiohttp_session.create_mock_response(201, json_data={"id": 1})

    # Side effect sequence: first call for token (POST), second for check run (POST)
    mock_aiohttp_session.post.side_effect = [mock_token_response, mock_check_response]

    result = await github_client.create_check_run(
        "owner/repo", "sha", "name", "completed", "success", {}, installation_id=123
    )

    assert result == {"id": 1}


@pytest.mark.asyncio
async def test_get_pull_request_success(github_client, mock_aiohttp_session):
    mock_token_response = mock_aiohttp_session.create_mock_response(201, json_data={"token": "access_token"})
    mock_pr_response = mock_aiohttp_session.create_mock_response(200, json_data={"number": 1})

    mock_aiohttp_session.post.return_value = mock_token_response
    mock_aiohttp_session.get.return_value = mock_pr_response

    pr = await github_client.get_pull_request("owner/repo", 1, installation_id=123)

    assert pr == {"number": 1}


@pytest.mark.asyncio
async def test_list_pull_requests_success(github_client, mock_aiohttp_session):
    mock_token_response = mock_aiohttp_session.create_mock_response(201, json_data={"token": "access_token"})
    mock_prs_response = mock_aiohttp_session.create_mock_response(200, json_data=[{"number": 1}])

    mock_aiohttp_session.post.return_value = mock_token_response
    mock_aiohttp_session.get.return_value = mock_prs_response

    prs = await github_client.list_pull_requests("owner/repo", installation_id=123)

    assert prs == [{"number": 1}]
