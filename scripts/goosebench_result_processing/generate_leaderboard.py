#!/usr/bin/env python3
"""
Script to generate a leaderboard from evaluation results.

This script:
1. Reads evaluation results from CSV files
2. Calculates success rates and average correctness scores
3. Generates a leaderboard sorted by success rate
4. Outputs results to a CSV file

Example usage:
    python generate_leaderboard.py --input-dir ./goosebench-evals-processed --output leaderboard.csv
"""

import os
import sys
import pandas as pd
from typing import List, Dict, Optional
from dataclasses import dataclass
from pathlib import Path

@dataclass
class ModelMetrics:
    """Container for model performance metrics."""
    provider: str
    model: str
    success_rate: float
    avg_correctness_score_all: float
    avg_correctness_score_successful: float
    total_runs: int
    successful_runs: int
    avg_tokens_all: float
    avg_tokens_successful: float
    avg_tool_calls_all: float
    avg_tool_calls_successful: float
    avg_exec_time_all: float
    avg_exec_time_successful: float

class LeaderboardError(Exception):
    """Base exception for leaderboard-related errors."""
    pass

def load_evaluation_results(input_dir: str) -> List[pd.DataFrame]:
    """Load evaluation results from CSV files.
    
    Args:
        input_dir: Directory containing evaluation CSV files
        
    Returns:
        List of pandas DataFrames containing evaluation results
        
    Raises:
        LeaderboardError: If no valid CSV files are found
    """
    dfs = []
    for file in os.listdir(input_dir):
        if not file.endswith("-analysis.csv"):
            continue
            
        file_path = os.path.join(input_dir, file)
        try:
            df = pd.read_csv(file_path)
            dfs.append(df)
        except Exception as e:
            print(f"Warning: Error reading {file_path}: {str(e)}")
            continue
    
    if not dfs:
        raise LeaderboardError(f"No valid CSV files found in {input_dir}")
        
    return dfs

def calculate_model_metrics(df: pd.DataFrame) -> Dict[str, ModelMetrics]:
    """Calculate performance metrics for each model.
    
    Args:
        df: DataFrame containing evaluation results
        
    Returns:
        Dictionary mapping model names to their metrics
    """
    metrics: Dict[str, ModelMetrics] = {}
    
    # Group by provider and model
    for (provider, model), group in df.groupby(['provider', 'model']):
        total_runs = group['total_runs'].iloc[0]  # Use the total_runs column directly
        successful_runs = group['n_successful_runs'].iloc[0]  # Use n_successful_runs column
        success_rate = group['success_rate'].iloc[0]  # Use success_rate column directly
        
        # Use the average metrics columns directly
        avg_correctness_score_all = group['avg_correctness_score'].iloc[0]  # Fixed column name
        avg_tokens_all = group['avg_tokens_all'].iloc[0]
        avg_tool_calls_all = group['avg_tool_calls_all'].iloc[0]
        avg_exec_time_all = group['avg_exec_time_all'].iloc[0]
        
        # Get metrics for successful runs
        avg_correctness_score_successful = avg_correctness_score_all  # Same since we already have averages
        avg_tokens_successful = group['avg_tokens_successful'].iloc[0]
        avg_tool_calls_successful = group['avg_tool_calls_successful'].iloc[0]
        avg_exec_time_successful = group['avg_exec_time_successful'].iloc[0]
        
        metrics[f"{provider}-{model}"] = ModelMetrics(
            provider=provider,
            model=model,
            success_rate=success_rate,
            avg_correctness_score_all=avg_correctness_score_all,
            avg_correctness_score_successful=avg_correctness_score_successful,
            total_runs=total_runs,
            successful_runs=successful_runs,
            avg_tokens_all=avg_tokens_all,
            avg_tokens_successful=avg_tokens_successful,
            avg_tool_calls_all=avg_tool_calls_all,
            avg_tool_calls_successful=avg_tool_calls_successful,
            avg_exec_time_all=avg_exec_time_all,
            avg_exec_time_successful=avg_exec_time_successful
        )
    
    return metrics

