import os

from goose.toolkit import Toolkit
from goose.tools.google_oauth_handler import GoogleOAuthHandler

# Note: We need to implement or import GmailClient
# from .tools.gmail_client import GmailClient


SCOPES = ["https://www.googleapis.com/auth/gmail.readonly"]
CLIENT_SECRETS_FILE = os.path.expanduser("~/.config/goose/google_credentials.json")
TOKEN_FILE = os.path.expanduser("~/.config/goose/google_oauth_token.json")


class GoogleWorkspace(Toolkit):
    """A toolkit for integrating with Google APIs"""

    def login() -> str:
        try:
            oauth_handler = GoogleOAuthHandler(CLIENT_SECRETS_FILE, TOKEN_FILE, SCOPES)
            credentials = oauth_handler.get_credentials()
            print("Successfully authenticated with Google!")
            print(f"Access token: {credentials.token[:10]}...")
        except ValueError as e:
            print(f"Error: {str(e)}")
        except Exception as e:
            print(f"An unexpected error occurred: {str(e)}")

    # @tool
    # def list_emails(self) -> str:
    #     try:
    #         oauth_handler = GoogleOAuthHandler(CLIENT_SECRETS_FILE, TOKEN_FILE, SCOPES)
    #         credentials = oauth_handler.get_credentials()
    #         gmail_client = GmailClient(credentials)
    #         emails = gmail_client.list_emails()
    #         return emails
    #     except ValueError as e:
    #         return f"Error: {str(e)}"
    #     except Exception as e:
    #         return f"An unexpected error occurred: {str(e)}"
