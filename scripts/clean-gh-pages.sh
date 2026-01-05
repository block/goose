#!/bin/bash

set -e  # Exit on error

echo "Starting gh-pages branch cleanup..."

dirs_with_visible_files=$(git ls-tree -r origin/gh-pages:pr-preview --name-only 2>/dev/null | \
    grep -v '/\.' | \
    cut -d/ -f1 | \
    sort -u || true)

while IFS= read -r dir; do
    if [ -n "$dir" ]; then
        if ! echo "$dirs_with_visible_files" | grep -q "^${dir}$"; then
            dir_path="pr-preview/$dir"
            echo "Found directory to remove: $dir_path"
        fi
    fi
done <<< "$all_dirs"

CALLBACK="
    root, *rest = filename.split(b'/')
    keep = b'''$dirs_with_visible_files'''.splitlines()
    if root != b'pr-preview':
        # keep anything outside of pr-preview
        return filename
    elif rest and rest[0] not in keep:
        return None
    else:
        return filename
"

uvx git-filter-repo@2.47.0 --filename-callback "$CALLBACK" --refs origin/gh-pages
