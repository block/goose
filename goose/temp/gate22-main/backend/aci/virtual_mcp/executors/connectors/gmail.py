import base64
import json
from email.mime.text import MIMEText

from google.oauth2.credentials import Credentials
from googleapiclient.discovery import build
from mcp import types as mcp_types

from aci.common.logging_setup import get_logger
from aci.common.schemas.virtual_mcp import VirtualMCPAuthTokenData
from aci.virtual_mcp.executors.connectors.base import BaseConnector

logger = get_logger(__name__)


class Gmail(BaseConnector):
    """
    Gmail Connector.
    """

    def __init__(
        self,
        auth_token_data: VirtualMCPAuthTokenData,
    ):
        super().__init__(auth_token_data)
        self.credentials = Credentials(token=auth_token_data.token)  # type: ignore

    # TODO: support HTML type for body
    def send_email(
        self,
        sender: str,
        recipient: str,
        body: str,
        subject: str | None = None,
        cc: list[str] | None = None,
        bcc: list[str] | None = None,
    ) -> mcp_types.CallToolResult:
        """
        Send an email using Gmail API.

        Args:
            sender: Sender email address
            recipient: Recipient email address(es), comma-separated for multiple recipients
            body: Email body content
            subject: Optional email subject
            cc: Optional list of carbon copy recipients
            bcc: Optional list of blind carbon copy recipients

        Returns:
            mcp_types.CallToolResult: mcp compatible call tool result containing the response
            from the Gmail API
        """
        logger.debug("Executing send_email")

        # Create and encode the email message
        message = MIMEText(body)
        message["to"] = recipient

        if subject:
            message["subject"] = subject
        if cc:
            message["cc"] = ", ".join(cc)
        if bcc:
            message["bcc"] = ", ".join(bcc)

        # Create the final message body
        message_body = {"raw": base64.urlsafe_b64encode(message.as_bytes()).decode()}

        service = build("gmail", "v1", credentials=self.credentials)

        sent_message = service.users().messages().send(userId=sender, body=message_body).execute()  # type: ignore

        logger.debug(f"Email sent successfully, message_id={sent_message.get('id', 'unknown')}")

        # # TODO: if later found necessary, return the whole message object instead of just the id
        # return {"message_id": sent_message.get("id", "unknown")}
        result = {"message_id": sent_message.get("id", "unknown")}

        return mcp_types.CallToolResult(
            structuredContent=result,
            content=[mcp_types.TextContent(type="text", text=json.dumps(result))],
        )

    # TODO: support HTML type for body
    def drafts_create(
        self,
        sender: str,
        recipient: str,
        body: str,
        subject: str | None = None,
        cc: list[str] | None = None,
        bcc: list[str] | None = None,
    ) -> mcp_types.CallToolResult:
        """
        Create a draft email using Gmail API.

        Args:
            sender: Sender email address
            recipient: Recipient email address(es), comma-separated for multiple recipients
            body: Email body content
            subject: Optional email subject
            cc: Optional list of carbon copy recipients
            bcc: Optional list of blind carbon copy recipients

        Returns:
            mcp_types.CallToolResult: mcp compatible call tool result containing the response
            from the Gmail API
        """
        logger.debug("Executing drafts_create")

        # Create and encode the email message
        message = MIMEText(body)
        message["to"] = recipient

        if subject:
            message["subject"] = subject
        if cc:
            message["cc"] = ", ".join(cc)
        if bcc:
            message["bcc"] = ", ".join(bcc)

        # Create the message body
        message_body = {"message": {"raw": base64.urlsafe_b64encode(message.as_bytes()).decode()}}

        service = build("gmail", "v1", credentials=self.credentials)

        # Create the draft
        draft = service.users().drafts().create(userId=sender, body=message_body).execute()  # type: ignore

        logger.debug(f"Draft created successfully, draft_id={draft.get('id', 'unknown')}")

        result = {"draft_id": draft.get("id", "unknown")}

        return mcp_types.CallToolResult(
            structuredContent=result,
            content=[mcp_types.TextContent(type="text", text=json.dumps(result))],
        )

    # TODO: support HTML type for body
    def drafts_update(
        self,
        draft_id: str,
        sender: str,
        recipient: str,
        body: str,
        subject: str | None = None,
        cc: list[str] | None = None,
        bcc: list[str] | None = None,
    ) -> mcp_types.CallToolResult:
        """
        Update an existing draft email using Gmail API.

        Args:
            draft_id: ID of the draft to update
            sender: Sender email address
            recipient: Recipient email address(es), comma-separated for multiple recipients
            body: Email body content
            subject: Optional email subject
            cc: Optional list of carbon copy recipients
            bcc: Optional list of blind carbon copy recipients

        Returns:
            mcp_types.CallToolResult: mcp compatible call tool result containing the response
            from the Gmail API
        """
        logger.debug(f"Executing drafts_update for draft ID: {draft_id}")

        # Create and encode the email message
        message = MIMEText(body)
        message["to"] = recipient

        if subject:
            message["subject"] = subject
        if cc:
            message["cc"] = ", ".join(cc)
        if bcc:
            message["bcc"] = ", ".join(bcc)

        # Create the message body
        message_body = {
            "id": draft_id,
            "message": {"raw": base64.urlsafe_b64encode(message.as_bytes()).decode()},
        }

        service = build("gmail", "v1", credentials=self.credentials)

        # Update the draft
        updated_draft = (
            service.users().drafts().update(userId=sender, id=draft_id, body=message_body).execute()  # type: ignore
        )

        logger.debug(f"Draft updated successfully, draft_id={updated_draft.get('id', 'unknown')}")

        result = {"draft_id": updated_draft.get("id", "unknown")}

        return mcp_types.CallToolResult(
            structuredContent=result,
            content=[mcp_types.TextContent(type="text", text=json.dumps(result))],
        )
