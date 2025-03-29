#!/usr/bin/env python3
"""
Script to analyze squirrel_census evaluation results from benchmark directories.

This script finds all benchmark-[provider]-[model] directories, extracts metrics
from squirrel_census eval_result.json files, and outputs a CSV summary.

The analysis includes:
- Write tool usage verification
- Implementation validity check
- Standard metrics (tokens, tool calls, execution time)

Example usage:
    python analyze_squirrel_census.py --benchmarks-dir ./benchmarks --output-dir squirrel-census-analysis.csv
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

class SquirrelCensusAnalyzer(AnalyzeProtocol):
    """Analyzer for squirrel census evaluation results."""
    
    def analyze_eval(self, file_path: str) -> Dict[str, Any]:
        """Load and analyze a single eval_result.json file.
        
        Args:
            file_path: Path to the eval_result.json file
            
        Returns:
            Dict[str, Any]: Dictionary containing analysis results including:
                - correct_results (bool): Whether all criteria were met
                - correctness_score (float): Normalized score (0-1)
                - wrote_script (bool): Whether script was written
                - ran_script (bool): Whether script was run
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
            wrote_script = extract_metric_value(metrics, "wrote_script")
            ran_script = extract_metric_value(metrics, "ran_script")
            correct_results = extract_metric_value(metrics, "correct_results")
            
            # Get standard metrics (tokens, tool calls, execution time)
            standard_metrics = extract_standard_metrics(metrics)
            
            # Calculate correctness score (sum of three boolean metrics)
            correctness_score = (wrote_script or False) + (ran_script or False) + (correct_results or False)
            
            # Determine if run was successful (correctness_score of 3 means all criteria were met)
            is_successful = correctness_score == 3
            
            return {
                "correct_results": is_successful,
                "correctness_score": correctness_score/3,
                "wrote_script": wrote_script or False,
                "ran_script": ran_script or False,
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
        """Calculate additional statistics specific to squirrel census.
        
        Args:
            runs: List of run results from analyze_eval
            
        Returns:
            Dict[str, Any]: Dictionary containing:
                - success_rate: Rate of successful runs
                - avg_correctness_score: Average correctness score
                - wrote_script_rate: Rate of script writing
                - ran_script_rate: Rate of script execution
        """
        if not runs:
            return {
                "success_rate": 0,
                "avg_correctness_score": 0,
                "wrote_script_rate": 0,
                "ran_script_rate": 0
            }
        
        # Count runs by individual metrics
        wrote_script_count = sum(1 for run in runs if run.get("wrote_script") is True)
        ran_script_count = sum(1 for run in runs if run.get("ran_script") is True)
        success_count = sum(1 for run in runs if run.get("correct_results") is True)
        correctness_sum = sum(run.get("correctness_score", 0) for run in runs)
        
        return {
            "success_rate": success_count / len(runs),
            "avg_correctness_score": correctness_sum / len(runs),
            "wrote_script_rate": wrote_script_count / len(runs),
            "ran_script_rate": ran_script_count / len(runs),
        }

def main() -> None:
    """Main entry point for the squirrel census analysis script."""
    parser = create_argparser("squirrel_census", "squirrel-census-analysis.csv")
    args = parser.parse_args()
    
    analyzer = SquirrelCensusAnalyzer()
    analyze_benchmark_results(
        benchmarks_dir=args.benchmarks_dir,
        eval_name="squirrel_census",
        eval_processor=analyzer.analyze_eval,
        output_csv=args.output_dir,
        metric_aggregator=analyzer.aggregate_metrics
    )

if __name__ == "__main__":
    main()