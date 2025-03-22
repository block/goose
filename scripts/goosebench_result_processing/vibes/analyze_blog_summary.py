#!/usr/bin/env python3
"""
Script to analyze blog_summary evaluation results from benchmark directories.

This script finds all benchmark-[provider]-[model] directories, extracts metrics
from blog_summary eval_result.json files, and outputs a CSV summary.

The analysis includes:
- Fetch tool usage verification
- Markdown format validation
- OpenAI-based content evaluation
- Standard metrics (tokens, tool calls, execution time)

Example usage:
    python analyze_blog_summary.py --benchmarks-dir ./benchmarks --output-dir blog-summary-analysis.csv
"""

import os
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

# System prompt for OpenAI evaluation of blog summaries
BLOG_SUMMARY_PROMPT = """You are evaluating a response to a summarization task and will give a score of 0, 1, or 2. The instructions were:

'What are the top 5 most counterintuitive insights from this blog post? https://huyenchip.com/2025/01/07/agents.html'

Does the response below appropriately answer the query (ignore formatting)?
0 = does not provide any insights at all
1 = provides some insights, but not all 5
2 = provides all 5 insights"""

class BlogSummaryAnalyzer(AnalyzeProtocol):
    """Analyzer for blog summary evaluation results."""
    
    def evaluate_blog_summary(self, dir_path: str) -> float:
        """Evaluate blog summary using OpenAI.
        
        Args:
            dir_path: Directory containing the blog_summary_output.txt file
            
        Returns:
            float: OpenAI evaluation score (0-2)
            
        Raises:
            ValueError: If OpenAI API key is not set or if output file is missing
        """
        print(f"\nAttempting to evaluate blog summary in: {dir_path}")
        response_text = load_output_file(dir_path, "blog_summary_output.txt")
        if not response_text:
            print(f"No blog_summary_output.txt found in {dir_path}")
            raise ValueError(f"Missing output file: {dir_path}/blog_summary_output.txt")
            
        print(f"Found output file, length: {len(response_text)} chars")
        print("Calling OpenAI evaluation...")
        return evaluate_with_openai(BLOG_SUMMARY_PROMPT, response_text)
    
    def analyze_eval(self, file_path: str) -> Dict[str, Any]:
        """Load and analyze a single eval_result.json file.
        
        Args:
            file_path: Path to the eval_result.json file
            
        Returns:
            Dict[str, Any]: Dictionary containing analysis results including:
                - correct_results (bool): Whether all criteria were met
                - correctness_score (int): Sum of individual criteria scores
                - used_fetch_tool (bool): Whether fetch tool was used
                - valid_markdown_format (bool): Whether markdown format is valid
                - openai_evaluation (float): OpenAI evaluation score
                - Standard metrics (tokens, tool calls, execution time)
                
        Raises:
            FileNotFoundError: If the eval_result.json file doesn't exist
            json.JSONDecodeError: If the file contains invalid JSON
            ValueError: If OpenAI API key is not set or if required metrics are missing
        """
        try:
            with open(file_path, 'r', encoding='utf-8') as f:
                data = json.load(f)
            
            metrics = data.get("metrics", [])
            if not metrics:
                raise ValueError(f"No metrics found in {file_path}")
            
            # Extract key metrics from eval_result.json
            used_fetch_tool = extract_metric_value(metrics, "used_fetch_tool")
            valid_markdown_format = extract_metric_value(metrics, "valid_markdown_format")
            
            if used_fetch_tool is None or valid_markdown_format is None:
                raise ValueError(f"Missing required metrics in {file_path}")
            
            # Get standard metrics (tokens, tool calls, execution time)
            standard_metrics = extract_standard_metrics(metrics)
            
            # Get the directory containing the eval_result.json file
            dir_path = os.path.dirname(file_path)
            
            # Always run OpenAI evaluation
            print("Running OpenAI evaluation...")
            openai_evaluation = self.evaluate_blog_summary(dir_path)
            
            # Calculate correctness score (sum of two boolean metrics plus OpenAI score)
            correctness_score = (used_fetch_tool or False) + (valid_markdown_format or False) + openai_evaluation
            
            # Determine if run was successful (correctness_score of 4 means all criteria were met at the highest level)
            correct_results = correctness_score == 4
            
            return {
                "correct_results": correct_results,
                "correctness_score": correctness_score/4,
                "used_fetch_tool": used_fetch_tool or False,
                "valid_markdown_format": valid_markdown_format or False,
                "openai_evaluation": openai_evaluation,
                **standard_metrics
            }
                
        except FileNotFoundError:
            raise FileNotFoundError(f"Eval result file not found: {file_path}")
        except json.JSONDecodeError as e:
            raise json.JSONDecodeError(f"Invalid JSON in {file_path}", e.doc, e.pos)
        except ValueError as e:
            if "OPENAI_API_KEY" in str(e):
                raise  # Re-raise the OpenAI API key error
            raise ValueError(f"Error processing {file_path}: {str(e)}")
        except Exception as e:
            raise RuntimeError(f"Unexpected error processing {file_path}: {str(e)}")
    
    def aggregate_metrics(self, runs: List[Dict[str, Any]]) -> Dict[str, Any]:
        """Calculate additional statistics specific to blog summary.
        
        Args:
            runs: List of run results from analyze_eval
            
        Returns:
            Dict[str, Any]: Dictionary containing:
                - avg_success_rate: Average rate of successful runs
                - fetch_success_rate: Rate of fetch tool usage
                - markdown_success_rate: Rate of valid markdown format
                - avg_openai_score: Average OpenAI evaluation score
        """
        if not runs:
            return {
                "avg_success_rate": 0,
                "avg_correctness_score": 0,
                "fetch_success_rate": 0,
                "markdown_success_rate": 0,
                "avg_openai_score": 0
            }
        
        return {
            "avg_success_rate": sum(run.get("avg_success_rate", 0) for run in runs) / len(runs),
            "avg_correctness_score": sum(run.get("correctness_score", 0) for run in runs) / len(runs),
            "fetch_success_rate": sum(1 for run in runs if run.get("used_fetch_tool") is True) / len(runs),
            "markdown_success_rate": sum(1 for run in runs if run.get("valid_markdown_format") is True) / len(runs),
            "avg_openai_score": sum(run.get("openai_evaluation", 0) for run in runs) / len(runs)
        }

def main() -> None:
    """Main entry point for the blog summary analysis script."""
    parser = create_argparser("blog_summary", "blog-summary-analysis.csv")
    args = parser.parse_args()
    
    analyzer = BlogSummaryAnalyzer()
    analyze_benchmark_results(
        benchmarks_dir=args.benchmarks_dir,
        eval_name="blog_summary",
        eval_processor=analyzer.analyze_eval,
        output_csv=args.output_dir,
        metric_aggregator=analyzer.aggregate_metrics
    )

if __name__ == "__main__":
    main()