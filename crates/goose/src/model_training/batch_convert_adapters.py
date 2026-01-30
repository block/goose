#!/usr/bin/env python3
"""
Batch convert all LoRA adapters to GGUF format.
"""

import sys
import json
from pathlib import Path
from convert_lora_to_gguf import convert_lora_to_gguf


def batch_convert(training_dir: str, llama_cpp_dir: str, python_path: str):
    """
    Convert all adapters in the training directory.
    
    Args:
        training_dir: Path to training directory (e.g., ~/.config/goose/training)
        llama_cpp_dir: Path to llama.cpp repository
        python_path: Path to Python executable
    """
    training_path = Path(training_dir).expanduser()
    
    if not training_path.exists():
        print(f"Training directory not found: {training_path}")
        return
    
    # Find all job directories
    job_dirs = [d for d in training_path.iterdir() if d.is_dir() and d.name.startswith("job-")]
    
    print(f"Found {len(job_dirs)} training jobs")
    print("=" * 80)
    
    results = {
        "total": len(job_dirs),
        "converted": 0,
        "already_converted": 0,
        "failed": 0,
        "skipped": 0,
        "details": []
    }
    
    for i, job_dir in enumerate(sorted(job_dirs), 1):
        job_id = job_dir.name
        print(f"\n[{i}/{len(job_dirs)}] Processing {job_id}...")
        
        # Check if adapter exists
        safetensors_path = job_dir / "adapter_model.safetensors"
        if not safetensors_path.exists():
            print(f"  ⚠️  No adapter found, skipping")
            results["skipped"] += 1
            results["details"].append({
                "job_id": job_id,
                "status": "skipped",
                "reason": "No adapter_model.safetensors found"
            })
            continue
        
        # Convert
        result = convert_lora_to_gguf(str(job_dir), llama_cpp_dir, python_path)
        
        if result["success"]:
            if result.get("already_exists"):
                print(f"  ✓  Already converted")
                results["already_converted"] += 1
                results["details"].append({
                    "job_id": job_id,
                    "status": "already_converted",
                    "output_path": result["output_path"]
                })
            else:
                size_mb = result["size_bytes"] / (1024 * 1024)
                print(f"  ✅ Converted successfully ({size_mb:.1f} MB)")
                results["converted"] += 1
                results["details"].append({
                    "job_id": job_id,
                    "status": "converted",
                    "base_model": result.get("base_model"),
                    "output_path": result["output_path"],
                    "size_mb": round(size_mb, 1)
                })
        else:
            print(f"  ❌ Failed: {result.get('error', 'Unknown error')}")
            results["failed"] += 1
            results["details"].append({
                "job_id": job_id,
                "status": "failed",
                "error": result.get("error")
            })
    
    # Summary
    print("\n" + "=" * 80)
    print("SUMMARY")
    print("=" * 80)
    print(f"Total jobs:          {results['total']}")
    print(f"Newly converted:     {results['converted']}")
    print(f"Already converted:   {results['already_converted']}")
    print(f"Failed:              {results['failed']}")
    print(f"Skipped (no adapter): {results['skipped']}")
    print(f"\nTotal GGUF adapters: {results['converted'] + results['already_converted']}")
    
    # Save detailed results
    results_file = training_path / "conversion_results.json"
    with open(results_file, "w") as f:
        json.dump(results, f, indent=2)
    print(f"\nDetailed results saved to: {results_file}")
    
    return results


def main():
    """CLI entry point."""
    if len(sys.argv) < 4:
        print("Usage: batch_convert_adapters.py <training_dir> <llama_cpp_dir> <python_path>")
        print("\nExample:")
        print("  python3 batch_convert_adapters.py \\")
        print("    ~/.config/goose/training \\")
        print("    /Users/spencermartin/Desktop/Distil/llama.cpp \\")
        print("    ~/.config/goose/axolotl-venv/bin/python")
        sys.exit(1)
    
    training_dir = sys.argv[1]
    llama_cpp_dir = sys.argv[2]
    python_path = sys.argv[3]
    
    results = batch_convert(training_dir, llama_cpp_dir, python_path)
    
    # Exit with error if any conversions failed
    sys.exit(0 if results["failed"] == 0 else 1)


if __name__ == "__main__":
    main()
