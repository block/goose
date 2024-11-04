#!/usr/bin/env python3

import argparse
import os
import sys
from dataclasses import dataclass
from enum import Enum
from pathlib import Path

import requests
import tomli
import urllib3


class Color(str, Enum):
    """ANSI color codes with fallback for non-color terminals"""

    @staticmethod
    def supports_color() -> bool:
        """Check if the terminal supports color output."""
        if not hasattr(sys.stdout, "isatty"):
            return False
        if not sys.stdout.isatty():
            return False

        if "NO_COLOR" in os.environ:
            return False

        term = os.environ.get("TERM", "")
        if term == "dumb":
            return False

        return True

    has_color = supports_color()

    RED = "\033[91m" if has_color else ""
    GREEN = "\033[92m" if has_color else ""
    RESET = "\033[0m" if has_color else ""
    BOLD = "\033[1m" if has_color else ""


@dataclass(frozen=True)
class LicenseConfig:
    allowed_licenses: frozenset[str] = frozenset(
        {
            "MIT",
            "BSD-3-Clause",
            "Apache-2.0",
            "Apache License 2",
            "Apache Software License",
            "Python Software Foundation License",
            "BSD License",
            "ISC",
        }
    )
    exceptions: frozenset[str] = frozenset(
        {
            "ai-exchange",
            "tiktoken",
        }
    )


@dataclass(frozen=True)
class LicenseInfo:
    license: str | None
    allowed: bool = False

    def __str__(self) -> str:
        status = "✓" if self.allowed else "✗"
        color = Color.GREEN if self.allowed else Color.RED
        return f"{color}{status}{Color.RESET} {self.license}"


