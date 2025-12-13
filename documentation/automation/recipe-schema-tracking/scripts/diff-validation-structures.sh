#!/bin/bash
# Compare two validation structure files and output changes
# Usage: ./diff-validation-structures.sh old-validation-structure.json new-validation-structure.json
# Output: validation-changes.json

set -e

OLD_FILE=${1:-"old-validation-structure.json"}
NEW_FILE=${2:-"new-validation-structure.json"}

if [ ! -f "$OLD_FILE" ]; then
    echo "Error: Old validation structure file not found: $OLD_FILE" >&2
    exit 1
fi

if [ ! -f "$NEW_FILE" ]; then
    echo "Error: New validation structure file not found: $NEW_FILE" >&2
    exit 1
fi

# Extract versions for metadata
OLD_VERSION=$(jq -r '.version' "$OLD_FILE")
NEW_VERSION=$(jq -r '.version' "$NEW_FILE")

# Build the changes JSON using jq
jq -n \
  --arg old_version "$OLD_VERSION" \
  --arg new_version "$NEW_VERSION" \
  --arg compared_at "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" \
  --argjson old_data "$(cat "$OLD_FILE")" \
  --argjson new_data "$(cat "$NEW_FILE")" \
  '
  {
    old_version: $old_version,
    new_version: $new_version,
    compared_at: $compared_at,
    has_changes: false,
    changes: {
      struct_fields: {
        added: [],
        removed: [],
        type_changed: [],
        comment_changed: []
      },
      validation_functions: {
        added: [],
        removed: [],
        signature_changed: [],
        error_messages_changed: []
      }
    }
  } |
  
  # Detect field changes
  . as $result |
  
  # Find added fields (in new but not in old) - compare by struct.field
  ($new_data.struct_fields | map(.struct + "." + .field)) as $new_fields |
  ($old_data.struct_fields | map(.struct + "." + .field)) as $old_fields |
  ($new_fields - $old_fields) as $added_field_keys |
  
  # Find removed fields (in old but not in new) - compare by struct.field
  ($old_fields - $new_fields) as $removed_field_keys |
  
  # Find fields with type changes - compare by struct.field
  (
    $new_data.struct_fields | 
    map(select((.struct + "." + .field) as $key | $old_fields | contains([$key])) | 
        {struct: .struct, field: .field, new_type: .type, new_comment: .inline_comment}
    )
  ) as $new_common |
  (
    $old_data.struct_fields | 
    map(select((.struct + "." + .field) as $key | $new_fields | contains([$key])) | 
        {struct: .struct, field: .field, old_type: .type, old_comment: .inline_comment}
    )
  ) as $old_common |
  
  # Compare types and comments for common fields
  (
    $new_common | map(
      . as $new_item |
      ($old_common | map(select(.struct == $new_item.struct and .field == $new_item.field)) | .[0]) as $old_item |
      if $old_item.old_type != $new_item.new_type then
        {
          struct: $new_item.struct,
          field: $new_item.field,
          old_type: $old_item.old_type,
          new_type: $new_item.new_type
        }
      else
        empty
      end
    )
  ) as $type_changed |
  
  (
    $new_common | map(
      . as $new_item |
      ($old_common | map(select(.struct == $new_item.struct and .field == $new_item.field)) | .[0]) as $old_item |
      if $old_item.old_comment != $new_item.new_comment then
        {
          struct: $new_item.struct,
          field: $new_item.field,
          old_comment: $old_item.old_comment,
          new_comment: $new_item.new_comment
        }
      else
        empty
      end
    )
  ) as $comment_changed |
  
  # Find validation function changes
  ($new_data.validation_functions | map(.function)) as $new_funcs |
  ($old_data.validation_functions | map(.function)) as $old_funcs |
  ($new_funcs - $old_funcs) as $added_funcs |
  ($old_funcs - $new_funcs) as $removed_funcs |
  
  # Find functions with signature changes
  (
    $new_data.validation_functions | 
    map(select(.function as $f | $old_funcs | contains([$f])) | 
        {function: .function, new_signature: .signature, new_errors: .error_messages}
    )
  ) as $new_common_funcs |
  (
    $old_data.validation_functions | 
    map(select(.function as $f | $new_funcs | contains([$f])) | 
        {function: .function, old_signature: .signature, old_errors: .error_messages}
    )
  ) as $old_common_funcs |
  
  (
    $new_common_funcs | map(
      . as $new_func |
      ($old_common_funcs | map(select(.function == $new_func.function)) | .[0]) as $old_func |
      if $old_func.old_signature != $new_func.new_signature then
        {
          function: $new_func.function,
          old_signature: $old_func.old_signature,
          new_signature: $new_func.new_signature
        }
      else
        empty
      end
    )
  ) as $signature_changed |
  
  (
    $new_common_funcs | map(
      . as $new_func |
      ($old_common_funcs | map(select(.function == $new_func.function)) | .[0]) as $old_func |
      if $old_func.old_errors != $new_func.new_errors then
        {
          function: $new_func.function,
          old_errors: $old_func.old_errors,
          new_errors: $new_func.new_errors
        }
      else
        empty
      end
    )
  ) as $error_messages_changed |
  
  # Build final result with detected changes
  .changes.struct_fields.added = (
    $added_field_keys | map(
      . as $key |
      ($key | split(".")) as $parts |
      $new_data.struct_fields | map(select(.struct == $parts[0] and .field == $parts[1])) | .[0]
    )
  ) |
  .changes.struct_fields.removed = (
    $removed_field_keys | map(
      . as $key |
      ($key | split(".")) as $parts |
      $old_data.struct_fields | map(select(.struct == $parts[0] and .field == $parts[1])) | .[0]
    )
  ) |
  .changes.struct_fields.type_changed = $type_changed |
  .changes.struct_fields.comment_changed = $comment_changed |
  
  .changes.validation_functions.added = (
    $added_funcs | map(
      . as $func |
      $new_data.validation_functions | map(select(.function == $func)) | .[0]
    )
  ) |
  .changes.validation_functions.removed = (
    $removed_funcs | map(
      . as $func |
      $old_data.validation_functions | map(select(.function == $func)) | .[0]
    )
  ) |
  .changes.validation_functions.signature_changed = $signature_changed |
  .changes.validation_functions.error_messages_changed = $error_messages_changed |
  
  # Set has_changes flag
  .has_changes = (
    (.changes.struct_fields.added | length) > 0 or
    (.changes.struct_fields.removed | length) > 0 or
    (.changes.struct_fields.type_changed | length) > 0 or
    (.changes.struct_fields.comment_changed | length) > 0 or
    (.changes.validation_functions.added | length) > 0 or
    (.changes.validation_functions.removed | length) > 0 or
    (.changes.validation_functions.signature_changed | length) > 0 or
    (.changes.validation_functions.error_messages_changed | length) > 0
  )
  '


