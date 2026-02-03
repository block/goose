#!/usr/bin/env python3
"""
OpenLIT Instrumentation Package Setup Script

This script is executed in the init container to prepare instrumentation packages
for zero-code injection into application containers. It copies provider-specific
instrumentation packages to a shared volume that gets mounted into the target
application pods.

Key functions:
- Copies instrumentation packages from provider-specific directories
- Handles sitecustomize.py placement for automatic import
- Supports multiple instrumentation providers (OpenLIT, OpenInference, OpenLLMetry)
- Provides detailed logging for debugging setup issues

The script is provider-agnostic and uses the INSTRUMENTATION_PROVIDER environment
variable to determine which packages to copy from /instrumentations/{provider}/
to the shared /openlit-sdk/ directory.

Used by: OpenLIT Kubernetes operator init containers
Environment: Python 3.x in Alpine-based container
"""

import os
import shutil
import sys


def setup_instrumentation():
    """Copy instrumentation packages to target directory"""
    provider = os.environ.get("INSTRUMENTATION_PROVIDER", "openlit")
    source_dir = f"/instrumentations/{provider}"
    target_dir = os.environ.get("TARGET_PATH", "/instrumentation-packages")

    print(f"🚀 Setting up {provider} instrumentation...")

    # Ensure target directory exists
    os.makedirs(target_dir, exist_ok=True)

    # Copy packages
    if os.path.exists(source_dir):
        # Copy all contents from source to target
        for item in os.listdir(source_dir):
            source_path = os.path.join(source_dir, item)
            target_path = os.path.join(target_dir, item)

            if os.path.isdir(source_path):
                if os.path.exists(target_path):
                    shutil.rmtree(target_path)
                shutil.copytree(source_path, target_path)
            else:
                shutil.copy2(source_path, target_path)

        # Copy sitecustomize.py to the root for auto-import
        sitecustomize_src = os.path.join(source_dir, "sitecustomize.py")
        sitecustomize_dst = os.path.join(target_dir, "sitecustomize.py")
        if os.path.exists(sitecustomize_src):
            shutil.copy2(sitecustomize_src, sitecustomize_dst)
            print("✅ sitecustomize.py copied to root for auto-import")

        print(f"✅ {provider} instrumentation ready!")
        print(f"🎯 Provider: {provider}")
        print(f"📦 Packages copied to: {target_dir}")

        # Show some package info
        try:
            packages = [f for f in os.listdir(target_dir) if not f.startswith(".")][:10]
            print(f"📚 Key packages: {', '.join(packages[:5])}")
            if len(packages) > 5:
                print(f"    ... and {len(packages) - 5} more")
        except Exception as e:
            print(f"📦 Package listing failed: {e}")

    else:
        print(f"❌ Source directory not found: {source_dir}")
        sys.exit(1)


if __name__ == "__main__":
    setup_instrumentation()
