#!/usr/bin/env python3
"""
Utility module for benchmark analysis scripts.

This module contains common functions used across various benchmark analysis scripts
to extract metrics, process results, and generate standardized reports.
"""

import os
import json
import glob
import csv
import re
from collections import defaultdict, Counter
from typing import Dict, List, Any, Tuple, Optional, Callable
from openai import OpenAI

def find_benchmark_directories(benchmarks_dir: str) -> List[str]:
    """Find all benchmark directories matching the pattern benchmark-provider-model."""
    if not os.path.isdir(benchmarks_dir):
        print(f"Error: Benchmarks directory {benchmarks_dir} does not exist")
        return []
    
    # If the benchmarks_dir itself is a benchmark directory, return it
    if os.path.basename(benchmarks_dir).startswith('benchmark-'):
        return [benchmarks_dir]
    
    # Otherwise, find all directories matching the pattern benchmark-{provider}-{model}
    benchmark_dirs = []
    for item in os.listdir(benchmarks_dir):
        if item.startswith('benchmark-') and os.path.isdir(os.path.join(benchmarks_dir, item)):
            benchmark_dirs.append(os.path.join(benchmarks_dir, item))
    
    return benchmark_dirs

def find_eval_results(benchmark_dir: str, eval_name: str) -> List[str]:
    """Find all eval_result.json files for the specified evaluation."""
    
    # Get all subdirectories in the benchmark directory
    all_subdirs = [d for d in os.listdir(benchmark_dir) if os.path.isdir(os.path.join(benchmark_dir, d))]
    
    # Filter for timestamp-like directories (allow more flexible formats)
    timestamp_dirs = []
    for d in all_subdirs:
        # Check if directory name contains date-like elements
        if (d.startswith("20") and ("-" in d or ":" in d)) or re.match(r"\d{4}-\d{2}-\d{2}", d):
            timestamp_dirs.append(os.path.join(benchmark_dir, d))
    
    results = []
    
    # Check each timestamp directory for the evaluation
    for ts_dir in timestamp_dirs:
        # Walk through all subdirectories
        for root, _, files in os.walk(ts_dir):
            # Look for eval_result.json in any directory matching the eval_name
            if os.path.basename(root) == eval_name and "eval_result.json" in files:
                eval_path = os.path.join(root, "eval_result.json")
                results.append(eval_path)
    
    return results

def parse_provider_model(dir_path: str) -> Tuple[str, str]:
    """Extract provider and model from the benchmark directory name."""
    dir_name = os.path.basename(dir_path)
    match = re.match(r'benchmark-([^-]+)-(.+)', dir_name)
    
    if match:
        provider = match.group(1)
        model = match.group(2)
        return provider, model
    else:
        # Fallback if the format doesn't match expected pattern
        parts = dir_name.split('-', 2)
        if len(parts) >= 3:
            return parts[1], parts[2]
        else:
            return "unknown", "unknown"

def extract_metric_value(metrics: List, metric_name: str) -> Any:
    """Extract a specific metric value from metrics list."""
    for metric in metrics:
        if isinstance(metric, list) and len(metric) >= 2 and metric[0] == metric_name:
            # Handle different metric formats
            value = metric[1]
            if isinstance(value, dict):
                # Check for different value types
                if "Boolean" in value:
                    return value["Boolean"]
                elif "Integer" in value:
                    return value["Integer"]
                elif "Float" in value:
                    return value["Float"]
                elif "String" in value:
                    return value["String"]
                else:
                    return next(iter(value.values()), None)
            else:
                return value
    return None

def extract_standard_metrics(metrics: List) -> Dict[str, Any]:
    """Extract common metrics found in most eval results."""
    return {
        "total_tokens": extract_metric_value(metrics, "total_tokens") or 0,
        "total_tool_calls": extract_metric_value(metrics, "total_tool_calls") or 0,
        "prompt_execution_time_seconds": extract_metric_value(metrics, "prompt_execution_time_seconds") or 0
    }

def load_output_file(result_dir: str, filename: str) -> Optional[str]:
    """Load content from an output file if it exists."""
    output_file = os.path.join(result_dir, filename)
    if os.path.exists(output_file):
        try:
            with open(output_file, 'r', encoding='utf-8') as f:
                return f.read()
        except Exception as e:
            print(f"Error reading {output_file}: {str(e)}")
    else:
        print(f"Warning: {filename} not found in {result_dir}")
    return None

