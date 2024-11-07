from unittest.mock import MagicMock, patch

import pytest

from goose.toolkit.google_workspace import GoogleWorkspace
from goose.tools.google_oauth_handler import GoogleOAuthHandler


@pytest.fixture
def google_workspace_toolkit():
    return GoogleWorkspace(notifier=MagicMock())


@pytest.fixture
def mock_credentials():
    mock_creds = MagicMock()
    mock_creds.token = "mock_token"
    return mock_creds


def test_google_workspace_init(google_workspace_toolkit):
    assert isinstance(google_workspace_toolkit, GoogleWorkspace)


@patch.object(GoogleOAuthHandler, "get_credentials")
def test_login(mock_get_credentials, google_workspace_toolkit, mock_credentials):
    mock_get_credentials.return_value = mock_credentials
    result = google_workspace_toolkit.login()
    assert "Successfully authenticated with Google!" in result
    assert "Access token: mock_tok..." in result


@patch.object(GoogleOAuthHandler, "get_credentials")
def test_login_error(mock_get_credentials, google_workspace_toolkit):
    mock_get_credentials.side_effect = ValueError("Test error")
    result = google_workspace_toolkit.login()
    assert "Error: Test error" in result


@patch("goose.toolkit.google_workspace.get_file_paths")
def test_file_paths(mock_get_file_paths):
    mock_get_file_paths.return_value = {
        "CLIENT_SECRETS_FILE": "/mock/home/path/.config/goose/google_credentials.json",
        "TOKEN_FILE": "/mock/home/path/.config/goose/google_oauth_token.json",
    }
    from goose.toolkit.google_workspace import get_file_paths

    file_paths = get_file_paths()
    assert file_paths["CLIENT_SECRETS_FILE"] == "/mock/home/path/.config/goose/google_credentials.json"
    assert file_paths["TOKEN_FILE"] == "/mock/home/path/.config/goose/google_oauth_token.json"


def test_list_emails(mocker, google_workspace_toolkit):
    # Mock get_file_paths
    mock_get_file_paths = mocker.patch("goose.toolkit.google_workspace.get_file_paths")
    mock_get_file_paths.return_value = {
        "CLIENT_SECRETS_FILE": "/mock/home/path/.config/goose/google_credentials.json",
        "TOKEN_FILE": "/mock/home/path/.config/goose/google_oauth_token.json",
    }

    # Mock GoogleOAuthHandler
    mock_google_oauth_handler = mocker.patch("goose.toolkit.google_workspace.GoogleOAuthHandler")
    mock_credentials = mocker.MagicMock()
    mock_google_oauth_handler.return_value.get_credentials.return_value = mock_credentials

    # Mock GmailClient
    mock_gmail_client = mocker.patch("goose.toolkit.google_workspace.GmailClient")
    mock_gmail_client.return_value.list_emails.return_value = "mock_emails"

    # Call the method
    result = google_workspace_toolkit.list_emails()

    # Assertions
    assert result == "mock_emails"
    mock_get_file_paths.assert_called_once()
    mock_google_oauth_handler.assert_called_once_with(
        "/mock/home/path/.config/goose/google_credentials.json",
        "/mock/home/path/.config/goose/google_oauth_token.json",
        ["https://www.googleapis.com/auth/gmail.readonly", "https://www.googleapis.com/auth/calendar.readonly"],
    )
    mock_google_oauth_handler.return_value.get_credentials.assert_called_once()
    mock_gmail_client.assert_called_once_with(mock_credentials)
    mock_gmail_client.return_value.list_emails.assert_called_once()
