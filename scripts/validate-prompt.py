#!/usr/bin/env python3
"""
scripts/validate-prompt.py
Simple Python validator for prompt JSON files in the prompt library.
This is a convenience script to check required fields and extension counts
before opening a PR. It is not executed by Goose itself.

Usage:
    python3 scripts/validate-prompt.py path/to/prompt.json
"""
import sys
import json
from pathlib import Path

def validate(file_path):
    p = Path(file_path)
    if not p.exists():
        print(f"File not found: {file_path}")
        return 2
    try:
        data = json.loads(p.read_text())
    except Exception as e:
        print(f"JSON parse error for {file_path}: {e}")
        return 3

    required = ['id','title','description','category','example_prompt','example_result','extensions']
    missing = [k for k in required if k not in data]
    if missing:
        print(f"Missing required fields: {', '.join(missing)}")
        return 4

    exts = data.get('extensions')
    if not isinstance(exts, list) or len(exts) < 3:
        print(f"'extensions' must be a list with 3+ items (found {type(exts).__name__} with length {len(exts) if isinstance(exts,list) else 'N/A'})")
        return 5

    for i,ext in enumerate(exts):
        if not all(k in ext for k in ('name','description','is_builtin','environmentVariables')):
            print(f"Extension at index {i} missing required fields (name, description, is_builtin, environmentVariables)")
            return 6
        if not isinstance(ext.get('environmentVariables'), list):
            print(f"Extension environmentVariables must be a list at index {i}")
            return 7

    # small filename check
    fname = p.stem
    normalized_title = ''.join(c.lower() if c.isalnum() else '-' for c in data.get('title','')).strip('-')
    if normalized_title != fname:
        print(f"Warning: normalized title '{normalized_title}' does not match filename '{fname}' — recommended but optional.")

    print(f"VALID: {file_path} — {data.get('title')}")
    return 0

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print('Usage: validate-prompt.py <file.json>')
        sys.exit(1)
    rc = validate(sys.argv[1])
    sys.exit(rc)
