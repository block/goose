from pathlib import Path
from typing import Optional
from unittest.mock import Mock, patch

import pytest
import tomli
from check_licenses import Color, LicenseChecker, LicenseConfig, LicenseInfo, main


@pytest.fixture
def checker() -> LicenseChecker:
    return LicenseChecker()


@pytest.fixture
def mock_pypi_response() -> Mock:
    response = Mock()
    response.status_code = 200
    response.raise_for_status = Mock()
    response.ok = True
    response.json.return_value = {
        "info": {
            "license": "Apache-2.0",
        }
    }
    return response


@pytest.fixture
def mock_toml_content() -> str:
    return """
[project]
dependencies = [
    "requests>=2.28.0",
    "tomli==2.0.1",
    "urllib3<2.0.0",
    "package-with-workspace{workspace = true}",
]

[tool.uv]
dev-dependencies = [
    "pytest>=7.0.0",
    "black==23.3.0",
]
"""


@pytest.fixture
def mock_toml_files(tmp_path: Path) -> list[Path]:
    """Create mock TOML files with different dependencies."""
    file1 = tmp_path / "pyproject1.toml"

    file1.write_text("""
[project]
dependencies = [
    "requests>=2.28.0",
    "tomli==2.0.1",
]
""")

    file2 = tmp_path / "pyproject2.toml"
    file2.write_text("""
[project]
dependencies = [
    "urllib3<2.0.0",
    "requests>=2.27.0",  # Different version but same package
]
""")

    return [file1, file2]


def test_normalize_license_variations(checker: LicenseChecker) -> None:
    assert checker.normalize_license("MIT License") == "MIT"
    assert checker.normalize_license("Apache 2.0") == "APACHE-2.0"
    assert checker.normalize_license("BSD") == "BSD-3-CLAUSE"
    assert checker.normalize_license(None) is None
    assert checker.normalize_license("") is None
    assert checker.normalize_license("MIT License") == "MIT"
    assert checker.normalize_license("Apache 2.0") == "APACHE-2.0"
    assert checker.normalize_license("BSD") == "BSD-3-CLAUSE"
    assert checker.normalize_license(None) is None
    assert checker.normalize_license("") is None


@patch.object(LicenseChecker, "get_package_license")
def test_package_license_verification(mock_get_license: Mock, checker: LicenseChecker) -> None:
    def check_package_license(
        package: str, license: Optional[str], expected_allowed: bool, expected_license: str
    ) -> None:
        if license:
            mock_get_license.return_value = license
        result = checker._check_package(package=package)
        assert result.allowed is expected_allowed
        assert result.license == expected_license

    check_package_license(
        package="tiktoken",
        license=None,
        expected_allowed=True,
        expected_license="Approved Exception",
    )
    check_package_license(
        package="requests",
        license="Apache-2.0",
        expected_allowed=True,
        expected_license="Apache-2.0",
    )
    check_package_license(
        package="gpl-package",
        license="GPL",
        expected_allowed=False,
        expected_license="GPL",
    )


def test_color_support_with_environment_variables(monkeypatch: pytest.MonkeyPatch) -> None:
    def verify_color_support_disabled(*, env_var: str, value: str) -> None:
        monkeypatch.setenv(env_var, value)
        assert not Color.supports_color()
        monkeypatch.undo()

    verify_color_support_disabled(env_var="NO_COLOR", value="1")
    verify_color_support_disabled(env_var="TERM", value="dumb")


@patch("tomli.load")
@patch("builtins.open")
def test_extract_dependencies(
    mock_open: Mock, mock_tomli_load: Mock, checker: LicenseChecker, mock_toml_content: str
) -> None:
    mock_tomli_load.return_value = tomli.loads(mock_toml_content)
    mock_file = Mock()
    mock_open.return_value.__enter__.return_value = mock_file

    dependencies = checker.extract_dependencies(Path("mock_pyproject.toml"))
    expected = ["requests", "tomli", "urllib3", "pytest", "black"]
    assert sorted(dependencies) == sorted(expected)


@patch("requests.Session")
def test_get_package_license(mock_session: Mock, checker: LicenseChecker, mock_pypi_response: Mock) -> None:
    mock_session.return_value.get.return_value = mock_pypi_response
    assert checker.get_package_license("requests") == "Apache-2.0"

    # test exception handling
    mock_session.return_value.get.side_effect = Exception("error")
    assert checker.get_package_license("nonexistent-package") is None


def test_license_info_string_representation() -> None:
    def assert_license_info_str(info: LicenseInfo, no_color_expected: str, color_expected: str) -> None:
        expected = color_expected if Color.has_color else no_color_expected
        assert str(info) == expected

    assert_license_info_str(LicenseInfo("MIT", True), "✓ MIT", f"{Color.GREEN}✓{Color.RESET} MIT")
    assert_license_info_str(LicenseInfo("GPL", False), "✗ GPL", f"{Color.RED}✗{Color.RESET} GPL")


def test_custom_license_config() -> None:
    custom_config = LicenseConfig(
        allowed_licenses=frozenset({"MIT", "Apache-2.0"}), exceptions=frozenset({"special-package"})
    )
    checker = LicenseChecker(config=custom_config)

    assert "special-package" in checker.config.exceptions
    assert len(checker.config.allowed_licenses) == 2


def test_dependency_parsing_scenarios() -> None:
    """Test various dependency parsing scenarios."""

    def parse_dependencies(toml_string: str) -> list[str]:
        checker = LicenseChecker()
        return checker._parse_dependency_strings(tomli.loads(toml_string)["project"]["dependencies"])

    # basic version specifiers
    parsed = parse_dependencies("""
    [project]
    dependencies = [
        "requests>=2.28.0",
        "tomli==2.0.1",
        "urllib3<2.0.0",
    ]
    """)
    assert sorted(parsed) == sorted(["requests", "tomli", "urllib3"])

    # workspace dependencies
    parsed = parse_dependencies("""
    [project]
    dependencies = [
        "package-with-workspace{workspace = true}",
    ]
    """)
    assert parsed == []

    # mixed dependencies
    parsed = parse_dependencies("""
    [project]
    dependencies = [
        "requests>=2.28.0",
        "package-with-workspace{workspace = true}",
        "urllib3<2.0.0",
    ]
    """)
    assert sorted(parsed) == sorted(["requests", "urllib3"])

    # multiple version constraints
    parsed = parse_dependencies("""
    [project]
    dependencies = [
        "urllib3>=1.25.4,<2.0.0",
        "requests>=2.28.0,<3.0.0",
    ]
    """)
    assert sorted(parsed) == sorted(["urllib3", "requests"])

    # empty dependencies
    parsed = parse_dependencies("""
    [project]
    dependencies = []
    """)
    assert parsed == []


@patch.object(LicenseChecker, "get_package_license")
def test_multiple_toml_files(
    mock_get_license: Mock,
    mock_toml_files: list[Path],
    capsys: pytest.CaptureFixture,
) -> None:
    def get_license(package: str) -> Optional[str]:
        licenses = {"requests": "Apache-2.0", "tomli": "MIT", "urllib3": "MIT"}
        return licenses.get(package)

    mock_get_license.side_effect = get_license

    with patch("sys.argv", ["check_licenses.py"] + [str(f) for f in mock_toml_files]):
        try:
            main()
        except SystemExit as e:
            assert e.code == 0

    captured = capsys.readouterr()
    assert "requests" in captured.out
    assert "tomli" in captured.out
    assert "urllib3" in captured.out
    assert "✗" not in captured.out
