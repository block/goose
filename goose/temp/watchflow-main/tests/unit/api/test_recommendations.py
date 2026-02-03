import pytest

from src.api.recommendations import parse_repo_from_url


def test_valid_https_url():
    """
    Tests that a standard HTTPS URL is parsed correctly.
    """
    url = "https://github.com/owner/repo"
    assert parse_repo_from_url(url) == "owner/repo"


def test_url_with_git_suffix():
    """
    Tests that a URL with a .git suffix is parsed correctly.
    """
    url = "https://github.com/owner/repo.git"
    assert parse_repo_from_url(url) == "owner/repo"


def test_ssh_url():
    """
    Tests that an SSH URL is parsed correctly.
    """
    url = "git@github.com:owner/repo.git"
    assert parse_repo_from_url(url) == "owner/repo"


def test_invalid_url():
    """
    Tests that an invalid URL raises a ValueError.
    """
    url = "https://gitlab.com/owner/repo"
    with pytest.raises(ValueError):
        parse_repo_from_url(url)


def test_incomplete_url():
    """
    Tests that an incomplete GitHub URL raises a ValueError.
    """
    url = "https://github.com/owner"
    with pytest.raises(ValueError):
        parse_repo_from_url(url)


def test_non_url_string():
    """
    Tests that a non-URL string raises a ValueError.
    """
    url = "just a string"
    with pytest.raises(ValueError):
        parse_repo_from_url(url)