def calculate_run_statistics(runs: List[Dict[str, Any]], success_field: str = "correct_results") -> Dict[str, Any]:
    """Calculate common statistics for a set of runs."""
    if not runs:
        return {
            "best_run": False,
            "n_successful_runs": 0,
            "total_runs": 0,
            "success_rate": 0,
            "avg_tokens_all": 0,
            "avg_tool_calls_all": 0,
            "avg_exec_time_all": 0,
            "avg_tokens_successful": 0,
            "avg_tool_calls_successful": 0,
            "avg_exec_time_successful": 0
        }
    
    # Count successful runs
    successful_runs = [run for run in runs if run.get(success_field) is True]
    n_successful_runs = len(successful_runs)
    best_run = n_successful_runs > 0
    
    # Calculate averages for successful runs
    if successful_runs:
        avg_tokens_successful = sum(run.get("total_tokens", 0) for run in successful_runs) / n_successful_runs
        avg_tool_calls_successful = sum(run.get("total_tool_calls", 0) for run in successful_runs) / n_successful_runs
        avg_exec_time_successful = sum(run.get("prompt_execution_time_seconds", 0) for run in successful_runs) / n_successful_runs
    else:
        avg_tokens_successful = 0
        avg_tool_calls_successful = 0
        avg_exec_time_successful = 0
        
    # Calculate averages for all runs
    avg_tokens_all = sum(run.get("total_tokens", 0) for run in runs) / len(runs)
    avg_tool_calls_all = sum(run.get("total_tool_calls", 0) for run in runs) / len(runs)
    avg_exec_time_all = sum(run.get("prompt_execution_time_seconds", 0) for run in runs) / len(runs)
    
    return {
        "best_run": best_run,
        "n_successful_runs": n_successful_runs,
        "total_runs": len(runs),
        "success_rate": n_successful_runs / len(runs),
        "avg_tokens_successful": avg_tokens_successful,
        "avg_tool_calls_successful": avg_tool_calls_successful,
        "avg_exec_time_successful": avg_exec_time_successful,
        "avg_tokens_all": avg_tokens_all,
        "avg_tool_calls_all": avg_tool_calls_all,
        "avg_exec_time_all": avg_exec_time_all
    }

def analyze_benchmark_results(
    benchmarks_dir: str, 
    eval_name: str,
    eval_processor: Callable[[str], Dict[str, Any]],
    output_csv: str,
    metric_aggregator: Callable[[List[Dict[str, Any]]], Dict[str, Any]] = None
) -> None:
    """Generic function to analyze benchmark results for a specified evaluation type."""
    benchmark_dirs = find_benchmark_directories(benchmarks_dir)
    results_by_provider_model = defaultdict(list)
    
    print(f"\n=== Analyzing {eval_name} ===")
    print(f"Found {len(benchmark_dirs)} benchmark directories")
    
    # Count total result files found across all providers
    total_result_files = 0
    
    for benchmark_dir in benchmark_dirs:
        provider, model = parse_provider_model(benchmark_dir)
        key = (provider, model)
        
        print(f"\nChecking {provider}-{model}:")
        
        # Check the timestamp directories
        timestamp_dirs = [d for d in glob.glob(os.path.join(benchmark_dir, "*")) if os.path.isdir(d)]
        print(f"  Found {len(timestamp_dirs)} timestamp directories")
        for i, ts_dir in enumerate(timestamp_dirs[:3]):  # Show first 3
            print(f"    - {os.path.basename(ts_dir)}")
        if len(timestamp_dirs) > 3:
            print(f"    - ... and {len(timestamp_dirs) - 3} more")
            
        # Find results for this evaluation
        eval_results = find_eval_results(benchmark_dir, eval_name)
        total_result_files += len(eval_results)
        
        if eval_results:
            print(f"\n  Processing {len(eval_results)} {eval_name} results for {provider}-{model}")
            
            for result_file in eval_results:
                print(f"    Processing: {result_file}")
                try:
                    result_data = eval_processor(result_file)
                    if result_data:
                        results_by_provider_model[key].append(result_data)
                        print(f"      ✅ Success")
                    else:
                        print(f"      ❌ Failed to process result file")
                except Exception as e:
                    if "OPENAI_API_KEY" in str(e):
                        raise  # Re-raise OpenAI API key errors
                    print(f"      ❌ Error processing file: {str(e)}")
                    continue
    
    # Generate CSV data
    csv_rows = []
    
    print("\n=== Summary by Provider/Model ===")
    for (provider, model), runs in results_by_provider_model.items():
        print(f"{provider}-{model}: {len(runs)} valid runs processed")
        
        # Get standard statistics
        stats = calculate_run_statistics(runs)
        
        # Create row with provider and model
        row = {
            "provider": provider,
            "model": model,
            **stats
        }
        
        # Allow custom stats preprocessing if provided
        if metric_aggregator:
            custom_metrics = metric_aggregator(runs)
            row.update(custom_metrics)
            
        csv_rows.append(row)
    
    # Sort by provider, then model
    csv_rows.sort(key=lambda x: (x["provider"], x["model"]))
    
    # Write to CSV
    if csv_rows:
        with open(output_csv, 'w', newline='', encoding='utf-8') as csvfile:
            fieldnames = list(csv_rows[0].keys())
            writer = csv.DictWriter(csvfile, fieldnames=fieldnames)
            writer.writeheader()
            writer.writerows(csv_rows)
        
        print(f"\n{eval_name.replace('_', ' ').title()} analysis complete. Results saved to {output_csv}")
        print(f"Total result files found: {total_result_files}")
        print(f"Total valid runs processed: {sum(len(runs) for runs in results_by_provider_model.values())}")
    else:
        print(f"\nNo {eval_name} evaluation results found")

