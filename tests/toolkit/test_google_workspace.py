import os
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


@patch.object(os.path, "expanduser")
def test_file_paths(mock_expanduser):
    mock_expanduser.return_value = "/mock/home/path"
    from goose.toolkit.google_workspace import CLIENT_SECRETS_FILE, TOKEN_FILE

    assert CLIENT_SECRETS_FILE == "/mock/home/path/.config/goose/google_credentials.json"
    assert TOKEN_FILE == "/mock/home/path/.config/goose/google_oauth_token.json"


# Uncomment and implement when GmailClient is available
# @patch('goose.toolkits.tools.gmail_client.GmailClient')
# @patch.object(GoogleOAuthHandler, 'get_credentials')
# def test_list_emails(mock_get_credentials, mock_gmail_client, google_workspace_toolkit, mock_credentials):
#     mock_get_credentials.return_value = mock_credentials
#     mock_client_instance = MagicMock()
#     mock_gmail_client.return_value = mock_client_instance
#     mock_client_instance.list_emails.return_value = ["Email 1", "Email 2"]
#
#     result = google_workspace_toolkit.list_emails()
#     assert result == ["Email 1", "Email 2"]
#     mock_gmail_client.assert_called_once_with(mock_credentials)
#     mock_client_instance.list_emails.assert_called_once()
#     mock_gmail_client.assert_called_once_with(mock_credentials)
#     mock_client_instance.list_emails.assert_called_once()
