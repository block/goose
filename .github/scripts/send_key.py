import os
import requests
import re
import email_validator
from sendgrid import SendGridAPIClient
from sendgrid.helpers.mail import Mail
from python_http_client.exceptions import HTTPError

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

def extract_email_from_text(text):
    """Extract email from text using various patterns"""
    # Try PR template format: "**Email**: email@example.com"
    email_match = re.search(r"\*\*Email\*\*:\s*([A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,})", text)
    if email_match:
        return email_match.group(1)
    
    # Try other common email patterns
    email_match = re.search(r"[Ee]mail:\s*([A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,})", text)
    if email_match:
        return email_match.group(1)
    
    # Try general email pattern
    email_match = re.search(r"\b([A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,})\b", text)
    if email_match:
        return email_match.group(1)
    
    return None

def fetch_pr_comments(pr_url, github_token):
    """Fetch all comments on the PR"""
    # Convert PR URL to comments URL
    comments_url = pr_url.replace("/pulls/", "/issues/") + "/comments"
    
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
    """Validate email address format and deliverability"""
    try:
        # Validate and get normalized email
        valid_email = email_validator.validate_email(email)
        normalized_email = valid_email.email
        print(f"✅ Email validation passed: {normalized_email}")
        return normalized_email
    except email_validator.EmailNotValidError as e:
        print(f"❌ Email validation failed: {e}")
        return None

def extract_email(pr_body, pr_url, github_token):
    """Extract and validate email from PR body and comments"""
    print("🔍 Searching for email in PR body...")
    
    # First check PR body
    email = extract_email_from_text(pr_body)
    if email:
        print(f"📧 Found email in PR body: {email}")
        validated_email = validate_email_address(email)
        if validated_email:
            return validated_email
        else:
            print("⚠️ Email in PR body is invalid, checking comments...")
    
    print("🔍 No valid email found in PR body, checking comments...")
    
    # Check PR comments
    comments = fetch_pr_comments(pr_url, github_token)
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
    
    # No valid email found anywhere
    print("❌ No valid email found in PR body or comments. Skipping key issuance.")
    exit(0)

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
    
    try:
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
        
        response = sg.send(message)
        print(f"✅ Email sent successfully! Status code: {response.status_code}")
        
        # Check for potential issues even on "success"
        if response.status_code >= 300:
            print(f"⚠️ Warning: Unexpected status code {response.status_code}")
            print(f"Response body: {response.body}")
            return False
            
        return True
        
    except HTTPError as e:
        # Specific SendGrid HTTP errors
        status_code = e.status_code
        error_body = e.body
        
        if status_code == 401:
            print("❌ SendGrid authentication failed - invalid API key")
        elif status_code == 403:
            print("❌ SendGrid authorization failed - API key lacks permissions")
        elif status_code == 429:
            print("❌ SendGrid rate limit exceeded - too many requests")
        elif status_code == 400:
            print(f"❌ SendGrid bad request - invalid email data: {error_body}")
        elif status_code >= 500:
            print(f"❌ SendGrid server error ({status_code}) - try again later")
        else:
            print(f"❌ SendGrid HTTP error {status_code}: {error_body}")
            
        print(f"Full error details: {e}")
        return False
        
    except ValueError as e:
        print(f"❌ Invalid email format or API key: {e}")
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
    
    # Handle cases where pr_data might be missing expected fields
    pr_number = pr_data.get("number")
    if not pr_number:
        print("❌ Unable to get PR number from GitHub API response")
        print(f"Available keys in response: {list(pr_data.keys())}")
        # Try to extract number from URL if available
        if "html_url" in pr_data:
            import re
            match = re.search(r'/pull/(\d+)', pr_data["html_url"])
            if match:
                pr_number = int(match.group(1))
                print(f"✅ Extracted PR number from URL: {pr_number}")
            else:
                print("❌ Could not extract PR number from URL either")
                exit(1)
        else:
            print("❌ No html_url available to extract PR number")
            exit(1)
    
    # Get repo info
    if "base" in pr_data and "repo" in pr_data["base"]:
        repo_full_name = pr_data["base"]["repo"]["full_name"]
    elif "repository" in pr_data:
        repo_full_name = pr_data["repository"]["full_name"]
    else:
        print("❌ Unable to get repository name from GitHub API response")
        print(f"Available keys in response: {list(pr_data.keys())}")
        exit(1)

    email = extract_email(pr_body, PR_URL, GITHUB_TOKEN)
    print(f"📬 Found email: {email}")

    try:
        api_key = provision_api_key(PROVISIONING_API_KEY)
        print("✅ API key generated!")
        
        if send_email(email, api_key, SENDGRID_API_KEY):
            comment_on_pr(GITHUB_TOKEN, repo_full_name, pr_number, email)
    except Exception as err:
        print(f"❌ An error occurred: {err}")

if __name__ == "__main__":
    main()