def average_metrics_across_tasks(task_metrics_list: List[Dict[str, ModelMetrics]]) -> Dict[str, ModelMetrics]:
    """Average metrics across all tasks for each model.
    
    Args:
        task_metrics_list: List of dictionaries containing metrics for each task
        
    Returns:
        Dictionary mapping model names to their averaged metrics
    """
    # Get all unique model names across all tasks
    all_models = set()
    for task_metrics in task_metrics_list:
        all_models.update(task_metrics.keys())
    
    # Initialize averaged metrics
    averaged_metrics: Dict[str, ModelMetrics] = {}
    
    # For each model, average its metrics across all tasks
    for model in all_models:
        model_metrics_list = []
        for task_metrics in task_metrics_list:
            if model in task_metrics:
                model_metrics_list.append(task_metrics[model])
        
        if not model_metrics_list:
            continue
            
        # Calculate averages
        provider = model_metrics_list[0].provider  # All tasks should have same provider
        model_name = model_metrics_list[0].model  # All tasks should have same model name
        
        # Average all metrics
        success_rate = sum(m.success_rate for m in model_metrics_list) / len(model_metrics_list)
        avg_correctness_score_all = sum(m.avg_correctness_score_all for m in model_metrics_list) / len(model_metrics_list)
        avg_correctness_score_successful = sum(m.avg_correctness_score_successful for m in model_metrics_list) / len(model_metrics_list)
        total_runs = sum(m.total_runs for m in model_metrics_list)
        successful_runs = sum(m.successful_runs for m in model_metrics_list)
        avg_tokens_all = sum(m.avg_tokens_all for m in model_metrics_list) / len(model_metrics_list)
        avg_tokens_successful = sum(m.avg_tokens_successful for m in model_metrics_list) / len(model_metrics_list)
        avg_tool_calls_all = sum(m.avg_tool_calls_all for m in model_metrics_list) / len(model_metrics_list)
        avg_tool_calls_successful = sum(m.avg_tool_calls_successful for m in model_metrics_list) / len(model_metrics_list)
        avg_exec_time_all = sum(m.avg_exec_time_all for m in model_metrics_list) / len(model_metrics_list)
        avg_exec_time_successful = sum(m.avg_exec_time_successful for m in model_metrics_list) / len(model_metrics_list)
        
        averaged_metrics[model] = ModelMetrics(
            provider=provider,
            model=model_name,
            success_rate=success_rate,
            avg_correctness_score_all=avg_correctness_score_all,
            avg_correctness_score_successful=avg_correctness_score_successful,
            total_runs=total_runs,
            successful_runs=successful_runs,
            avg_tokens_all=avg_tokens_all,
            avg_tokens_successful=avg_tokens_successful,
            avg_tool_calls_all=avg_tool_calls_all,
            avg_tool_calls_successful=avg_tool_calls_successful,
            avg_exec_time_all=avg_exec_time_all,
            avg_exec_time_successful=avg_exec_time_successful
        )
    
    return averaged_metrics

def generate_leaderboard(input_dir: str, output_path: str) -> None:
    """Generate a leaderboard from evaluation results.
    
    Args:
        input_dir: Directory containing evaluation CSV files
        output_path: Path to save the leaderboard CSV
        
    Raises:
        LeaderboardError: If there are errors processing the results
    """
    try:
        # Load all evaluation results
        dfs = load_evaluation_results(input_dir)
        
        # Calculate metrics for each task separately
        task_metrics_list = []
        for df in dfs:
            task_metrics = calculate_model_metrics(df)
            task_metrics_list.append(task_metrics)
        
        # Average metrics across all tasks
        model_metrics = average_metrics_across_tasks(task_metrics_list)
        
        # Convert metrics to DataFrame for sorting
        leaderboard_data = []
        for metrics in model_metrics.values():
            leaderboard_data.append({
                'provider': metrics.provider,
                'model': metrics.model,
                'success_rate': round(metrics.success_rate * 100, 1),
                'avg_correctness_score_all': round(metrics.avg_correctness_score_all, 2),
                'avg_correctness_score_successful': round(metrics.avg_correctness_score_successful, 2),
                'successful_runs': metrics.successful_runs,
                'total_runs': metrics.total_runs,
                'avg_tokens_all': round(metrics.avg_tokens_all),
                'avg_tokens_successful': round(metrics.avg_tokens_successful),
                'avg_tool_calls_all': round(metrics.avg_tool_calls_all, 1),
                'avg_tool_calls_successful': round(metrics.avg_tool_calls_successful, 1),
                'avg_exec_time_all': round(metrics.avg_exec_time_all, 1),
                'avg_exec_time_successful': round(metrics.avg_exec_time_successful, 1)
            })
        
        leaderboard_df = pd.DataFrame(leaderboard_data)
        
        # Sort by success rate and average correctness
        leaderboard_df = leaderboard_df.sort_values(
            by=['success_rate', 'avg_correctness_score_all'],
            ascending=[False, False]
        )
        
        # Save leaderboard
        leaderboard_df.to_csv(output_path, index=False)
        print(f"Leaderboard generated successfully: {output_path}")
        
        # Print top 5 models
        print("\nTop 5 Models:")
        print(leaderboard_df.head().to_string(index=False))
        
    except Exception as e:
        raise LeaderboardError(f"Failed to generate leaderboard: {str(e)}")

def main() -> None:
    """Main entry point for the leaderboard generation script."""
    import argparse
    
    parser = argparse.ArgumentParser(description="Generate a leaderboard from evaluation results")
    parser.add_argument("--input-dir", required=True, help="Directory containing evaluation CSV files")
    parser.add_argument("--output", required=True, help="Path to save the leaderboard CSV")
    
    args = parser.parse_args()
    
    try:
        generate_leaderboard(args.input_dir, args.output)
    except LeaderboardError as e:
        print(f"Error: {str(e)}")
        sys.exit(1)
    except Exception as e:
        print(f"Unexpected error: {str(e)}")
        sys.exit(1)

if __name__ == "__main__":
    main()