def create_argparser(eval_name: str, default_output: str) -> Any:
    """Create a standard argument parser for benchmark analysis scripts."""
    import argparse
    
    parser = argparse.ArgumentParser(description=f'Analyze {eval_name} benchmark results')
    parser.add_argument('--benchmarks-dir', default='.', 
                        help='Directory containing benchmark-provider-model directories')
    parser.add_argument('--output-dir', default=default_output,
                        help='Output directory for analysis results')
    
    return parser

# OpenAI evaluation utility (used by both restaurant_research and blog_summary)
def evaluate_with_openai(prompt: str, text: str, rubric_max_score: int = 2) -> float:
    """Evaluate response using OpenAI's API.
    
    Args:
        prompt: System prompt for evaluation
        text: Text to evaluate
        rubric_max_score: Maximum score for the rubric (default: 2.0)
        
    Returns:
        float: Evaluation score (0 to rubric_max_score)
        
    Raises:
        ValueError: If OPENAI_API_KEY environment variable is not set
    """
    print("Starting OpenAI evaluation...")
    api_key = os.getenv("OPENAI_API_KEY")
    if not api_key:
        print("No OpenAI API key found!")
        raise ValueError("OPENAI_API_KEY environment variable is not set, but is needed to run this evaluation.")
        
    try:
        client = OpenAI(api_key=api_key)
        
        # Append output instructions to system prompt
        output_instructions = f"""
Output Instructions:
Return your evaluation as a JSON object in the following format:
{{
    "reasoning": "Your brief reasoning for the score",
    "score": <integer between 0 and {rubric_max_score}>
}}

Do not include any markdown formatting or additional text. Return only the JSON object."""
        
        input_prompt = f"{prompt} {output_instructions}\Response to evaluate: {text}"
        
        # Run the chat completion 3 times and collect scores
        scores = []
        for _ in range(3):
            response = client.chat.completions.create(
                model="gpt-4",
                messages=[
                    {"role": "user", "content": input_prompt}
                ],
                temperature=0.9
            )
            
            # Extract and parse JSON from response
            response_text = response.choices[0].message.content.strip()
            try:
                evaluation = json.loads(response_text)
                score = float(evaluation.get("score", 0.0))
                score = max(0.0, min(score, rubric_max_score))
                scores.append(score)
                print(f"Run score: {score}")
            except (json.JSONDecodeError, ValueError) as e:
                print(f"Error parsing OpenAI response as JSON: {str(e)}")
                print(f"Response text: {response_text}")
                raise ValueError(f"Failed to parse OpenAI evaluation response: {str(e)}")
        
        # Count occurrences of each score
        score_counts = Counter(scores)
        
        # If there's no single most common score (all scores are different), run one more time
        if len(scores) == 3 and max(score_counts.values()) == 1:
            print("No majority score found. Running tie-breaker...")
            response = client.chat.completions.create(
                model="gpt-4",
                messages=[
                    {"role": "user", "content": input_prompt}
                ],
                temperature=0.9
            )
            
            response_text = response.choices[0].message.content.strip()
            try:
                evaluation = json.loads(response_text)
                score = float(evaluation.get("score", 0.0))
                score = max(0.0, min(score, rubric_max_score))
                scores.append(score)
                print(f"Tie-breaker score: {score}")
                score_counts = Counter(scores)
            except (json.JSONDecodeError, ValueError) as e:
                print(f"Error parsing tie-breaker response as JSON: {str(e)}")
                print(f"Response text: {response_text}")
                raise ValueError(f"Failed to parse tie-breaker response: {str(e)}")
        
        # Get the most common score
        most_common_score = score_counts.most_common(1)[0][0]
        print(f"Most common score: {most_common_score} (occurred {score_counts[most_common_score]} times)")
        return most_common_score
            
    except Exception as e:
        if "OPENAI_API_KEY" in str(e):
            raise  # Re-raise API key errors
        print(f"Error evaluating with OpenAI: {str(e)}")
        raise ValueError(f"OpenAI evaluation failed: {str(e)}")
