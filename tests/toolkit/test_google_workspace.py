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


def test_todays_schedule(mocker, google_workspace_toolkit):
    mock_calendar_client = mocker.Mock()
    mock_calendar_client.list_events_for_today.return_value = [
        {
            "summary": "Test Event 1",
            "start": {"dateTime": "2023-05-01T09:00:00"},
            "end": {"dateTime": "2023-05-01T10:00:00"},
        },
        {
            "summary": "Test Event 2",
            "start": {"dateTime": "2023-05-01T14:00:00"},
            "end": {"dateTime": "2023-05-01T15:00:00"},
        },
    ]
    mocker.patch("goose.toolkit.google_workspace.GoogleCalendarClient", return_value=mock_calendar_client)
    mocker.patch(
        "goose.toolkit.google_workspace.get_file_paths",
        return_value={"CLIENT_SECRETS_FILE": "mock_path", "TOKEN_FILE": "mock_path"},
    )
    mocker.patch("goose.toolkit.google_workspace.GoogleOAuthHandler")

    result = google_workspace_toolkit.todays_schedule()

    assert isinstance(result, list)
    assert len(result) == 2
    assert result[0]["summary"] == "Test Event 1"
    assert result[1]["summary"] == "Test Event 2"


def test_list_calendars(mocker, google_workspace_toolkit):
    mock_calendar_client = mocker.Mock()
    mock_calendar_client.list_calendars.return_value = [
        {"summary": "Calendar 1", "id": "calendar1@example.com"},
        {"summary": "Calendar 2", "id": "calendar2@example.com"},
    ]
    mocker.patch("goose.toolkit.google_workspace.GoogleCalendarClient", return_value=mock_calendar_client)
    mocker.patch(
        "goose.toolkit.google_workspace.get_file_paths",
        return_value={"CLIENT_SECRETS_FILE": "mock_path", "TOKEN_FILE": "mock_path"},
    )
    mocker.patch("goose.toolkit.google_workspace.GoogleOAuthHandler")

    result = google_workspace_toolkit.list_calendars()

    assert isinstance(result, list)
    assert len(result) == 2
    assert result[0]["summary"] == "Calendar 1"
    assert result[1]["summary"] == "Calendar 2"
    assert result[0]["id"] == "calendar1@example.com"
    assert result[1]["id"] == "calendar2@example.com"
