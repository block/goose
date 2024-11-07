import os

from exchange import Message  # type: ignore

from goose.toolkit.base import Toolkit, tool
from goose.tools.gmail_client import GmailClient
from goose.tools.google_calendar_client import GoogleCalendarClient
from goose.tools.google_oauth_handler import GoogleOAuthHandler


def get_file_paths() -> dict[str, str]:
    return {
        "CLIENT_SECRETS_FILE": os.path.expanduser("~/.config/goose/google_credentials.json"),
        "TOKEN_FILE": os.path.expanduser("~/.config/goose/google_oauth_token.json"),
    }


SCOPES = ["https://www.googleapis.com/auth/gmail.readonly", "https://www.googleapis.com/auth/calendar.readonly"]


class GoogleWorkspace(Toolkit):
    """A toolkit for integrating with Google APIs"""

    def system(self) -> str:
        """Retrieve detailed configuration and procedural guidelines for Jira operations"""
        template_content = Message.load("prompts/google_workspace.jinja").text
        return template_content

    def login(self) -> str:
        try:
            file_paths = get_file_paths()
            oauth_handler = GoogleOAuthHandler(file_paths["CLIENT_SECRETS_FILE"], file_paths["TOKEN_FILE"], SCOPES)
            credentials = oauth_handler.get_credentials()
            return f"Successfully authenticated with Google! Access token: {credentials.token[:8]}..."
        except Exception as e:
            return f"Error: {str(e)}"

    @tool
    def list_emails(self) -> str:
        try:
            file_paths = get_file_paths()
            oauth_handler = GoogleOAuthHandler(file_paths["CLIENT_SECRETS_FILE"], file_paths["TOKEN_FILE"], SCOPES)
            credentials = oauth_handler.get_credentials()
            gmail_client = GmailClient(credentials)
            emails = gmail_client.list_emails()
            return emails
        except ValueError as e:
            return f"Error: {str(e)}"
        except Exception as e:
            return f"An unexpected error occurred: {str(e)}"

    @tool
    def todays_schedule(self) -> str:
        try:
            file_paths = get_file_paths()
            oauth_handler = GoogleOAuthHandler(file_paths["CLIENT_SECRETS_FILE"], file_paths["TOKEN_FILE"], SCOPES)
            credentials = oauth_handler.get_credentials()
            calendar_client = GoogleCalendarClient(credentials)
            schedule = calendar_client.list_events_for_today()
            return schedule
        except ValueError as e:
            return f"Error: {str(e)}"
        except Exception as e:
            return f"An unexpected error occurred: {str(e)}"
