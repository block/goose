#!/usr/bin/env bash
set -e

# Check for prebuilt executables/binaries in the repository
# This is a security measure to prevent supply chain attacks via checked-in binaries
#
# Similar checks in other projects:
# - Rust compiler: https://github.com/rust-lang/rust/blob/master/src/tools/tidy/src/bins.rs
# - OSSF Scorecard: https://github.com/ossf/scorecard/blob/main/checks/binary_artifact.go

echo "Checking for prebuilt executables and binaries..."

# Allowlist for existing binaries (TEMPORARY - to be removed)
# These binaries should be removed and built from source or downloaded at build time
# Added in: https://github.com/block/goose/pull/880 (commit cfd3ee8fd9c)
declare -A allowlist=(
  ["ui/desktop/src/platform/windows/bin/libgcc_s_seh-1.dll"]=1
  ["ui/desktop/src/platform/windows/bin/libstdc++-6.dll"]=1
  ["ui/desktop/src/platform/windows/bin/libwinpthread-1.dll"]=1
  ["ui/desktop/src/platform/windows/bin/uv.exe"]=1
  ["ui/desktop/src/platform/windows/bin/uvx.exe"]=1
)

# Denylist of file type patterns that indicate binary executables or libraries
# These patterns match against the output of the 'file' command
denylist_patterns=(
  "PE32.*executable"
  "PE32.*DLL"
  "MS-DOS executable"
  "ELF.*executable"
  "ELF.*shared object"
  "Mach-O.*executable"
  "Mach-O.*dynamically linked shared library"
  "Mach-O.*bundle"
  "Java archive data"
  "compiled Java class"
  "python.*byte-compiled"
  "WebAssembly"
  "current ar archive"
)

# Associative arrays for fast O(1) lookups
declare -A violations
declare -A allowlisted_files
declare -A checked_files

echo "Scanning git-tracked files..."

# Collect all git-tracked files
mapfile -t all_files < <(git ls-files)

echo "Checking ${#all_files[@]} tracked files..."

# First pass: Quick extension check (no file command needed)
extension_regex='\.(exe|dll|so|dylib|a|o|bin|app|wasm|jar|class|pyc|pyd|pyo|lib)$'

for file in "${all_files[@]}"; do
  # Skip if not a regular file or if it's a symlink
  [ -f "$file" ] && [ ! -L "$file" ] || continue

  # Use bash regex matching (no grep subprocess)
  if [[ "$file" =~ $extension_regex ]]; then
    checked_files["$file"]=1

    if [[ -n "${allowlist[$file]}" ]]; then
      allowlisted_files["$file"]=1
    else
      violations["$file"]=1
    fi
  fi
done

echo "Found ${#violations[@]} files with suspicious extensions"

# Second pass: Batch check remaining files with ONE file command
# Build list of files not yet checked
files_to_check=()
for file in "${all_files[@]}"; do
  # Skip if not a regular file or if it's a symlink
  [ -f "$file" ] && [ ! -L "$file" ] || continue
  [[ -z "${checked_files[$file]}" ]] || continue
  files_to_check+=("$file")
done

if [ ${#files_to_check[@]} -gt 0 ]; then
  echo "Deep scanning ${#files_to_check[@]} additional files..."

  # Use xargs to batch process files (handles ARG_MAX limits, processes in chunks of 100)
  # Use process substitution instead of pipe to avoid subshell
  while IFS=: read -r filepath description; do
    # Check if description matches any denylist pattern
    is_binary=false
    for pattern in "${denylist_patterns[@]}"; do
      if [[ "$description" =~ $pattern ]]; then
        is_binary=true
        break
      fi
    done

    if [ "$is_binary" = true ]; then
      if [[ -n "${allowlist[$filepath]}" ]]; then
        allowlisted_files["$filepath"]=1
      else
        violations["$filepath"]=1
      fi
    fi
  done < <(printf '%s\n' "${files_to_check[@]}" | xargs -n 100 -P 1 file 2>/dev/null)
fi

# Report allowlisted files
if [ ${#allowlisted_files[@]} -gt 0 ]; then
  echo ""
  echo "WARNING: The following binaries are temporarily allowlisted:"
  echo ""

  for file in "${!allowlisted_files[@]}"; do
    echo "  [ALLOWLISTED] $file"
    if [ -f "$file" ]; then
      echo "                $(file -b "$file")"
      echo "                Size: $(du -h "$file" | cut -f1)"
    fi
    echo ""
  done

  echo "These files should be removed and replaced with build-time solutions."
  echo ""
fi

# Report violations
if [ ${#violations[@]} -gt 0 ]; then
  echo ""
  echo "ERROR: PREBUILT BINARIES DETECTED!"
  echo ""
  echo "The following prebuilt executables or binary files were found in the repository:"
  echo ""

  for file in "${!violations[@]}"; do
    echo "  [VIOLATION] $file"
    echo "              $(file -b "$file")"
    if [ -f "$file" ]; then
      echo "              Size: $(du -h "$file" | cut -f1)"
    fi
    echo ""
  done

  echo "========================================================================"
  echo ""
  echo "POLICY: Prebuilt binaries are not allowed in this repository"
  echo ""
  echo "This is a security measure to prevent supply chain attacks."
  echo "All executables must be built from source in CI/CD pipelines."
  echo ""
  echo "If you need platform-specific binaries:"
  echo "  1. Build them in the GitHub Actions workflow"
  echo "  2. Use package managers (npm, cargo, apt, brew, etc.)"
  echo "  3. Download from trusted sources during build time"
  echo "  4. Add checksums/signatures verification"
  echo ""
  echo "To remove these files:"
  for file in "${!violations[@]}"; do
    echo "  git rm \"$file\""
  done
  echo ""
  echo "If this is a false positive, add the file to the allowlist in:"
  echo "  scripts/ci/structure/check-prebuilt-binaries.sh"
  echo ""
  echo "========================================================================"

  exit 1
fi

echo "No new prebuilt binaries detected (${#allowlisted_files[@]} allowlisted)"
