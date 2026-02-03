#!/bin/bash

# This script checks if changes in files include the string 'prisma.' and are not in the 'lib' directory

# Get the root directory of the Git repository
root_directory=$(git rev-parse --show-toplevel)

# Get the list of staged files
staged_files=$(git diff --name-only)

# Iterate over the staged files
for file in $staged_files; do
    # Get the full path of the file relative to the root directory
    full_path="$root_directory/$file"
    
    # Check if the file is not in the 'lib' directory
    if [[ ! $file =~ ^lib/ ]]; then
        # Check if the content of the file includes 'prisma.'
        if grep -q 'prisma\.' "$full_path"; then
            echo "Error: Files with 'prisma.' content are only allowed in the 'lib' directory."
            echo "File: $file"
            exit 1
        fi
    fi
done

# If no issues found, exit successfully
exit 0
