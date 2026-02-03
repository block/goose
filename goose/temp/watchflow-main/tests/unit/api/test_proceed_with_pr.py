from fastapi.testclient import TestClient

from src.main import app


def test_proceed_with_pr_happy_path(monkeypatch):
    client = TestClient(app)

    async def _fake_get_repo(repo_full_name, installation_id=None, user_token=None):
        return {"default_branch": "main"}

    async def _fake_get_sha(repo_full_name, ref, installation_id=None, user_token=None):
        return "base-sha"

    async def _fake_create_ref(repo_full_name, ref, sha, installation_id=None, user_token=None):
        return {"ref": f"refs/heads/{ref}", "object": {"sha": sha}}

    async def _fake_create_or_update_file(
        repo_full_name, path, content, message, branch, installation_id=None, user_token=None, sha=None
    ):
        return {"commit": {"sha": "new-sha"}}

    async def _fake_create_pr(repo_full_name, title, head, base, body, installation_id=None, user_token=None):
        return {"html_url": "https://github.com/owner/repo/pull/1", "number": 1}

    from src.integrations.github import api as github_api

    monkeypatch.setattr(github_api.github_client, "get_repository", _fake_get_repo)
    monkeypatch.setattr(github_api.github_client, "get_git_ref_sha", _fake_get_sha)
    monkeypatch.setattr(github_api.github_client, "create_git_ref", _fake_create_ref)
    monkeypatch.setattr(github_api.github_client, "create_or_update_file", _fake_create_or_update_file)
    monkeypatch.setattr(github_api.github_client, "create_pull_request", _fake_create_pr)

    payload = {
        "repository_full_name": "owner/repo",
        "installation_id": 123,
        "rules_yaml": "description: sample\nenabled: true",
        "branch_name": "watchflow/rules",
        "pr_title": "Add Watchflow rules",
        "pr_body": "Body",
    }

    response = client.post("/api/v1/rules/recommend/proceed-with-pr", json=payload)
    assert response.status_code == 200
    data = response.json()
    assert data["pull_request_url"] == "https://github.com/owner/repo/pull/1"
    assert data["branch_name"] == "watchflow/rules"
    assert data["file_path"] == ".watchflow/rules.yaml"


def test_proceed_with_pr_requires_auth(monkeypatch):
    client = TestClient(app)
    payload = {"repository_full_name": "owner/repo", "rules_yaml": "description: sample\nenabled: true"}

    response = client.post("/api/v1/rules/recommend/proceed-with-pr", json=payload)
    assert response.status_code == 401
