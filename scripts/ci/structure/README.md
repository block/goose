# Repository Structure Checks

This directory contains scripts that perform static analysis and structural validation of the repository **before** any builds are executed. These checks enforce repository policies and prevent issues from being committed.

## How It Works

All executable shell scripts (`*.sh`) in this directory are automatically run by:
- **CI**: `.github/workflows/ci.yml` - `structure-check` job
- **Local**: `just check-everything` command

Scripts are executed in alphabetical order. All scripts must pass for the check to succeed.

## Adding a New Check

1. Create a new executable shell script in this directory:
   ```bash
   touch scripts/ci/structure/check-your-thing.sh
   chmod +x scripts/ci/structure/check-your-thing.sh
   ```

2. Write your check script following this template:
   ```bash
   #!/usr/bin/env bash
   set -e

   # Brief description of what this checks

   echo "Checking your thing..."

   # Your validation logic here
   if ! validation_passes; then
     echo ""
     echo "ERROR: Your thing validation failed"
     echo ""
     echo "To fix this issue:"
     echo "  some-command-to-fix-it"
     echo ""
     exit 1
   fi

   echo "Your thing is valid"
   ```

3. Test locally:
   ```bash
   ./scripts/ci/structure/check-your-thing.sh
   just check-everything
   ```

4. Commit and push - CI will automatically run your new check

## Existing Checks

- **check-prebuilt-binaries.sh** - Prevents prebuilt executables from being committed (security)
- **check-rust-format.sh** - Validates Rust code formatting

## Guidelines

- **Keep checks fast** - These run on every PR
- **Make errors actionable** - Tell users how to fix issues
- **Exit codes** - Exit 0 for success, non-zero for failure
- **Echo progress** - Let users know what's being checked
- **Use allowlists sparingly** - Only for temporary exceptions
