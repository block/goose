#!/usr/bin/env python3
"""
Convert LoRA adapters from safetensors to GGUF format for Ollama integration.
"""

import sys
import json
import subprocess
from pathlib import Path


def convert_lora_to_gguf(adapter_dir: str, llama_cpp_dir: str, python_path: str) -> dict:
    """
    Convert a LoRA adapter from safetensors to GGUF format.
    
    Args:
        adapter_dir: Path to the adapter directory (contains adapter_model.safetensors)
        llama_cpp_dir: Path to llama.cpp repository
        python_path: Path to Python executable with required dependencies
        
    Returns:
        dict with status, message, and output_path
    """
    adapter_path = Path(adapter_dir)
    
    # Check if adapter exists
    safetensors_path = adapter_path / "adapter_model.safetensors"
    if not safetensors_path.exists():
        return {
            "success": False,
            "error": f"Adapter not found: {safetensors_path}",
            "output_path": None
        }
    
    # Check if already converted
    gguf_path = adapter_path / "adapter_model.gguf"
    if gguf_path.exists():
        return {
            "success": True,
            "message": "Adapter already converted to GGUF",
            "output_path": str(gguf_path),
            "already_exists": True
        }
    
    # Read adapter config to get base model
    config_path = adapter_path / "adapter_config.json"
    if not config_path.exists():
        return {
            "success": False,
            "error": f"Adapter config not found: {config_path}",
            "output_path": None
        }
    
    with open(config_path) as f:
        config = json.load(f)
    
    base_model = config.get("base_model_name_or_path", "unknown")
    
    # Run conversion
    convert_script = Path(llama_cpp_dir) / "convert_lora_to_gguf.py"
    if not convert_script.exists():
        return {
            "success": False,
            "error": f"Conversion script not found: {convert_script}",
            "output_path": None
        }
    
    try:
        result = subprocess.run(
            [
                python_path,
                str(convert_script),
                str(adapter_path),
                "--outfile", str(gguf_path),
                "--outtype", "f16",
            ],
            capture_output=True,
            text=True,
            timeout=300  # 5 minute timeout
        )
        
        if result.returncode != 0:
            return {
                "success": False,
                "error": f"Conversion failed: {result.stderr}",
                "output_path": None,
                "stdout": result.stdout,
                "stderr": result.stderr
            }
        
        # Verify output exists
        if not gguf_path.exists():
            return {
                "success": False,
                "error": "Conversion completed but output file not found",
                "output_path": None,
                "stdout": result.stdout
            }
        
        return {
            "success": True,
            "message": f"Successfully converted adapter for {base_model}",
            "output_path": str(gguf_path),
            "base_model": base_model,
            "size_bytes": gguf_path.stat().st_size,
            "stdout": result.stdout
        }
        
    except subprocess.TimeoutExpired:
        return {
            "success": False,
            "error": "Conversion timed out after 5 minutes",
            "output_path": None
        }
    except Exception as e:
        return {
            "success": False,
            "error": f"Conversion error: {str(e)}",
            "output_path": None
        }


def main():
    """CLI entry point for testing."""
    if len(sys.argv) < 4:
        print("Usage: convert_lora_to_gguf.py <adapter_dir> <llama_cpp_dir> <python_path>")
        sys.exit(1)
    
    adapter_dir = sys.argv[1]
    llama_cpp_dir = sys.argv[2]
    python_path = sys.argv[3]
    
    result = convert_lora_to_gguf(adapter_dir, llama_cpp_dir, python_path)
    print(json.dumps(result, indent=2))
    
    sys.exit(0 if result["success"] else 1)


if __name__ == "__main__":
    main()
