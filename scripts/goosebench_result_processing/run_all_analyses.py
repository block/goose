#!/usr/bin/env python3
"""
Script to run all evaluation analysis scripts and combine their results.

This script:
1. Finds all analyze_*.py files in specified directories
2. Runs each analysis script
3. Combines results into a single CSV with provider and model columns
4. Outputs a combined CSV with all metrics

Example usage:
    python run_all_analyses.py --base-dir ./benchmarks --output all-evaluations.csv
"""

import os
import sys
import subprocess
import pandas as pd
from concurrent.futures import ThreadPoolExecutor
from typing import List, Tuple, Dict, Optional, NamedTuple
from dataclasses import dataclass
from pathlib import Path

# Add project root to Python path
project_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
sys.path.insert(0, project_root)

# Get the directory containing this script
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))

# Define analysis script directories and their corresponding evaluation names
ANALYSIS_DIRS = {
    "vibes": [
        ("analyze_blog_summary.py", "blog_summary"),
        ("analyze_squirrel_census.py", "squirrel_census"),
        ("analyze_flappy_bird.py", "flappy_bird"),
        ("analyze_goose_wiki.py", "goose_wiki"),
        ("analyze_restaurant_research.py", "restaurant_research")
    ],
    "core": [
        ("analyze_search_replace.py", "search_replace"),
        # ("analyze_save_fact.py", "save_fact"),
        ("analyze_create_file.py", "create_file"),
        ("analyze_list_files.py", "list_files")
    ]
}

@dataclass
class ScriptResult:
    """Container for script execution results."""
    script_name: str
    script_dir: str
    output_path: str
    return_code: int
    error_message: Optional[str] = None

def check_openai_key_error(error_msg: str) -> bool:
    """Check if the error message indicates a missing OpenAI API key.
    
    Args:
        error_msg: Error message to check
        
    Returns:
        bool: True if error is related to missing OpenAI API key
    """
    return "OPENAI_API_KEY" in error_msg or "openai api key" in error_msg.lower()

def run_analysis_script(script_name: str, script_dir: str, benchmarks_dir: str, output_dir: str) -> ScriptResult:
    """Run a single analysis script.
    
    Args:
        script_name: Name of the analysis script to run
        script_dir: Directory containing the script
        benchmarks_dir: Directory containing benchmark results
        output_dir: Directory to write output files
        
    Returns:
        ScriptResult containing execution results
        
    Raises:
        ValueError: If OpenAI API key is missing
    """
    # Create output directory if it doesn't exist
    os.makedirs(output_dir, exist_ok=True)
    
    # Construct output path
    output_name = script_name.replace("analyze_", "").replace(".py", "-analysis.csv")
    output_path = os.path.join(output_dir, output_name)
    
    # Get full path to the script
    script_path = os.path.join(script_dir, script_name)
    
    # Add the project root to PYTHONPATH so scripts can find their dependencies
    env = os.environ.copy()
    if "PYTHONPATH" in env:
        env["PYTHONPATH"] = f"{project_root}:{env['PYTHONPATH']}"
    else:
        env["PYTHONPATH"] = project_root
    
    # Construct command
    cmd = [
        sys.executable,
        script_path,
        "--benchmarks-dir", benchmarks_dir,
        "--output-dir", output_path
    ]
    
    try:
        # Run the script and capture output
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            check=False,  # Don't raise exception on non-zero exit
            cwd=project_root,  # Run from the project root directory
            env=env  # Use modified environment with updated PYTHONPATH
        )
        
        # Check if there was an error
        if result.returncode != 0:
            error_msg = result.stderr.strip()
            if not error_msg:
                error_msg = result.stdout.strip()
                
            # Check for OpenAI API key error
            if check_openai_key_error(error_msg):
                raise ValueError(f"OpenAI API key is required for {script_name} but not set")
                
            return ScriptResult(
                script_name=script_name,
                script_dir=script_dir,
                output_path=output_path,
                return_code=result.returncode,
                error_message=error_msg
            )
            
        return ScriptResult(
            script_name=script_name,
            script_dir=script_dir,
            output_path=output_path,
            return_code=0
        )
        
    except ValueError as e:
        # Re-raise OpenAI API key errors
        raise
    except Exception as e:
        return ScriptResult(
            script_name=script_name,
            script_dir=script_dir,
            output_path=output_path,
            return_code=1,
            error_message=str(e)
        )

def main() -> None:
    """Main entry point for running all analyses."""
    import argparse
    
    parser = argparse.ArgumentParser(description="Run all evaluation analysis scripts")
    parser.add_argument(
        "--benchmarks-dir",
        default=".",
        help="Base directory containing benchmark results"
    )
    parser.add_argument(
        "--output-dir",
        default="goosebench-evals-processed",
        help="Directory to write output files"
    )
    args = parser.parse_args()
    
    # Convert relative paths to absolute
    benchmarks_dir = os.path.abspath(args.benchmarks_dir)
    output_dir = os.path.abspath(args.output_dir)
    
    print(f"\nRunning all analyses with benchmarks directory: {benchmarks_dir}")
    print(f"Output directory: {output_dir}\n")
    
    # Track OpenAI API key errors
    openai_errors = []
    
    # Run all scripts in parallel
    with ThreadPoolExecutor() as executor:
        futures = []
        for dir_name, scripts in ANALYSIS_DIRS.items():
            script_dir = os.path.join(SCRIPT_DIR, dir_name)
            print(f"Processing scripts in {dir_name}/")
            for script_name, _ in scripts:
                futures.append(
                    executor.submit(run_analysis_script, script_name, script_dir, benchmarks_dir, output_dir)
                )
        
        # Collect results
        results = []
        for dir_name, scripts in ANALYSIS_DIRS.items():
            for script_name, _ in scripts:
                try:
                    result = futures.pop(0).result()
                    results.append(result)
                    
                    # Print results as they complete
                    if result.return_code == 0:
                        print(f"✅ {dir_name}/{script_name}: Success")
                    else:
                        print(f"❌ {dir_name}/{script_name}: Failed")
                        if result.error_message:
                            print(f"   Error: {result.error_message}")
                except ValueError as e:
                    # Collect OpenAI API key errors
                    openai_errors.append(str(e))
                    print(f"❌ {dir_name}/{script_name}: Failed")
                    print(f"   Error: {str(e)}")
    
    # Check for OpenAI API key errors
    if openai_errors:
        print("\n❌ OpenAI API key errors detected:")
        for error in openai_errors:
            print(f"  - {error}")
        print("\nPlease set your OpenAI API key:")
        print("  export OPENAI_API_KEY='your-api-key'")
        sys.exit(1)
    
    # Check if any other scripts failed
    failed_scripts = [r for r in results if r.return_code != 0]
    if failed_scripts:
        print(f"\n❌ {len(failed_scripts)} scripts failed:")
        for result in failed_scripts:
            print(f"  - {result.script_dir}/{result.script_name}")
        sys.exit(1)
    else:
        print("\n✅ All scripts completed successfully")

if __name__ == "__main__":
    main()