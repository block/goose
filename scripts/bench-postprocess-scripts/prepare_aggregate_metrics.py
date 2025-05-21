#!/usr/bin/env python3
# Compatible with Python 3.6+
"""
Prepare aggregate_metrics.csv files from individual eval-results.json files with error detection.

This script:
1. Looks for model folders in the benchmark directory
2. For each model folder, finds all eval-results.json files in subfolders
3. Checks session files for server errors
4. Extracts metrics from these files and combines them
5. Creates an eval-results directory in each model folder
6. Saves a aggregate_metrics.csv file with aggregated metrics

Usage:
    python prepare_aggregate_metrics.py --benchmark-dir /path/to/benchmark-dir
"""

import argparse
import json
import pandas as pd
from pathlib import Path
import sys

def extract_provider_model(model_dir):
    """Extract provider and model name from directory name."""
    dir_name = model_dir.name
    parts = dir_name.split('-')
    
    if len(parts) > 1:
        model_name = parts[-1]  # Last part is the model name
        provider = '-'.join(parts[:-1])  # Everything else is the provider
    else:
        model_name = dir_name
        provider = "unknown"
    
    return provider, model_name

def find_eval_results_files(model_dir):
    """Find all eval-results.json files in a model directory."""
    return list(model_dir.glob("**/eval-results.json"))

def find_session_files(model_dir):
    """Find all session jsonl files in a model directory."""
    return list(model_dir.glob("**/*.jsonl"))

def check_for_errors_in_session(session_file):
    """Check if a session file contains server errors."""
    try:
        error_found = False
        error_messages = []
        
        with open(session_file, 'r') as f:
            for line in f:
                try:
                    message_obj = json.loads(line.strip())
                    # Check for error messages in the content
                    if 'content' in message_obj and isinstance(message_obj['content'], list):
                        for content_item in message_obj['content']:
                            if isinstance(content_item, dict) and 'text' in content_item:
                                text = content_item['text']
                                if 'Server error' in text or 'error_code' in text or 'TEMPORARILY_UNAVAILABLE' in text:
                                    error_found = True
                                    error_messages.append(text)
                except json.JSONDecodeError:
                    continue
        
        return error_found, error_messages
    except Exception as e:
        print(f"Error checking session file {session_file}: {str(e)}")
        return False, []

def extract_metrics_from_eval_file(eval_file, provider, model_name, session_files):
    """Extract metrics from an eval-results.json file."""
    try:
        with open(eval_file, 'r') as f:
            data = json.load(f)
        
        # Extract directory structure to determine eval suite and name
        path_parts = eval_file.parts
        run_index = -1
        for i, part in enumerate(path_parts):
            if part.startswith("run-"):
                run_index = i
                break
        
        if run_index == -1 or run_index + 2 >= len(path_parts):
            print(f"Warning: Could not determine eval suite and name from {eval_file}")
            return None
        
        run_number = path_parts[run_index].split('-')[1]  # Extract "0" from "run-0"
        eval_suite = path_parts[run_index + 1]  # Directory after run-N
        eval_name = path_parts[run_index + 2]  # Directory after eval_suite
        
        # Create a row with basic identification
        row = {
            'provider': provider,
            'model_name': model_name,
            'eval_suite': eval_suite,
            'eval_name': eval_name,
            'run': run_number
        }
        
        # Check for server errors in session files for this evaluation
        eval_dir = eval_file.parent
        related_session_files = [sf for sf in session_files if eval_dir in sf.parents]
        
        server_error_found = False
        for session_file in related_session_files:
            error_found, _ = check_for_errors_in_session(session_file)
            if error_found:
                server_error_found = True
                break
        
        # Add server error flag
        row['server_error'] = 1 if server_error_found else 0
        
        # Extract all metrics (flatten the JSON structure)
        if isinstance(data, dict):
            metrics = {}
            
            # Extract top-level metrics
            for key, value in data.items():
                if isinstance(value, (int, float)) and not isinstance(value, bool):
                    metrics[key] = value
            
            # Look for nested metrics structure (list of [name, value] pairs)
            if 'metrics' in data and isinstance(data['metrics'], list):
                for metric_item in data['metrics']:
                    if isinstance(metric_item, list) and len(metric_item) == 2:
                        metric_name = metric_item[0]
                        metric_value = metric_item[1]
                        
                        # Handle different value formats
                        if isinstance(metric_value, dict):
                            if 'Integer' in metric_value:
                                metrics[metric_name] = int(metric_value['Integer'])
                            elif 'Float' in metric_value:
                                metrics[metric_name] = float(metric_value['Float'])
                            elif 'Bool' in metric_value:
                                metrics[metric_name] = 1 if metric_value['Bool'] else 0
                            # Skip string values for aggregation
                        elif isinstance(metric_value, (int, float)) and not isinstance(metric_value, bool):
                            metrics[metric_name] = metric_value
                        elif isinstance(metric_value, bool):
                            metrics[metric_name] = 1 if metric_value else 0
            
            # Look for metrics in other common locations
            for metric_location in ['metrics', 'result', 'evaluation']:
                if metric_location in data and isinstance(data[metric_location], dict):
                    for key, value in data[metric_location].items():
                        if isinstance(value, (int, float)) and not isinstance(value, bool):
                            metrics[key] = value
                        elif isinstance(value, bool):
                            metrics[key] = 1 if value else 0
            
            # Add all metrics to the row
            row.update(metrics)
            
            # Ensure a score is present (if not, add a placeholder)
            if 'score' not in row:
                # Try to use existing fields to calculate a score
                if server_error_found:
                    row['score'] = 0  # Failed runs get a zero score
                else:
                    # Set a default based on presence of "success" fields
                    for key in row:
                        if 'success' in key.lower() and isinstance(row[key], (int, float)):
                            row['score'] = row[key]
                            break
                    else:
                        # No success field found, mark as NaN
                        row['score'] = float('nan')
            
            return row
        else:
            print(f"Warning: Unexpected format in {eval_file}")
            return None
    
    except Exception as e:
        print(f"Error processing {eval_file}: {str(e)}")
        return None

