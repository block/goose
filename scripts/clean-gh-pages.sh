#!/bin/bash

set -e  # Exit on error

echo "Starting gh-pages branch cleanup..."

REMOVE_PATHS_FILE="/tmp/remove-paths.txt"
> "$REMOVE_PATHS_FILE"

dirs_with_visible_files=$(git ls-tree -r origin/gh-pages:pr-preview --name-only 2>/dev/null | \
    grep -v '/\.' | \
    cut -d/ -f1 | \
    sort -u || true)

all_dirs=$(git ls-tree -d origin/gh-pages:pr-preview --name-only 2>/dev/null || true)

if [ -z "$all_dirs" ]; then
    echo "No directories found in pr-preview or pr-preview does not exist"
    rm "$REMOVE_PATHS_FILE"
    exit 0
fi

while IFS= read -r dir; do
    if [ -n "$dir" ]; then
        if ! echo "$dirs_with_visible_files" | grep -q "^${dir}$"; then
            dir_path="pr-preview/$dir"
            echo "Found directory to remove: $dir_path"
            echo "$dir_path" >> "$REMOVE_PATHS_FILE"
        fi
    fi
done <<< "$all_dirs"

if [ ! -s "$REMOVE_PATHS_FILE" ]; then
    echo "No empty or hidden-file-only directories found. Nothing to clean up."
    rm "$REMOVE_PATHS_FILE"
    exit 0
fi

uvx git-filter-repo@2.47.0 --paths-from-file "$REMOVE_PATHS_FILE" --invert-paths --refs origin/gh-pages
