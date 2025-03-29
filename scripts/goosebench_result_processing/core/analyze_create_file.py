#!/usr/bin/env python3
"""
Script to analyze create_file evaluation results from benchmark directories.

This script finds all benchmark-[provider]-[model] directories, extracts metrics
from create_file eval_result.json files, and outputs a CSV summary.

The analysis includes:
- File creation operation verification
- Implementation validity check
- Standard metrics (tokens, tool calls, execution time)

Example usage:
    python analyze_create_file.py --benchmarks-dir ./benchmarks --output-dir create-file-analysis.csv
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

class CreateFileAnalyzer(AnalyzeProtocol):
    """Analyzer for Create File evaluation results."""
    
    def analyze_eval(self, file_path: str) -> Dict[str, Any]:
        """Load and analyze a single eval_result.json file.
        
        Args:
            file_path: Path to the eval_result.json file
            
        Returns:
            Dict[str, Any]: Dictionary containing analysis results including:
                - correct_results (bool): Whether all criteria were met
                - correctness_score (float): Normalized score (0-1)
                - created_file (bool): Whether file was successfully created
                - read_file (bool): Whether file was successfully read
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
            created_file = extract_metric_value(metrics, "Create file")
            read_file = extract_metric_value(metrics, "Read file")
            complete_operation = extract_metric_value(metrics, "Complete create and read")
            
            # Get standard metrics (tokens, tool calls, execution time)
            standard_metrics = extract_standard_metrics(metrics)
            
         
            return {
                "correct_results": complete_operation or False,
                "correctness_score": complete_operation or False,
                "created_file": created_file or False,
                "read_file": read_file or False,
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
        """Calculate additional statistics specific to Create File.
        
        Args:
            runs: List of run results from analyze_eval
            
        Returns:
            Dict[str, Any]: Dictionary containing:
                - success_rate: Rate of successful runs
                - avg_correctness_score: Average correctness score
                - create_rate: Rate of successful file creation
                - read_rate: Rate of successful file reading
        """
        if not runs:
            return {
                "success_rate": 0,
                "avg_correctness_score": 0,
                "create_rate": 0,
                "read_rate": 0,
            }
        
        # Count runs by individual metrics
        create_count = sum(1 for run in runs if run.get("created_file") is True)
        read_count = sum(1 for run in runs if run.get("read_file") is True)
        success_count = sum(1 for run in runs if run.get("correct_results") is True)
        correctness_sum = sum(run.get("correctness_score", 0) for run in runs)
        
        return {
            "success_rate": success_count / len(runs),
            "avg_correctness_score": correctness_sum / len(runs),
            "create_rate": create_count / len(runs),
            "read_rate": read_count / len(runs),
        }

def main() -> None:
    """Main entry point for the create file analysis script."""
    parser = create_argparser("create_file", "create-file-analysis.csv")
    args = parser.parse_args()
    
    analyzer = CreateFileAnalyzer()
    analyze_benchmark_results(
        benchmarks_dir=args.benchmarks_dir,
        eval_name="create_file",
        eval_processor=analyzer.analyze_eval,
        output_csv=args.output_dir,
        metric_aggregator=analyzer.aggregate_metrics
    )

if __name__ == "__main__":
    main() 