def process_model_directory(model_dir):
    """Process a model directory to create aggregate_metrics.csv."""
    provider, model_name = extract_provider_model(model_dir)
    
    # Find all eval results files
    eval_files = find_eval_results_files(model_dir)
    if not eval_files:
        print(f"No eval-results.json files found in {model_dir}")
        return False
    
    # Find all session files for error checking
    session_files = find_session_files(model_dir)
    
    # Extract metrics from each eval file
    rows = []
    for eval_file in eval_files:
        row = extract_metrics_from_eval_file(eval_file, provider, model_name, session_files)
        if row is not None:
            rows.append(row)
    
    if not rows:
        print(f"No valid metrics extracted from {model_dir}")
        return False
    
    # Create a dataframe from all rows
    combined_df = pd.DataFrame(rows)
    
    # Calculate aggregates for numeric columns, grouped by eval_suite, eval_name
    numeric_cols = combined_df.select_dtypes(include=['number']).columns.tolist()
    # Exclude the run column from aggregation
    if 'run' in numeric_cols:
        numeric_cols.remove('run')
    
    # Group by provider, model_name, eval_suite, eval_name and calculate mean for numeric columns
    group_by_cols = ['provider', 'model_name', 'eval_suite', 'eval_name']
    agg_dict = {col: 'mean' for col in numeric_cols}
    
    # Only perform aggregation if we have numeric columns
    if numeric_cols:
        aggregate_df = combined_df.groupby(group_by_cols).agg(agg_dict).reset_index()
        
        # Rename columns to add _mean suffix for the averaged metrics
        for col in numeric_cols:
            aggregate_df = aggregate_df.rename(columns={col: f"{col}_mean"})
    else:
        print(f"Warning: No numeric metrics found in {model_dir}")
        # Create a minimal dataframe with just the grouping columns
        aggregate_df = combined_df[group_by_cols].drop_duplicates()
    
    # Make sure we have prompt_execution_time_mean and prompt_error_mean columns
    # These are expected by the generate_leaderboard.py script
    if 'prompt_execution_time_mean' not in aggregate_df.columns:
        aggregate_df['prompt_execution_time_mean'] = float('nan')
    
    if 'prompt_error_mean' not in aggregate_df.columns:
        aggregate_df['prompt_error_mean'] = float('nan')
    
    # Add server_error_mean column if not present
    if 'server_error_mean' not in aggregate_df.columns:
        aggregate_df['server_error_mean'] = 0.0
    
    # Create eval-results directory
    eval_results_dir = model_dir / "eval-results"
    eval_results_dir.mkdir(exist_ok=True)
    
    # Save to CSV
    csv_path = eval_results_dir / "aggregate_metrics.csv"
    aggregate_df.to_csv(csv_path, index=False)
    
    # Count number of evaluations that had server errors
    if 'server_error_mean' in aggregate_df.columns:
        error_count = len(aggregate_df[aggregate_df['server_error_mean'] > 0])
        total_count = len(aggregate_df)
        print(f"Saved aggregate metrics to {csv_path} with {len(aggregate_df)} rows " +
              f"({error_count}/{total_count} evals had server errors)")
    else:
        print(f"Saved aggregate metrics to {csv_path} with {len(aggregate_df)} rows")
    
    return True

def main():
    parser = argparse.ArgumentParser(
        description="Prepare aggregate_metrics.csv files from eval-results.json files with error detection"
    )
    parser.add_argument(
        "--benchmark-dir",
        type=str,
        required=True,
        help="Path to the benchmark directory containing model subdirectories"
    )
    
    args = parser.parse_args()
    
    # Convert path to Path object and validate it exists
    benchmark_dir = Path(args.benchmark_dir)
    if not benchmark_dir.exists() or not benchmark_dir.is_dir():
        print(f"Error: Benchmark directory {benchmark_dir} does not exist or is not a directory")
        sys.exit(1)
    
    success_count = 0
    
    # Process each model directory
    for model_dir in benchmark_dir.iterdir():
        if model_dir.is_dir() and not model_dir.name.startswith('.'):
            if process_model_directory(model_dir):
                success_count += 1
    
    if success_count == 0:
        print("No aggregate_metrics.csv files were created")
        sys.exit(1)
    
    print(f"Successfully created aggregate_metrics.csv files for {success_count} model directories")
    print("You can now run generate_leaderboard.py to create the final leaderboard.")
    print("Note: The server_error_mean column indicates the average rate of server errors across evaluations.")

if __name__ == "__main__":
    main()

