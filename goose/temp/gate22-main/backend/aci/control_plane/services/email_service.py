import html
import textwrap
from typing import Any

import anyio
import boto3  # type: ignore[import-untyped]
from botocore.exceptions import ClientError  # type: ignore[import-untyped]

from aci.common import utils
from aci.common.logging_setup import get_logger
from aci.control_plane import config
from aci.control_plane.exceptions import EmailSendError

logger = get_logger(__name__)


EMAIL_BASE_STYLES = textwrap.dedent(
    """
    :root {
        color-scheme: light dark;
    }
    body {
        margin: 0;
        padding: 32px 0;
        background-color: #f5f5f5;
        color: #0a0a0a;
        font-family: "Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
        line-height: 1.6;
    }
    .email-wrapper {
        padding: 0 24px;
    }
    .card {
        margin: 0 auto;
        max-width: 600px;
        background-color: #ffffff;
        border-radius: 16px;
        border: 1px solid #e5e5e5;
        padding: 32px;
    }
    h1 {
        font-size: 24px;
        margin: 0 0 16px;
        color: #0a0a0a;
    }
    p {
        margin: 0 0 16px;
    }
    .button-row {
        margin: 32px 0;
        text-align: center;
    }
    .button {
        display: inline-block;
        padding: 14px 28px;
        border-radius: 999px;
        background-color: #0a0a0a;
        color: #fafafa !important;
        text-decoration: none;
        font-weight: 600;
        letter-spacing: 0.01em;
    }
    .button:hover {
        background-color: #171717;
    }
    .link {
        color: hsl(221.2, 83.2%, 53.3%);
        word-break: break-all;
        text-decoration: none;
    }
    .link:hover {
        text-decoration: underline;
    }
    .muted {
        color: #737373;
    }
    .footer {
        margin: 24px auto 0;
        max-width: 600px;
        padding: 16px 24px 0;
        border-top: 1px solid #e5e5e5;
        color: #737373;
        font-size: 13px;
        text-align: center;
    }
    @media (max-width: 480px) {
        body {
            padding: 24px 0;
        }
        .card {
            padding: 24px;
        }
    }
    @media (prefers-color-scheme: dark) {
        body {
            background-color: #0a0a0a;
            color: #fafafa;
        }
        .card {
            background-color: #171717;
            border-color: #262626;
        }
        h1 {
            color: #fafafa;
        }
        .button {
            background-color: #fafafa;
            color: #171717 !important;
        }
        .button:hover {
            background-color: #e5e5e5;
        }
        .footer {
            border-color: #262626;
            color: #a3a3a3;
        }
        .muted {
            color: #a3a3a3;
        }
        .link {
            color: hsl(212, 95%, 68%);
        }
    }
    """
).strip()


