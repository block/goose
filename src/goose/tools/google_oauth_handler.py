import json
import os
import webbrowser
from http.server import HTTPServer, BaseHTTPRequestHandler
import urllib.parse
from threading import Thread
import socket

from google.oauth2.credentials import Credentials
import google_auth_oauthlib.flow

class OAuthConfig:
    def __init__(self, client_secrets_file, token_file, scopes):
        self.client_secrets_file = client_secrets_file
        self.token_file = token_file
        self.scopes = scopes
        self.auth_success_message = """
        <html>
          <body>
            <h1>Authentication Successful!</h1>
            <p>You can now close this window and return to the terminal.</p>
          </body>
        </html>
        """

class OAuthCallbackHandler(BaseHTTPRequestHandler):
    def __init__(self, *args, state=None, **kwargs):
        self.state = state
        self.credentials = None
        super().__init__(*args, **kwargs)

    def do_GET(self):
        query_components = urllib.parse.parse_qs(urllib.parse.urlparse(self.path).query)

        received_state = query_components.get('state', [''])[0]
        if received_state != self.state:
            self.send_error(400, "State mismatch. Possible CSRF attack.")
            return

        code = query_components.get('code', [''])[0]
        if not code:
            self.send_error(400, "No authorization code received.")
            return

        try:
            flow = google_auth_oauthlib.flow.Flow.from_client_secrets_file(
                self.server.oauth_config.client_secrets_file,
                scopes=self.server.oauth_config.scopes,
                state=received_state
            )

            flow.redirect_uri = f"http://localhost:{self.server.server_port}/auth/google/callback/"
            flow.fetch_token(code=code)

            credentials_dict = credentials_to_dict(flow.credentials)
            with open(self.server.oauth_config.token_file, 'w') as token_file:
                json.dump(credentials_dict, token_file)

            self.send_response(200)
            self.send_header('Content-type', 'text/html')
            self.end_headers()
            self.wfile.write(self.server.oauth_config.auth_success_message.encode())

            self.server.credentials = flow.credentials

        except Exception as e:
            self.send_error(500, f"Error exchanging authorization code: {str(e)}")

    def log_message(self, format, *args):
        pass

def find_free_port() -> int:
    return 8000

def credentials_to_dict(credentials: dict) -> dict:
    return {
        'token': credentials.token,
        'refresh_token': credentials.refresh_token,
        'token_uri': credentials.token_uri,
        'client_id': credentials.client_id,
        'client_secret': credentials.client_secret,
        'scopes': credentials.scopes
    }

class GoogleOAuthHandler:
    def __init__(self, client_secrets_file, token_file, scopes):
        self.oauth_config = OAuthConfig(client_secrets_file, token_file, scopes)

    def get_credentials(self):
        if os.path.exists(self.oauth_config.token_file):
            with open(self.oauth_config.token_file, 'r') as token_file:
                creds_dict = json.load(token_file)
                return Credentials(
                    token=creds_dict['token'],
                    refresh_token=creds_dict['refresh_token'],
                    token_uri=creds_dict['token_uri'],
                    client_id=creds_dict['client_id'],
                    client_secret=creds_dict['client_secret'],
                    scopes=creds_dict['scopes']
                )

        return self._authenticate_user()

    def _save_token(self, credentials_dict):
        os.makedirs(os.path.dirname(self.oauth_config.token_file), exist_ok=True)
        with open(self.oauth_config.token_file, 'w') as token_file:
            json.dump(credentials_dict, token_file)

    def _authenticate_user(self):
        port = find_free_port()
        redirect_uri = f"http://localhost:{port}/auth/google/callback/"

        flow = google_auth_oauthlib.flow.Flow.from_client_secrets_file(
            self.oauth_config.client_secrets_file,
            scopes=self.oauth_config.scopes
        )
        flow.redirect_uri = redirect_uri

        auth_url, state = flow.authorization_url(
            access_type='offline',
            include_granted_scopes='true',
            prompt='consent'
        )

        server_address = ('', port)
        httpd = HTTPServer(server_address,
                          lambda *args, **kwargs: OAuthCallbackHandler(*args, state=state, **kwargs))
        httpd.oauth_config = self.oauth_config
        httpd.credentials = None

        server_thread = Thread(target=httpd.serve_forever)
        server_thread.daemon = True
        server_thread.start()

        print(f"Listening on port {port}")
        print("Opening browser for authentication...")
        webbrowser.open(auth_url)

        print("Waiting for authentication...")
        while httpd.credentials is None:
            pass

        httpd.shutdown()
        server_thread.join()

        credentials = httpd.credentials
        self._save_token(credentials_to_dict(credentials))

        return credentials
