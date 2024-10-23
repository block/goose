#!/usr/bin/env python3

from pathlib import Path
import tomli
import sys
import requests
import urllib3
from typing import Dict, List, Optional, Set

# Define allowed licenses and exceptions directly in the script
ALLOWED_LICENSES = {
    "MIT",
    "BSD-3-Clause",
    "Apache-2.0",
    "Apache Software License",
    "Python Software Foundation License",
    "BSD License",
    "ISC"
}

# Package-specific exceptions
EXCEPTIONS = {
    "ai-exchange": True,  # Local workspace package
    "tiktoken": True,     # Known MIT license with non-standard format
}

class LicenseChecker:
    def __init__(self):
        self.session = requests.Session()
        # Configure session for robust SSL handling
        self.session.verify = True
        adapter = requests.adapters.HTTPAdapter(
            max_retries=urllib3.util.Retry(
                total=3,
                backoff_factor=0.5,
                status_forcelist=[500, 502, 503, 504]
            )
        )
        self.session.mount('https://', adapter)
        
    def normalize_license(self, license_str: Optional[str]) -> Optional[str]:
        """Normalize license string for comparison."""
        if not license_str:
            return None
        
        # Convert to uppercase and remove common words and punctuation
        normalized = license_str.upper().replace(' LICENSE', '').replace(' LICENCE', '').strip()
        
        # Common substitutions
        replacements = {
            'APACHE 2.0': 'APACHE-2.0',
            'APACHE SOFTWARE LICENSE': 'APACHE-2.0',
            'BSD': 'BSD-3-CLAUSE',
            'MIT LICENSE': 'MIT',
            'PYTHON SOFTWARE FOUNDATION': 'PSF',
        }
        
        return replacements.get(normalized, normalized)
    
    def get_package_license(self, package_name: str) -> Optional[str]:
        """Fetch license information from PyPI."""
        if package_name in EXCEPTIONS:
            return "APPROVED-EXCEPTION"

        try:
            response = self.session.get(f"https://pypi.org/pypi/{package_name}/json")
            response.raise_for_status()
            data = response.json()
            
            license_info = (
                data['info'].get('license') or
                data['info'].get('classifiers', [])
            )
            
            if isinstance(license_info, list):
                for classifier in license_info:
                    if classifier.startswith('License :: '):
                        parts = classifier.split(' :: ')
                        return parts[-1]
            
            return license_info if isinstance(license_info, str) else None
        
        except requests.exceptions.SSLError as e:
            print(f"SSL Error fetching license for {package_name}: {e}", file=sys.stderr)
            return None
        except Exception as e:
            print(f"Warning: Could not fetch license for {package_name}: {e}", file=sys.stderr)
            return None

    def extract_dependencies(self, toml_file: Path) -> List[str]:
        """Extract all dependencies from a TOML file."""
        with open(toml_file, 'rb') as f:
            data = tomli.load(f)
        
        dependencies = []
        
        # Get direct dependencies
        project_deps = data.get('project', {}).get('dependencies', [])
        dependencies.extend(self._parse_dependency_strings(project_deps))
        
        # Get dev dependencies
        tool_deps = data.get('tool', {}).get('uv', {}).get('dev-dependencies', [])
        dependencies.extend(self._parse_dependency_strings(tool_deps))
        
        return list(set(dependencies))
    
    def _parse_dependency_strings(self, deps: List[str]) -> List[str]:
        """Parse dependency strings to extract package names."""
        packages = []
        for dep in deps:
            # Skip workspace references
            if dep.endswith('workspace = true}'):
                continue
            
            # Handle basic package specifiers
            package = dep.split('>=')[0].split('==')[0].split('<')[0].split('>')[0].strip()
            package = package.split('{')[0].strip()
            packages.append(package)
        return packages
    
    def check_licenses(self, toml_file: Path) -> Dict[str, Dict[str, bool]]:
        """Check licenses for all dependencies in the TOML file."""
        dependencies = self.extract_dependencies(toml_file)
        results = {}
        checked = set()
        
        for package in dependencies:
            if package in checked:
                continue
                
            checked.add(package)
            
            if package in EXCEPTIONS:
                results[package] = {
                    'license': 'Approved Exception',
                    'allowed': True
                }
                continue
                
            license_info = self.get_package_license(package)
            normalized_license = self.normalize_license(license_info)
            allowed = False
            
            if normalized_license:
                allowed = (normalized_license in {self.normalize_license(l) for l in ALLOWED_LICENSES} or
                          package in EXCEPTIONS)
            
            results[package] = {
                'license': license_info,
                'allowed': allowed
            }
        
        return results

def main():
    if len(sys.argv) < 2:
        print("Usage: check_licenses.py <toml_file>", file=sys.stderr)
        sys.exit(1)
    
    toml_file = Path(sys.argv[1])
    checker = LicenseChecker()
    results = checker.check_licenses(toml_file)
    
    any_disallowed = False
    for package, info in sorted(results.items()):
        status = "✓" if info['allowed'] else "✗"
        print(f"{status} {package}: {info['license']}")
        if not info['allowed']:
            any_disallowed = True
    
    sys.exit(1 if any_disallowed else 0)

if __name__ == '__main__':
    main()