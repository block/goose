#!/usr/bin/env -S uv run --quiet --script
# /// script
# dependencies = ["gguf"]
# ///
"""
Extract tokenizer data from GGUF model file and save as tokenizer.json
"""
import sys
import json
from pathlib import Path
from gguf import GGUFReader

def extract_tokenizer(gguf_path, output_path=None):
    """Extract tokenizer from GGUF file and save as JSON"""
    gguf_path = Path(gguf_path)

    if not gguf_path.exists():
        print(f"Error: Model file not found: {gguf_path}")
        sys.exit(1)

    print(f"Reading GGUF file: {gguf_path}")
    reader = GGUFReader(gguf_path)

    tokenizer_data = {}
    for field in reader.fields.values():
        if field.name.startswith("tokenizer."):
            key = field.name.replace("tokenizer.", "")
            tokenizer_data[key] = field.parts[-1].tolist() if hasattr(field.parts[-1], 'tolist') else field.parts[-1]

    if not tokenizer_data:
        print("Error: No tokenizer data found in GGUF file")
        sys.exit(1)

    # Default output path: same directory as model, with _tokenizer.json suffix
    if output_path is None:
        output_path = gguf_path.parent / f"{gguf_path.stem}_tokenizer.json"
    else:
        output_path = Path(output_path)

    print(f"Writing tokenizer to: {output_path}")
    with open(output_path, "w") as f:
        json.dump(tokenizer_data, f, indent=2)

    print(f"✓ Successfully extracted tokenizer with {len(tokenizer_data)} fields")
    return output_path

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python extract_tokenizer_from_gguf.py <model.gguf> [output.json]")
        print("\nExample:")
        print("  python extract_tokenizer_from_gguf.py model.gguf")
        print("  python extract_tokenizer_from_gguf.py model.gguf tokenizer.json")
        sys.exit(1)

    gguf_path = sys.argv[1]
    output_path = sys.argv[2] if len(sys.argv) > 2 else None

    extract_tokenizer(gguf_path, output_path)
