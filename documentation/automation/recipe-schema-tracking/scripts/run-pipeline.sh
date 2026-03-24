#!/bin/bash
# End-to-end pipeline test
# Usage: ./run-pipeline.sh <old_version> <new_version>
# Example: ./run-pipeline.sh v1.9.0 v1.15.0

set -e

OLD_VERSION=${1:-"v1.9.0"}
NEW_VERSION=${2:-"v1.15.0"}

echo "=========================================="
echo "Recipe Validation Documentation Pipeline"
echo "=========================================="
echo "Old Version: $OLD_VERSION"
echo "New Version: $NEW_VERSION"
echo ""

# Change to output directory
cd "$(dirname "$0")/../output"

echo "Step 1: Extracting validation structure from $OLD_VERSION..."
if ! ../scripts/extract-validation-structure.sh "$OLD_VERSION" > old-validation-structure.json 2>&1; then
    echo "✗ Failed to extract validation structure from $OLD_VERSION" >&2
    echo "Error output:" >&2
    cat old-validation-structure.json >&2
    exit 1
fi
echo "✓ Extracted $(jq '.struct_fields | length' old-validation-structure.json) fields, $(jq '.validation_functions | length' old-validation-structure.json) functions"

echo ""
echo "Step 1b: Extracting schema from $OLD_VERSION..."
if ! ../scripts/extract-schema.sh "$OLD_VERSION" > old-schema.json 2>&1; then
    echo "✗ Failed to extract schema from $OLD_VERSION" >&2
    echo "Error output:" >&2
    cat old-schema.json >&2
    exit 1
fi
echo "✓ Extracted schema ($(jq '.properties | length' old-schema.json) properties)"

echo ""
echo "Step 2: Extracting validation structure from $NEW_VERSION..."
if ! ../scripts/extract-validation-structure.sh "$NEW_VERSION" > new-validation-structure.json 2>&1; then
    echo "✗ Failed to extract validation structure from $NEW_VERSION" >&2
    echo "Error output:" >&2
    cat new-validation-structure.json >&2
    exit 1
fi
echo "✓ Extracted $(jq '.struct_fields | length' new-validation-structure.json) fields, $(jq '.validation_functions | length' new-validation-structure.json) functions"

echo ""
echo "Step 2b: Extracting schema from $NEW_VERSION..."
if ! ../scripts/extract-schema.sh "$NEW_VERSION" > new-schema.json 2>&1; then
    echo "✗ Failed to extract schema from $NEW_VERSION" >&2
    echo "Error output:" >&2
    cat new-schema.json >&2
    exit 1
fi
echo "✓ Extracted schema ($(jq '.properties | length' new-schema.json) properties)"

echo ""
echo "Step 3: Comparing validation structures..."
../scripts/diff-validation-structures.sh old-validation-structure.json new-validation-structure.json > validation-changes.json 2>&1

HAS_CHANGES=$(jq -r '.has_changes' validation-changes.json)
echo "✓ Comparison complete. Has changes: $HAS_CHANGES"

if [ "$HAS_CHANGES" = "true" ]; then
    echo ""
    echo "Changes detected:"
    echo "  - Fields added: $(jq '.changes.struct_fields.added | length' validation-changes.json)"
    echo "  - Fields removed: $(jq '.changes.struct_fields.removed | length' validation-changes.json)"
    echo "  - Fields type changed: $(jq '.changes.struct_fields.type_changed | length' validation-changes.json)"
    echo "  - Fields comment changed: $(jq '.changes.struct_fields.comment_changed | length' validation-changes.json)"
    echo "  - Validation functions added: $(jq '.changes.validation_functions.added | length' validation-changes.json)"
    echo "  - Validation functions removed: $(jq '.changes.validation_functions.removed | length' validation-changes.json)"
    echo "  - Validation functions signature changed: $(jq '.changes.validation_functions.signature_changed | length' validation-changes.json)"
    echo "  - Validation functions error messages changed: $(jq '.changes.validation_functions.error_messages_changed | length' validation-changes.json)"
    
    echo ""
    echo "Step 4: Synthesizing validation changes documentation..."
    
    # Run goose and capture output, filtering out session logs
    goose run --recipe ../recipes/synthesize-validation-changes.yaml 2>&1 | \
        sed -E 's/\x1B\[[0-9;]*[mK]//g' | \
        grep -v "^starting session" | \
        grep -v "^    session id:" | \
        grep -v "^    working directory:" | \
        grep -v "^─── text_editor" | \
        grep -v "^path:" | \
        grep -v "^command:" | \
        grep -v "^Closing session" | \
        grep -v "^Loading recipe:" | \
        grep -v "^Description:" | \
        sed '/^$/N;/^\n$/D' > validation-changes.md.tmp
    
    # Check if we got meaningful content (more than just whitespace)
    if [ -s validation-changes.md.tmp ] && grep -q "# Recipe Validation Changes" validation-changes.md.tmp; then
        mv validation-changes.md.tmp validation-changes.md
        echo "✓ Generated validation-changes.md ($(wc -l < validation-changes.md) lines)"
        echo ""
        echo "=========================================="
        echo "Pipeline Complete!"
        echo "=========================================="
        echo ""
        echo "Output files:"
        echo "  - old-validation-structure.json"
        echo "  - old-schema.json"
        echo "  - new-validation-structure.json"
        echo "  - new-schema.json"
        echo "  - validation-changes.json"
        echo "  - validation-changes.md"
        echo ""
        echo "Review validation-changes.md for documentation updates."
    else
        echo "✗ Failed to generate validation-changes.md"
        exit 1
    fi
else
    echo ""
    echo "=========================================="
    echo "No Changes Detected"
    echo "=========================================="
    echo ""
    echo "No validation changes between $OLD_VERSION and $NEW_VERSION."
    echo "Documentation update not needed."
fi
