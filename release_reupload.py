# /// script
# requires-python = ">=3.13"
# dependencies = [
#   "requests"
# ]
# ///

import argparse
import os
import re
import requests
import sys


def get_release_info(repo, tag, token):
    """Get release information including assets"""
    headers = {
        "Authorization": f"token {token}",
        "Accept": "application/vnd.github.v3+json",
    }

    url = f"https://api.github.com/repos/{repo}/releases/tags/{tag}"
    response = requests.get(url, headers=headers)

    if response.status_code != 200:
        print(f"Error fetching release: {response.status_code} - {response.text}")
        sys.exit(1)

    return response.json()


def download_asset(asset_url, filename, token):
    """Download a release asset"""
    headers = {"Authorization": f"token {token}", "Accept": "application/octet-stream"}

    print(f"Downloading {filename}...")
    response = requests.get(asset_url, headers=headers, stream=True)

    if response.status_code != 200:
        print(f"Error downloading {filename}: {response.status_code}")
        return False

    with open(filename, "wb") as f:
        for chunk in response.iter_content(chunk_size=8192):
            f.write(chunk)

    return True


def upload_asset(release_id, repo, filename, new_filename, token):
    """Upload an asset to a release"""
    headers = {
        "Authorization": f"token {token}",
        "Content-Type": "application/octet-stream",
    }

    upload_url = f"https://uploads.github.com/repos/{repo}/releases/{release_id}/assets?name={new_filename}"

    print(f"Uploading {new_filename}...")
    with open(filename, "rb") as f:
        response = requests.post(upload_url, headers=headers, data=f)

    if response.status_code not in [200, 201]:
        print(
            f"Error uploading {new_filename}: {response.status_code} - {response.text}"
        )
        return False

    return True


def get_uppercase_name(filename):
    """Convert filename to lowercase version"""
    # Handle different patterns
    return filename.replace("goose", "Goose")


def main():
    parser = argparse.ArgumentParser(
        description="Update GitHub release assets with lowercase names"
    )
    parser.add_argument("repo", help="GitHub repository (owner/repo)")
    parser.add_argument("tag", help="Release tag")
    parser.add_argument("token", help="GitHub authentication token")

    args = parser.parse_args()

    # Get release information
    release_info = get_release_info(args.repo, args.tag, args.token)
    release_id = release_info["id"]
    assets = release_info["assets"]

    # Define patterns to match
    patterns = [
        re.compile(r"^goose-.*\.x86_64\.rpm$"),
        re.compile(r"^goose-win32-x64\.zip$"),
        re.compile(r"^goose\.zip$"),
        re.compile(r"^goose_intel_mac\.zip$"),
    ]

    # Find matching assets
    matching_assets = []
    for asset in assets:
        asset_name = asset["name"]
        for pattern in patterns:
            if pattern.match(asset_name):
                matching_assets.append(asset)
                break

    if not matching_assets:
        print("No matching assets found")
        return

    print(f"Found {len(matching_assets)} matching assets")

    # Process each matching asset
    for asset in matching_assets:
        original_name = asset["name"]
        new_name = get_uppercase_name(original_name)

        if original_name == new_name:
            print(f"Skipping {original_name} (already lowercase)")
            continue

        print(f"Processing: {original_name} -> {new_name}")

        # Download the asset
        if not download_asset(asset["url"], original_name, args.token):
            continue

        # Upload with new name
        if upload_asset(release_id, args.repo, original_name, new_name, args.token):
            print(f"Successfully uploaded {new_name}")

            # Clean up downloaded file
            os.remove(original_name)
            print(f"Cleaned up {original_name}")
        else:
            print(f"Failed to upload {new_name}")


if __name__ == "__main__":
    main()
