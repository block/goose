import os
import requests
import base64
import re
from sendgrid import SendGridAPIClient
from sendgrid.helpers.mail import Mail

# Load environment variables
GITHUB_TOKEN = os.environ["GITHUB_TOKEN"]
PR_URL = os.environ["GITHUB_API_URL"]
PROVISIONING_API_KEY = os.environ["PROVISIONING_API_KEY"]
SENDGRID_API_KEY = os.environ["EMAIL_API_KEY"]

# Step 1: Fetch PR body
print("🔍 Fetching PR body...")
pr_resp = requests.get(
    PR_URL,
    headers={"Authorization": f"Bearer {GITHUB_TOKEN}"}
)
pr_resp.raise_for_status()
pr_data = pr_resp.json()
pr_body = pr_data.get("body", "")
pr_number = pr_data["number"]
repo_full_name = pr_data["base"]["repo"]["full_name"]

# Step 2: Extract and decode base64 email from PR body
match = re.search(r"<!--EMAIL:([A-Za-z0-9+/=]+)-->", pr_body)
if not match:
    print("❌ No encoded email found in PR body. Skipping key issuance.")
    exit(0)

email_b64 = match.group(1)
email = base64.b64decode(email_b64).decode("utf-8")
print(f"📬 Decoded email: {email}")

# Step 3: Provision OpenRouter API key
print("🔐 Creating OpenRouter key...")
key_resp = requests.post(
    "https://openrouter.ai/api/v1/keys/",
    headers={
        "Authorization": f"Bearer {PROVISIONING_API_KEY}",
        "Content-Type": "application/json"
    },
    json={
        "name": "Goose Contributor",
        "label": "goose-cookbook",
        "limit": 10.0
    }
)
key_resp.raise_for_status()
api_key = key_resp.json()["key"]
print("✅ API key generated!")

# Step 4: Send email using SendGrid
print("📤 Sending email via SendGrid...")
sg = SendGridAPIClient(SENDGRID_API_KEY)

from_email = "Goose Team <onboarding@goosecredits.xyz>"  # ✅ Use your verified domain here
subject = "🎉 Your Goose Contributor API Key"
html_content = f"""
    <p>Thanks for contributing to the Goose Recipe Cookbook!</p>
    <p>Here’s your <strong>$10 OpenRouter API key</strong>:</p>
    <p><code>{api_key}</code></p>
    <p>Happy vibe-coding!<br>– The Goose Team 🪿</p>
"""

message = Mail(
    from_email=from_email,
    to_emails=email,
    subject=subject,
    html_content=html_content
)

try:
    response = sg.send(message)
    print("✅ Email sent! Status code:", response.status_code)
except Exception as e:
    print("❌ Failed to send email:", str(e))

# Step 5: Comment on PR confirming success
print("💬 Commenting on PR...")
comment_url = f"https://api.github.com/repos/{repo_full_name}/issues/{pr_number}/comments"

comment_resp = requests.post(
    comment_url,
    headers={
        "Authorization": f"Bearer {GITHUB_TOKEN}",
        "Accept": "application/vnd.github+json"
    },
    json={
        "body": f"✅ $10 OpenRouter API key sent to `{email}`. Thanks for your contribution to the Goose Cookbook!"
    }
)
comment_resp.raise_for_status()
print("✅ Confirmation comment added to PR.")
