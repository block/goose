import os
import requests
import base64
import re
from sendgrid import SendGridAPIClient
from sendgrid.helpers.mail import Mail

def fetch_pr_body(pr_url, github_token):
    print("🔍 Fetching PR body...")
    try:
        pr_resp = requests.get(
            pr_url,
            headers={"Authorization": f"Bearer {github_token}"}
        )
        pr_resp.raise_for_status()
    except requests.exceptions.RequestException as e:
        print("❌ Failed to fetch PR body:", str(e))
        raise
    return pr_resp.json()

def extract_email(pr_body):
    match = re.search(r"<!--EMAIL:([A-Za-z0-9+/=]+)-->", pr_body)
    if not match:
        print("❌ No encoded email found in PR body. Skipping key issuance.")
        exit(0)
    email_b64 = match.group(1)
    return base64.b64decode(email_b64).decode("utf-8")

def provision_api_key(provisioning_api_key):
    print("🔐 Creating OpenRouter key...")
    try:
        key_resp = requests.post(
            "https://openrouter.ai/api/v1/keys/",
            headers={
                "Authorization": f"Bearer {provisioning_api_key}",
                "Content-Type": "application/json"
            },
            json={
                "name": "Goose Contributor",
                "label": "goose-cookbook",
                "limit": 10.0
            }
        )
        key_resp.raise_for_status()
    except requests.exceptions.RequestException as e:
        print("❌ Failed to provision API key:", str(e))
        raise
    return key_resp.json()["key"]

def send_email(email, api_key, sendgrid_api_key):
    print("📤 Sending email via SendGrid...")
    sg = SendGridAPIClient(sendgrid_api_key)
    from_email = "Goose Team <goose@opensource.block.xyz>"  
    subject = "🎉 Your Goose Contributor API Key"
    html_content = f"""
        <p>Thanks for contributing to the Goose Recipe Cookbook!</p>
        <p>Here's your <strong>$10 OpenRouter API key</strong>:</p>
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
        return True
    except (requests.exceptions.RequestException, ValueError, KeyError) as e:
        print("❌ Failed to send email:", str(e))
        return False

def comment_on_pr(github_token, repo_full_name, pr_number, email):
    print("💬 Commenting on PR...")
    comment_url = f"https://api.github.com/repos/{repo_full_name}/issues/{pr_number}/comments"
    try:
        comment_resp = requests.post(
            comment_url,
            headers={
                "Authorization": f"Bearer {github_token}",
                "Accept": "application/vnd.github+json"
            },
            json={
                "body": f"✅ $10 OpenRouter API key sent to `{email}`. Thanks for your contribution to the Goose Cookbook!"
            }
        )
        comment_resp.raise_for_status()
        print("✅ Confirmation comment added to PR.")
    except requests.exceptions.RequestException as e:
        print("❌ Failed to comment on PR:", str(e))
        raise

def main():
    # Load environment variables
    GITHUB_TOKEN = os.environ["GITHUB_TOKEN"]
    PR_URL = os.environ["GITHUB_API_URL"]
    PROVISIONING_API_KEY = os.environ["PROVISIONING_API_KEY"]
    SENDGRID_API_KEY = os.environ["EMAIL_API_KEY"]

    pr_data = fetch_pr_body(PR_URL, GITHUB_TOKEN)
    pr_body = pr_data.get("body", "")
    pr_number = pr_data["number"]
    repo_full_name = pr_data["base"]["repo"]["full_name"]

    email = extract_email(pr_body)
    print(f"📬 Decoded email: {email}")

    try:
        api_key = provision_api_key(PROVISIONING_API_KEY)
        print("✅ API key generated!")
        
        if send_email(email, api_key, SENDGRID_API_KEY):
            comment_on_pr(GITHUB_TOKEN, repo_full_name, pr_number, email)
    except Exception as err:
        print(f"❌ An error occurred: {err}")

if __name__ == "__main__":
    main()