class LicenseChecker:
    def __init__(self, config: LicenseConfig = LicenseConfig()) -> None:
        self.config = config
        self.session = self._setup_session()

    def _setup_session(self) -> requests.Session:
        session = requests.Session()
        session.verify = True
        max_retries = urllib3.util.Retry(
            total=3,
            backoff_factor=0.5,
            status_forcelist=[
                500,
                502,
                503,
                504,
            ],
        )
        adapter = requests.adapters.HTTPAdapter(max_retries=max_retries)
        session.mount("https://", adapter)
        return session

    def normalize_license(self, license_str: str | None) -> str | None:
        """
        Normalize license string for comparison.

        This method takes a license string and normalizes it by:
        1. Converting to uppercase
        2. Removing 'LICENSE' or 'LICENCE' suffixes
        3. Stripping whitespace
        4. Replacing common variations with standardized forms

        Args:
            license_str (str | None): The original license string to normalize.

        Returns:
            str | None: The normalized license string, or None if the input was None.
        """
        if not license_str:
            return None

        # fmt: off
        normalized = (
            license_str.upper()
            .replace(" LICENSE", "")
            .replace(" LICENCE", "")
            .strip()
        )
        # fmt: on

        replacements = {
            "APACHE 2.0": "APACHE-2.0",
            "APACHE SOFTWARE LICENSE": "APACHE-2.0",
            "BSD": "BSD-3-CLAUSE",
            "MIT LICENSE": "MIT",
            "PYTHON SOFTWARE FOUNDATION": "PSF",
        }

        return replacements.get(normalized, normalized)

    def get_package_license(self, package_name: str) -> str | None:
        """Fetch license information from PyPI.

        Args:
            package_name (str): The name of the package to fetch the license for.

        Returns:
            str | None: The license of the package, or None if not found.
        """
        try:
            response = self.session.get(
                f"https://pypi.org/pypi/{package_name}/json",
                timeout=10,
            )
            response.raise_for_status()
            data = response.json()

            # fmt: off
            license_info = (
                data["info"].get("license") or
                data["info"].get("classifiers", [])
            )
            # fmt: on

            if isinstance(license_info, list):
                for classifier in license_info:
                    if classifier.startswith("License :: "):
                        parts = classifier.split(" :: ")
                        return parts[-1]

            return license_info if isinstance(license_info, str) else None

        except requests.exceptions.SSLError as e:
            print(f"SSL Error fetching license for {package_name}: {e}", file=sys.stderr)
        except Exception as e:
            print(f"Warning: Could not fetch license for {package_name}: {e}", file=sys.stderr)
        return None

    def extract_dependencies(self, toml_file: Path) -> list[str]:
        """Extract all dependencies from a TOML file."""
        with open(toml_file, "rb") as f:
            data = tomli.load(f)

        dependencies = []

        # Get direct dependencies
        project_deps = data.get("project", {}).get("dependencies", [])
        dependencies.extend(self._parse_dependency_strings(project_deps))

        # Get dev dependencies
        tool_deps = data.get("tool", {}).get("uv", {}).get("dev-dependencies", [])
        dependencies.extend(self._parse_dependency_strings(tool_deps))

        return list(set(dependencies))

    def _parse_dependency_strings(self, deps: list[str]) -> list[str]:
        """
        Parse dependency strings to extract package names.

        Args:
            deps (list[str]): A list of dependency strings to parse.

        Returns:
            list[str]: A list of extracted package names.
        """
        packages = []
        for dep in deps:
            if "workspace = true" in dep:
                continue

            # fmt: off
            # Handle basic package specifiers
            package = (
                dep.split(">=")[0]
                   .split("==")[0]
                   .split("<")[0]
                   .split(">")[0]
                   .strip()
            )
            package = package.split("{")[0].strip()
            # fmt: on
            if package:
                packages.append(package)
        return packages

    def check_licenses(self, toml_file: Path) -> dict[str, LicenseInfo]:
        """
        Check licenses for all dependencies in the TOML file.

        Args:
            toml_file (Path): The path to the TOML file containing the dependencies.

        Returns:
            dict[str, LicenseInfo]: A dictionary where the keys are package names and the values are LicenseInfo objects
                                    containing the license information and whether it's allowed."""
        dependencies = self.extract_dependencies(toml_file)
        results: dict[str, LicenseInfo] = {}
        checked: set[str] = set()

        for package in dependencies:
            if package in checked:
                continue

            checked.add(package)
            results[package] = self._check_package(package)

        return results

    def _check_package(self, package: str) -> LicenseInfo:
        """
        Check license for a single package.

        Args:
            package (str): The name of the package to check.

        Returns:
            LicenseInfo: A LicenseInfo object containing the license
                         information and whether it's allowed.
        """

        if package in self.config.exceptions:
            return LicenseInfo("Approved Exception", True)

        license_info = self.get_package_license(package)
        normalized_license = self.normalize_license(license_info)
        allowed = False

        # fmt: off
        if normalized_license:
            allowed = normalized_license in {
                self.normalize_license(x)
                for x in self.config.allowed_licenses
            }
        # fmt: on
        return LicenseInfo(license_info, allowed)


def main() -> None:
    parser = argparse.ArgumentParser(description="Check package licenses in TOML files")
    parser.add_argument("toml_files", type=Path, nargs="*", help="Paths to TOML files")
    parser.add_argument("--supported-licenses", action="store_true", help="Print supported licenses")

    checker = LicenseChecker()
    all_results: dict[str, LicenseInfo] = {}

    args = parser.parse_args()
    if args.supported_licenses:
        for license in sorted(checker.config.allowed_licenses, key=str.casefold):
            print(f" - {license}")
        sys.exit(0)

    if not args.toml_files:
        print("Error: No TOML files specified", file=sys.stderr)
        parser.print_help()
        sys.exit(1)

    for toml_file in args.toml_files:
        results = checker.check_licenses(toml_file)
        for package, info in results.items():
            if package in all_results and all_results[package] != info:
                print(f"Warning: Package {package} has conflicting license info:", file=sys.stderr)
                print(f"  {toml_file}: {info}", file=sys.stderr)
                print(f"  Previous: {all_results[package]}", file=sys.stderr)
            all_results[package] = info

    max_package_length = max(len(package) for package in all_results.keys())
    any_disallowed = False

    for package, info in sorted(all_results.items()):
        if Color.has_color:
            package_name = f"{Color.BOLD}{package}{Color.RESET}"
            padding = len(Color.BOLD) + len(Color.RESET)
        else:
            package_name = package
            padding = 0

        print(f"{package_name:<{max_package_length + padding}} {info}")
        if not info.allowed:
            any_disallowed = True

    sys.exit(1 if any_disallowed else 0)


if __name__ == "__main__":
    main()
