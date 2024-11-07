import base64
from datetime import datetime

from googleapiclient.discovery import build


class GmailClient:
    def __init__(self, credentials: dict) -> None:
        self.service = build("gmail", "v1", credentials=credentials)

    def list_emails(self, max_results: int = 10) -> str:
        """List the emails in the user's Gmail inbox"""
        try:
            results = self.service.users().messages().list(userId="me", maxResults=max_results).execute()
            messages = results.get("messages", [])

            if not messages:
                return "No messages found."
            else:
                output = "Recent emails:\n"
                for message in messages:
                    msg = self.service.users().messages().get(userId="me", id=message["id"]).execute()
                    subject = next(
                        (header["value"] for header in msg["payload"]["headers"] if header["name"] == "Subject"),
                        "No subject",
                    )
                    sender = next(
                        (header["value"] for header in msg["payload"]["headers"] if header["name"] == "From"),
                        "Unknown sender",
                    )
                    output += f"ID: {message['id']}\nFrom: {sender}\nSubject: {subject}\n\n"
                return output
        except Exception as e:
            return f"Error listing emails: {str(e)}"

    def get_email_content(self, email_id: str) -> str:
        """Get the contents of an email by its ID"""
        try:
            message = self.service.users().messages().get(userId="me", id=email_id, format="full").execute()

            headers = message["payload"]["headers"]
            subject = next((header["value"] for header in headers if header["name"] == "Subject"), "No subject")
            sender = next((header["value"] for header in headers if header["name"] == "From"), "Unknown sender")

            if "parts" in message["payload"]:
                parts = message["payload"]["parts"]
                body = next((part["body"]["data"] for part in parts if part["mimeType"] == "text/plain"), None)
            else:
                body = message["payload"]["body"]["data"]

            if body:
                decoded_body = base64.urlsafe_b64decode(body.encode("ASCII")).decode("utf-8")
            else:
                decoded_body = "No plain text content found in the email."

            return f"From: {sender}\nSubject: {subject}\n\nBody:\n{decoded_body}"
        except Exception as e:
            return f"Error retrieving email: {str(e)}"

    def _format_email_date(self, date_str: str) -> str:
        try:
            date_obj = datetime.fromtimestamp(int(date_str) / 1000.0)
            return date_obj.strftime("%Y-%m-%d %H:%M:%S")
        except Exception:
            return date_str

    def _get_email_content(self, msg_id: str) -> dict:
        try:
            message = self.service.users().messages().get(userId="me", id=msg_id, format="full").execute()

            headers = message["payload"]["headers"]
            subject = next((h["value"] for h in headers if h["name"].lower() == "subject"), "No Subject")
            from_header = next((h["value"] for h in headers if h["name"].lower() == "from"), "Unknown Sender")
            date = self._format_email_date(message["internalDate"])

            # Get email body
            if "parts" in message["payload"]:
                parts = message["payload"]["parts"]
                body = ""
                for part in parts:
                    if part["mimeType"] == "text/plain":
                        if "data" in part["body"]:
                            body += base64.urlsafe_b64decode(part["body"]["data"].encode("ASCII")).decode("utf-8")
            else:
                if "data" in message["payload"]["body"]:
                    # NOTE: Trunace the body to 100 characters.
                    # TODO: Add ability to look up specific emails.
                    body = base64.urlsafe_b64decode(message["payload"]["body"]["data"].encode("ASCII")).decode("utf-8")[
                        0:100
                    ]
                else:
                    body = "No content"

            return {"subject": subject, "from": from_header, "date": date, "body": body}
        except Exception as e:
            return {"error": f"Error fetching email content: {str(e)}"}

    # def list_emails(self, max_results: int = 10, output_format: str = "text") -> str:
    #     try:
    #         results = self.service.users().messages().list(userId="me", maxResults=max_results).execute()
    #         messages = results.get("messages", [])

    #         if not messages:
    #             return "No emails found."

    #         emails = []
    #         for message in messages:
    #             email_content = self._get_email_content(message["id"])
    #             emails.append(email_content)

    #         if output_format == "json":
    #             return json.dumps(emails, indent=2)

    #         # Format as text
    #         text_output = []
    #         for email in emails:
    #             text_output.append(f"\nSubject: {email['subject']}")
    #             text_output.append(f"From: {email['from']}")
    #             text_output.append(f"Date: {email['date']}")
    #             text_output.append("\nBody:")
    #             text_output.append(email["body"])
    #             text_output.append("\n" + "=" * 50)

    #         return "\n".join(text_output)

    #     except HttpError as error:
    #         raise ValueError(f"Error accessing Gmail: {str(error)}")
    #     except HttpError as error:
    #         raise ValueError(f"Error accessing Gmail: {str(error)}")
