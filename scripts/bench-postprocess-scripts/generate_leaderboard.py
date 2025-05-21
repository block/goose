#!/usr/bin/env python3
# Compatible with Python 3.6+
"""
Generate a leaderboard CSV from benchmark results, including server error information.

This script:
1. Looks for model folders in the benchmark directory
2. Finds eval-results/aggregate_metrics.csv in each model folder
3. Extracts key metrics (provider, model_name, eval_suite, eval_name, tool calls, execution time, tokens, score, prompt error, server error)
4. Creates a union of all CSVs with these columns
5. Creates a leaderboard.csv grouping by provider and model_name, averaging numeric columns

Usage:
    python generate_leaderboard.py --benchmark-dir /path/to/benchmark-dir
"""

import argparse
import pandas as pd
from pathlib import Path
import sys


def find_aggregate_metrics_files(benchmark_dir: Path) -> list:
    """Find all aggregate_metrics.csv files in model subdirectories."""
    csv_files = []
    
    # Look for model directories in the benchmark directory
    for model_dir in benchmark_dir.iterdir():
        if model_dir.is_dir():
            # Look for eval-results/aggregate_metrics.csv in each model directory
            eval_results_dir = model_dir / "eval-results"
            if eval_results_dir.exists() and eval_results_dir.is_dir():
                csv_path = eval_results_dir / "aggregate_metrics.csv"
                if csv_path.exists():
                    csv_files.append(csv_path)
    
    return csv_files


def process_csv_files(csv_files: list) -> tuple:
    """
    Process all CSV files and create two dataframes:
    1. A union of all CSVs with selected columns
    2. A leaderboard grouping by provider and model_name with averaged metrics
    """
    selected_columns = [
        'provider', 
        'model_name', 
        'eval_suite', 
        'eval_name', 
        'total_tool_calls_mean', 
        'prompt_execution_time_mean', 
        'total_tokens_mean', 
        'score_mean', 
        'prompt_error_mean',
        'server_error_mean' 
    ]
    
    all_data = []
    
    for csv_file in csv_files:
        try:
            df = pd.read_csv(csv_file)
            
            # Check which selected columns are available
            missing_columns = [col for col in selected_columns if col not in df.columns]
            if missing_columns:
                print(f"Warning: {csv_file} is missing columns: {missing_columns}")
                
                # For missing columns, add them with NaN values
                for col in missing_columns:
                    df[col] = float('nan')
            
            # Select only the columns we care about
            df_subset = df[selected_columns].copy()  # Create a copy to avoid SettingWithCopyWarning
            
            # Add model folder name as additional context
            model_folder = csv_file.parent.parent.name
            df_subset['model_folder'] = model_folder
            
            all_data.append(df_subset)
            
        except Exception as e:
            print(f"Error processing {csv_file}: {str(e)}")
    
    if not all_data:
        raise ValueError("No valid CSV files found with required columns")
    
    # Concatenate all dataframes to create a union
    union_df = pd.concat(all_data, ignore_index=True)
    
    # Create leaderboard by grouping and averaging numerical columns
    numeric_columns = [
        'total_tool_calls_mean', 
        'prompt_execution_time_mean', 
        'total_tokens_mean', 
        'score_mean', 
        'prompt_error_mean',
        'server_error_mean'
    ]
    
    # Group by provider and model_name, then calculate averages for numeric columns
    leaderboard_df = union_df.groupby(['provider', 'model_name'])[numeric_columns].mean().reset_index()
    
    # Sort by score_mean in descending order (highest scores first)
    leaderboard_df = leaderboard_df.sort_values('score_mean', ascending=False)
    
    return union_df, leaderboard_df


def main():
    parser = argparse.ArgumentParser(
        description="Generate a leaderboard CSV from benchmark results, including server error information"
    )
    parser.add_argument(
        "--benchmark-dir",
        type=str,
        required=True,
        help="Path to the benchmark directory containing model subdirectories"
    )
    parser.add_argument(
        "--union-output",
        type=str,
        default="all_metrics.csv",
        help="Output filename for the union of all CSVs (default: all_metrics.csv)"
    )
    parser.add_argument(
        "--leaderboard-output",
        type=str,
        default="leaderboard.csv",
        help="Output filename for the leaderboard (default: leaderboard.csv)"
    )
    
    args = parser.parse_args()
    
    benchmark_dir = Path(args.benchmark_dir)
    if not benchmark_dir.exists() or not benchmark_dir.is_dir():
        print(f"Error: Benchmark directory {benchmark_dir} does not exist or is not a directory")
        sys.exit(1)
    
    try:
        # Find all aggregate_metrics.csv files in model subdirectories
        csv_files = find_aggregate_metrics_files(benchmark_dir)
        
        if not csv_files:
            print(f"No aggregate_metrics.csv files found in any model directory under {benchmark_dir}")
            sys.exit(1)
        
        print(f"Found {len(csv_files)} aggregate_metrics.csv files in model directories")
        
        # Process and create the union and leaderboard dataframes
        union_df, leaderboard_df = process_csv_files(csv_files)
        
        # Save the union CSV to the benchmark directory
        union_output_path = benchmark_dir / args.union_output
        union_df.to_csv(union_output_path, index=False)
        print(f"Union CSV with all metrics saved to: {union_output_path}")
        
        # Save the leaderboard CSV to the benchmark directory
        leaderboard_output_path = benchmark_dir / args.leaderboard_output
        leaderboard_df.to_csv(leaderboard_output_path, index=False)
        print(f"Leaderboard CSV with averaged metrics saved to: {leaderboard_output_path}")
        
        # Print a summary of the leaderboard
        print("\nLeaderboard Summary:")
        pd.set_option('display.max_columns', None)  # Show all columns
        print(leaderboard_df.to_string(index=False))
        
        # Highlight models with server errors
        if 'server_error_mean' in leaderboard_df.columns:
            models_with_errors = leaderboard_df[leaderboard_df['server_error_mean'] > 0]
            if not models_with_errors.empty:
                print("\nWARNING - Models with server errors detected:")
                for _, row in models_with_errors.iterrows():
                    print(f"  * {row['provider']} {row['model_name']} - {row['server_error_mean']*100:.1f}% of evaluations had server errors")
                print("\nThese models may need to be re-run to get accurate results.")
        
    except Exception as e:
        print(f"Error: {str(e)}")
        sys.exit(1)


if __name__ == "__main__":
    main()
