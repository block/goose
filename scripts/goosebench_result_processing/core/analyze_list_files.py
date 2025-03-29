#!/usr/bin/env python3
"""
Script to analyze list_files evaluation results from benchmark directories.

This script finds all benchmark-[provider]-[model] directories, extracts metrics
from list_files eval_result.json files, and outputs a CSV summary.

The analysis includes:
- File listing operation verification
- Implementation validity check
- Standard metrics (tokens, tool calls, execution time)

Example usage:
    python analyze_list_files.py --benchmarks-dir ./benchmarks --output-dir list-files-analysis.csv
"""

import json
from typing import Dict, Any, List, Optional
from scripts.goosebench_result_processing.benchmark_utils import (
    extract_metric_value,
    extract_standard_metrics,
    analyze_benchmark_results,
    create_argparser,
    load_output_file,
    evaluate_with_openai
)
from scripts.goosebench_result_processing.analyze_interface import AnalyzeProtocol

class ListFilesAnalyzer(AnalyzeProtocol):
    """Analyzer for List Files evaluation results."""
    
    def analyze_eval(self, file_path: str) -> Dict[str, Any]:
        """Load and analyze a single eval_result.json file.
        
        Args:
            file_path: Path to the eval_result.json file
            
        Returns:
            Dict[str, Any]: Dictionary containing analysis results including:
                - correct_results (bool): Whether all criteria were met
                - correctness_score (float): Normalized score (0-1)
                - used_shell_command (bool): Whether shell command was used correctly
                - Standard metrics (tokens, tool calls, execution time)
                
        Raises:
            FileNotFoundError: If the eval_result.json file doesn't exist
            json.JSONDecodeError: If the file contains invalid JSON
        """
        try:
            with open(file_path, 'r', encoding='utf-8') as f:
                data = json.load(f)
            
            metrics = data.get("metrics", [])
            
            # Extract key metrics
            used_shell_command = extract_metric_value(metrics, "Using the shell command tool")
            
            # Get standard metrics (tokens, tool calls, execution time)
            standard_metrics = extract_standard_metrics(metrics)
            
            return {
                "correct_results": used_shell_command,
                "correctness_score": float(used_shell_command),
                "used_shell_command": used_shell_command or False,
                **standard_metrics
            }
        except FileNotFoundError:
            print(f"File not found: {file_path}")
            return {}
        except json.JSONDecodeError as e:
            print(f"Invalid JSON in {file_path}: {str(e)}")
            return {}
        except Exception as e:
            print(f"Unexpected error loading {file_path}: {str(e)}")
            return {}
    
    def aggregate_metrics(self, runs: List[Dict[str, Any]]) -> Dict[str, Any]:
        """Calculate additional statistics specific to List Files.
        
        Args:
            runs: List of run results from analyze_eval
            
        Returns:
            Dict[str, Any]: Dictionary containing:
                - success_rate: Rate of successful runs
                - avg_correctness_score: Average correctness score
                - shell_command_usage_rate: Rate of correct shell command usage
        """
        if not runs:
            return {
                "success_rate": 0,
                "avg_correctness_score": 0,
                "shell_command_usage_rate": 0
            }
        
        # Count runs by individual metrics
        shell_command_count = sum(1 for run in runs if run.get("used_shell_command") is True)
        success_count = sum(1 for run in runs if run.get("correct_results") is True)
        correctness_sum = sum(run.get("correctness_score", 0) for run in runs)
        
        return {
            "success_rate": success_count / len(runs),
            "avg_correctness_score": correctness_sum / len(runs),
            "shell_command_usage_rate": shell_command_count / len(runs),
        }

def main() -> None:
    """Main entry point for the list files analysis script."""
    parser = create_argparser("list_files", "list-files-analysis.csv")
    args = parser.parse_args()
    
    analyzer = ListFilesAnalyzer()
    analyze_benchmark_results(
        benchmarks_dir=args.benchmarks_dir,
        eval_name="list_files",
        eval_processor=analyzer.analyze_eval,
        output_csv=args.output_dir,
        metric_aggregator=analyzer.aggregate_metrics
    )

if __name__ == "__main__":
    main() 