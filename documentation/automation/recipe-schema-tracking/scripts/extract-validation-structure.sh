#!/bin/bash
# Extract validation structure from Rust source files
# Usage: ./extract-validation-structure.sh <version>
# Example: ./extract-validation-structure.sh v1.15.0

set -e

VERSION=${1:-"main"}
GOOSE_REPO=${GOOSE_REPO:-"$HOME/Development/goose"}

if [ ! -d "$GOOSE_REPO" ]; then
    echo "Error: GOOSE_REPO directory not found: $GOOSE_REPO" >&2
    exit 1
fi

cd "$GOOSE_REPO"

# Verify version exists (for non-main versions)
if [ "$VERSION" != "main" ]; then
    if ! git rev-parse "$VERSION" >/dev/null 2>&1; then
        echo "Error: Version $VERSION not found in git history" >&2
        exit 1
    fi
fi

# Start JSON output
# Use ISO 8601 format that works on both macOS and Linux
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date -u -Iseconds 2>/dev/null || date -u)
cat << EOF
{
  "version": "$VERSION",
  "extracted_at": "$TIMESTAMP",
  "struct_fields": [
EOF

# Extract fields from multiple structs
FIRST_FIELD=true

# Get file content from git history or working directory
if [ "$VERSION" = "main" ]; then
    MOD_RS_CONTENT=$(cat crates/goose/src/recipe/mod.rs)
else
    MOD_RS_CONTENT=$(git show "$VERSION:crates/goose/src/recipe/mod.rs" 2>/dev/null || {
        echo "Error: Failed to read mod.rs from version $VERSION" >&2
        exit 1
    })
fi

# List of structs to extract (in order of appearance in file)
STRUCTS="Recipe Author Settings Response SubRecipe RecipeParameter"

# Create a temporary file to collect all fields
TEMP_FIELDS=$(mktemp)

for STRUCT_NAME in $STRUCTS; do
  # Extract just this struct's definition (from "pub struct Name" to the closing "}")
  echo "$MOD_RS_CONTENT" | awk "
    /^pub struct $STRUCT_NAME/ { in_struct=1; next }
    in_struct && /^}/ { exit }
    in_struct && /^[[:space:]]+pub [a-z_]+:/ { print }
  " | while IFS= read -r line; do
      # Extract field name (word after 'pub ')
      field_name=$(echo "$line" | sed -E 's/.*pub ([a-z_]+):.*/\1/')
      
      # Extract type (between : and , or // or end of line)
      field_type=$(echo "$line" | sed -E 's/.*:\s*([^,\/]+).*/\1/' | sed 's/[[:space:]]*$//')
      
      # Extract inline comment (after //)
      inline_comment=""
      if echo "$line" | grep -q "//"; then
        inline_comment=$(echo "$line" | sed -E 's/.*\/\/\s*(.*)$/\1/' | sed 's/[[:space:]]*$//' | sed 's/"/\\"/g')
      fi
      
      # Check if optional
      is_optional="false"
      if echo "$field_type" | grep -q "Option<"; then
        is_optional="true"
      fi
      
      # Output to temp file
      cat << FIELD_JSON >> "$TEMP_FIELDS"
{
  "struct": "$STRUCT_NAME",
  "field": "$field_name",
  "type": "$field_type",
  "optional": $is_optional,
  "inline_comment": "$inline_comment"
}
FIELD_JSON
    done
done

# Output fields with proper comma separation
if [ -s "$TEMP_FIELDS" ]; then
    # Read all JSON objects into an array and format with commas
    jq -s '.' "$TEMP_FIELDS" | jq -r 'to_entries | .[] | (if .key > 0 then "," else "" end) + "    " + (.value | tostring)'
fi

rm -f "$TEMP_FIELDS"

# Close struct_fields array, start validation_functions
cat << EOF

  ],
  "validation_functions": [
EOF

# Extract validation functions with error messages and code snippets
FIRST_FUNC=true

# Get validation file content from git history or working directory
# Note: validate_recipe.rs may not exist in older versions
if [ "$VERSION" = "main" ]; then
    if [ -f crates/goose/src/recipe/validate_recipe.rs ]; then
        VALIDATE_RS_CONTENT=$(cat crates/goose/src/recipe/validate_recipe.rs)
    else
        VALIDATE_RS_CONTENT=""
    fi
else
    VALIDATE_RS_CONTENT=$(git show "$VERSION:crates/goose/src/recipe/validate_recipe.rs" 2>/dev/null || echo "")
fi

if [ -n "$VALIDATE_RS_CONTENT" ]; then
    echo "$VALIDATE_RS_CONTENT" | rg "^fn validate_" -A 30 | \
    awk '
      /^fn validate_/ {
        if (func_name != "") {
          # Output previous function
          if (first_func == "true") {
            first_func = "false"
          } else {
            print ","
          }
          printf "    {\n"
          printf "      \"function\": \"%s\",\n", func_name
          printf "      \"signature\": \"%s\",\n", signature
          printf "      \"error_messages\": [%s],\n", error_msgs
          printf "      \"code_snippet\": %s\n", code_snippet
          printf "    }"
        }
        
        # Start new function
        func_name = $0
        gsub(/^fn /, "", func_name)
        gsub(/\(.*/, "", func_name)
        signature = $0
        gsub(/"/, "\\\"", signature)
        error_msgs = ""
        code_snippet = "\"...\""
        first_func = (first_func == "") ? "true" : first_func
      }
      
      /anyhow::anyhow!\("/ {
        # Extract error message
        msg = $0
        gsub(/.*anyhow::anyhow!\("/, "", msg)
        gsub(/".*/, "", msg)
        gsub(/"/, "\\\"", msg)
        if (error_msgs != "") error_msgs = error_msgs ", "
        error_msgs = error_msgs "\"" msg "\""
      }
      
      END {
        # Output last function
        if (func_name != "") {
          if (first_func != "true") {
            print ","
          }
          printf "    {\n"
          printf "      \"function\": \"%s\",\n", func_name
          printf "      \"signature\": \"%s\",\n", signature
          printf "      \"error_messages\": [%s],\n", error_msgs
          printf "      \"code_snippet\": %s\n", code_snippet
          printf "    }"
        }
      }
    '
fi

# Close validation_functions array and JSON
cat << EOF

  ]
}
EOF
