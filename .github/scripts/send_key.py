import os
import requests
import re
import email_validator
from sendgrid import SendGridAPIClient
from sendgrid.helpers.mail import Mail
from python_http_client.exceptions import HTTPError


def main():
    # Environment variable validation
    required_envs = ["GITHUB_TOKEN", "GITHUB_SHA", "GITHUB_REPOSITORY", "PROVISIONING_API_KEY", "EMAIL_API_KEY"]
    missing = [env for env in required_envs if env not in os.environ]
    if missing:
        print(f"❌ Missing environment variables: {', '.join(missing)}")
        exit(2)

    GITHUB_TOKEN = os.environ["GITHUB_TOKEN"]
    GITHUB_SHA = os.environ["GITHUB_SHA"]
    REPO_NAME = os.environ["GITHUB_REPOSITORY"]
    PROVISIONING_API_KEY = os.environ["PROVISIONING_API_KEY"]
    SENDGRID_API_KEY = os.environ["EMAIL_API_KEY"]

    pr_number = get_pr_number_from_sha()
    pr_data = fetch_pr_body(pr_number, GITHUB_TOKEN, REPO_NAME)
    pr_body = pr_data.get("body", "")

    email = extract_email(pr_body, REPO_NAME, pr_number, GITHUB_TOKEN)
    print(f"📬 Found email: {email}")

    try:
        api_key = provision_api_key(PROVISIONING_API_KEY)
        print("✅ API key generated!")

        if not send_email(email, api_key, SENDGRID_API_KEY):
            print("❌ Email failed to send. Exiting without PR comment.")
            exit(2)

        comment_on_pr(GITHUB_TOKEN, REPO_NAME, pr_number, email)

    except Exception as err:
        print(f"❌ An error occurred: {err}")
        exit(2)

def get_pr_number_from_sha():
    token = os.getenv("GITHUB_TOKEN")
    repo = os.getenv("GITHUB_REPOSITORY")
    sha = os.getenv("GITHUB_SHA")

    url = f"https://api.github.com/repos/{repo}/commits/{sha}/pulls"
    headers = {
        "Authorization": f"token {token}",
        "Accept": "application/vnd.github.groot-preview+json"
    }

    response = requests.get(url, headers=headers)
    response.raise_for_status()

    pr_data = response.json()
    if pr_data:
        return pr_data[0]["number"]
    else:
        raise Exception("No PR found for this SHA")

def fetch_pr_body(pr_number, github_token, repo_full_name):
    print("🔍 Fetching PR body...")
    pr_url = f"https://api.github.com/repos/{repo_full_name}/pulls/{pr_number}"
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

def extract_email_from_text(text):
    email_match = re.search(r"\*\*Email\*\*:\s*([A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,})", text)
    if email_match:
        return email_match.group(1)
    email_match = re.search(r"[Ee]mail:\s*([A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,})", text)
    if email_match:
        return email_match.group(1)
    email_match = re.search(r"\b([A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,})\b", text)
    if email_match:
        return email_match.group(1)
    return None

def fetch_pr_comments(repo_full_name, pr_number, github_token):
    comments_url = f"https://api.github.com/repos/{repo_full_name}/issues/{pr_number}/comments"
    try:
        comments_resp = requests.get(
            comments_url,
            headers={"Authorization": f"Bearer {github_token}"}
        )
        comments_resp.raise_for_status()
        return comments_resp.json()
    except requests.exceptions.RequestException as e:
        print(f"⚠️ Failed to fetch PR comments: {e}")
        return []

def validate_email_address(email):
    try:
        valid_email = email_validator.validate_email(email)
        normalized_email = valid_email.email
        print(f"✅ Email validation passed: {normalized_email}")
        return normalized_email
    except email_validator.EmailNotValidError as e:
        print(f"❌ Email validation failed: {e}")
        return None

def extract_email(pr_body, repo_full_name, pr_number, github_token):
    print("🔍 Searching for email in PR body...")
    email = extract_email_from_text(pr_body)
    if email:
        print(f"📧 Found email in PR body: {email}")
        validated_email = validate_email_address(email)
        if validated_email:
            return validated_email
        else:
            print("⚠️ Email in PR body is invalid, checking comments...")

    print("🔍 No valid email found in PR body, checking comments...")
    comments = fetch_pr_comments(repo_full_name, pr_number, github_token)
    for comment in comments:
        comment_body = comment.get("body", "")
        email = extract_email_from_text(comment_body)
        if email:
            print(f"📧 Found email in comment by {comment.get('user', {}).get('login', 'unknown')}: {email}")
            validated_email = validate_email_address(email)
            if validated_email:
                return validated_email
            else:
                print("⚠️ Email in comment is invalid, continuing search...")

    print("❌ No valid email found in PR body or comments. Skipping key issuance.")
    exit(2)

def provision_api_key(provisioning_api_key):
    print("🔐 Creating OpenRouter key...")
    try:
        key_resp = requests.post(
            "https://openrouter.ai/api/v1/keys",
            headers={
                "Authorization": f"Bearer {provisioning_api_key}",
                "Content-Type": "application/json"
            },
            json={
                "name": "goose contributor",
                "label": "goose-cookbook",
                "limit": 10.0
            }
        )
        key_resp.raise_for_status()
    except requests.exceptions.RequestException as e:
        print("❌ Failed to provision API key:", str(e))
        raise
    key = key_resp.json().get("key")
    if not key:
        print("❌ API response did not include a key.")
        exit(2)
    return key

def send_email(email, api_key, sendgrid_api_key):
    print("📤 Sending email via SendGrid...")
    try:
        sg = SendGridAPIClient(sendgrid_api_key)
        from_email = "goose team <goose@opensource.block.xyz>"
        subject = "🎉 Your goose contributor API key"
        html_content = f"""
            <p>Thank you for contributing to the <strong>goose recipe cookbook</strong>!</p>
            <p>🎉 Here's your <strong>$10 OpenRouter API key</strong>:</p>
            <pre style="background-color:#f4f4f4;padding:10px;border-radius:6px;"><code>{api_key}</code></pre>
            <p>To use this in goose (CLI or Desktop):</p>
            <ul>
              <li>Go to your <strong>Provider Settings</strong></li>
              <li>Select <strong>OpenRouter</strong> from the provider list</li>
              <li>Paste your API key</li>
            </ul>
            <p>📚 Full setup instructions:<br>
            <a href="https://block.github.io/goose/docs/getting-started/providers/#configure-provider">
            https://block.github.io/goose/docs/getting-started/providers/#configure-provider</a></p>
            <p>Happy coding!<br>– the goose team</p>
        """
        message = Mail(
            from_email=from_email,
            to_emails=email,
            subject=subject,
            html_content=html_content
        )
        response = sg.send(message)
        print(f"✅ Email sent successfully! Status code: {response.status_code}")
        if response.status_code >= 300:
            print(f"⚠️ Warning: Unexpected status code {response.status_code}")
            print(f"Response body: {response.body}")
            return False
        return True

    except HTTPError as e:
        print(f"❌ SendGrid HTTP error {e.status_code}: {e.body}")
        return False
    except Exception as e:
        print(f"❌ Unexpected error sending email: {type(e).__name__}: {e}")
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
                "body": f"✅ $10 OpenRouter API key sent to `{email}`. Thanks for your contribution to the goose cookbook!"
            }
        )
        comment_resp.raise_for_status()
        print("✅ Confirmation comment added to PR.")
    except requests.exceptions.RequestException as e:
        print("❌ Failed to comment on PR:", str(e))
        raise

if __name__ == "__main__":
    main()