class EmailService:
    def __init__(self) -> None:
        self.charset = "UTF-8"
        self._client = boto3.client("ses")
        self._sender = f"{config.SENDER_NAME} <{config.SENDER_EMAIL}>"

    async def send_email(
        self,
        recipient: str,
        subject: str,
        body_text: str,
        body_html: str,
    ) -> dict[str, Any]:
        try:
            response = await anyio.to_thread.run_sync(
                lambda: self._client.send_email(
                    Destination={
                        "ToAddresses": [recipient],
                    },
                    Message={
                        "Body": {
                            "Html": {
                                "Charset": self.charset,
                                "Data": body_html,
                            },
                            "Text": {
                                "Charset": self.charset,
                                "Data": body_text,
                            },
                        },
                        "Subject": {
                            "Charset": self.charset,
                            "Data": subject,
                        },
                    },
                    Source=self._sender,
                )
            )
            logger.info("Email sent via SES. MessageId=%s", response.get("MessageId"))
            send_at = response.get("ResponseMetadata", {}).get("HTTPHeaders", {}).get("date")
            return {
                "email_recipient": recipient,
                "email_provider": "aws",
                "email_send_at": send_at,
                "email_reference_id": response.get("MessageId"),
            }
        except ClientError as e:
            logger.error(
                "SES send_email failed: %s",
                e.response.get("Error", {}).get("Message"),
            )
            raise EmailSendError("email provider error") from e
        except Exception as e:
            logger.error("Unexpected error sending email: %s", e)
            raise EmailSendError("unexpected error sending email") from e

    async def send_verification_email(
        self,
        recipient: str,
        user_name: str,
        verification_url: str,
    ) -> dict[str, Any]:
        subject = "Verify Your Email Address"

        expires_label = utils.format_duration_from_minutes(config.EMAIL_VERIFICATION_EXPIRE_MINUTES)

        body_text = textwrap.dedent(
            f"""
            Hi {user_name},

            Thank you for signing up! Please verify your email address by clicking the
            link below:

            {verification_url}

            This link will expire in {expires_label}.

            If you didn't create an account, you can safely ignore this email.

            Best regards,
            The Aipolabs Team
            """
        ).strip()

        safe_name = html.escape(user_name)
        safe_url = html.escape(verification_url, quote=True)

        body_html = f"""
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="utf-8">
            <meta name="viewport" content="width=device-width, initial-scale=1">
            <meta name="color-scheme" content="light dark">
            <meta name="supported-color-schemes" content="light dark">
            <style>{EMAIL_BASE_STYLES}</style>
        </head>
        <body>
            <div class="email-wrapper">
                <div class="card">
                    <h1>Verify your email</h1>
                    <p>Hi {safe_name},</p>
                    <p>
                        Thanks for signing up for Aipolabs.
                        Confirm your email address to finish setting up your account.
                    </p>
                    <p class="button-row">
                        <a href="{safe_url}" class="button">Verify email</a>
                    </p>
                    <p class="muted">This link expires in {html.escape(expires_label)}.</p>
                    <p>If the button does not work, copy and paste this link into your browser:</p>
                    <p><a href="{safe_url}" class="link">{safe_url}</a></p>
                    <p>If you didn't create an account, you can safely ignore this message.</p>
                    <p>Best regards,<br>The Aipolabs Team</p>
                </div>
                <div class="footer">
                    <p>Sent with care by Aipolabs</p>
                    <p>© 2025 Aipolabs. All rights reserved.</p>
                </div>
            </div>
        </body>
        </html>
        """

        return await self.send_email(recipient, subject, body_text, body_html)

    async def send_organization_invitation_email(
        self,
        recipient: str,
        organization_name: str,
        inviter_name: str,
        invitation_url: str,
        expires_label: str,
    ) -> dict[str, Any]:
        subject = f"You're invited to join {organization_name}"

        body_text = textwrap.dedent(
            f"""
            Hi,

            {inviter_name} has invited you to join the organization "{organization_name}" on
            ACI.dev. View the invitation below to respond:

            View invitation: {invitation_url}

            This invitation will expire in {expires_label}.

            If the link above does not work, copy and paste it into your browser.

            Best regards,
            The ACI.dev Team
            """
        ).strip()

        safe_org = html.escape(organization_name)
        safe_inviter = html.escape(inviter_name)
        safe_invitation_url = html.escape(invitation_url, quote=True)
        safe_expires = html.escape(expires_label)

        body_html = f"""
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="utf-8">
            <meta name="viewport" content="width=device-width, initial-scale=1">
            <meta name="color-scheme" content="light dark">
            <meta name="supported-color-schemes" content="light dark">
            <style>{EMAIL_BASE_STYLES}</style>
        </head>
        <body>
            <div class="email-wrapper">
                <div class="card">
                    <h1>Join {safe_org}</h1>
                    <p>Hi there,</p>
                    <p>
                        <strong>{safe_inviter}</strong> invited you to collaborate with
                        <strong>{safe_org}</strong> on ACI.dev.
                    </p>
                    <p>
                        Open the invitation to review the details and sign in.
                        Choose how you'd like to participate.
                    </p>
                    <p class="button-row">
                        <a href="{safe_invitation_url}" class="button">View invitation</a>
                    </p>
                    <p class="muted">This invitation expires in {safe_expires}.</p>
                    <p>If the button does not work, copy and paste this link into your browser:</p>
                    <p><a href="{safe_invitation_url}" class="link">{safe_invitation_url}</a></p>
                    <p>Best regards,<br>The ACI.dev Team</p>
                </div>
                <div class="footer">
                    <p>You're receiving this email because someone invited you to ACI.dev.</p>
                    <p>© 2025 ACI.dev. All rights reserved.</p>
                </div>
            </div>
        </body>
        </html>
        """

        return await self.send_email(recipient, subject, body_text, body_html)
