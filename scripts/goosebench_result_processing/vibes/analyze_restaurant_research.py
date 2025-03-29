#!/usr/bin/env python3
"""
Script to analyze restaurant_research evaluation results from benchmark directories.

This script finds all benchmark-[provider]-[model] directories, extracts metrics
from restaurant_research eval_result.json files, and outputs a CSV summary.

The analysis includes:
- Write tool usage verification
- Implementation validity check
- OpenAI-based content evaluation
- Standard metrics (tokens, tool calls, execution time)

Example usage:
    python analyze_restaurant_research.py --benchmarks-dir ./benchmarks --output-dir restaurant-research-analysis.csv
"""

import json
import os
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

# System prompt for OpenAI evaluation of restaurant research
RESTAURANT_RESEARCH_PROMPT = """You are evaluating an AI assistant's response to a restaurant research task. The instructions were:

'Search the internet for and provide a current, detailed list of the best Sichuanese restaurants specifically in the East Village neighborhood of NYC. Format your response in Markdown using bullet points (either - or *) for each restaurant. For each restaurant include:
- Restaurant name and what they're known for
- Signature dishes
- Atmosphere/setting
- Any relevant details about reservations or dining experience
- What distinguishes them from others

Present the information in order of significance or quality. Focus specifically on Sichuanese establishments, not general Chinese restaurants. If you encounter a page you cannot access, try another one. Do not ask me for confirmation just conduct the searches yourself until you find the needed information. Remember to use your tools if applicable.'

Give a score of 0, 1, or 2:
0 = does not provide any restaurants at all
1 = provides some restaurants, but not all are Sichuanese or in the East Village NYC
2 = provides all Sichuanese restaurants in the East Village, probably including Mala project and Szechuan Mountain House, or Uluh. Use your memory/knowledge of the East Village NYC restaurants to double check non-East Village restaurants."""

class RestaurantResearchAnalyzer(AnalyzeProtocol):
    """Analyzer for restaurant research evaluation results."""
    
    def evaluate_restaurant_research(self, dir_path: str) -> float:
        """Evaluate restaurant research using OpenAI.
        
        Args:
            dir_path: Directory containing the restaurant_research_output.txt file
            
        Returns:
            float: OpenAI evaluation score (0-2)
            
        Raises:
            ValueError: If OpenAI API key is not set
        """
        response_text = load_output_file(dir_path, "restaurant_research_output.txt")
        if not response_text:
            return 0.0
            
        print(f"Evaluating output from {dir_path}/restaurant_research_output.txt")
        return evaluate_with_openai(response_text, RESTAURANT_RESEARCH_PROMPT)
    
    def analyze_eval(self, file_path: str) -> Dict[str, Any]:
        """Load and analyze a single eval_result.json file.
        
        Args:
            file_path: Path to the eval_result.json file
            
        Returns:
            Dict[str, Any]: Dictionary containing analysis results including:
                - correct_results (bool): Whether all criteria were met
                - correctness_score (float): Normalized score (0-1)
                - valid_markdown_format (bool): Whether markdown format is valid
                - bullet_point_count (int): Number of bullet points
                - used_fetch_tool (bool): Whether fetch tool was used
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
            
            # Extract key metrics
            valid_markdown_format = extract_metric_value(metrics, "valid_markdown_format")
            bullet_point_count = extract_metric_value(metrics, "bullet_point_count")
            used_fetch_tool = extract_metric_value(metrics, "used_fetch_tool")
            
            if valid_markdown_format is None or bullet_point_count is None or used_fetch_tool is None:
                raise ValueError(f"Missing required metrics in {file_path}")
            
            # Get standard metrics (tokens, tool calls, execution time)
            standard_metrics = extract_standard_metrics(metrics)
            
            # Get the directory containing the eval_result.json file
            dir_path = os.path.dirname(file_path)
            
            # Evaluate with OpenAI
            try:
                openai_evaluation = self.evaluate_restaurant_research(dir_path)
            except ValueError as e:
                if "OPENAI_API_KEY" in str(e):
                    raise  # Re-raise the OpenAI API key error
                return {}  # Return empty dict for other evaluation errors
            
            # Calculate correctness score (sum of boolean metrics plus OpenAI score)
            correctness_score = (valid_markdown_format or False) + (used_fetch_tool or False) + openai_evaluation
            
            # Determine if run was successful (correctness_score of 4 means all criteria were met at the highest level)
            is_successful = correctness_score == 4
            
            return {
                "correct_results": is_successful,
                "correctness_score": correctness_score/4,
                "valid_markdown_format": valid_markdown_format or False,
                "bullet_point_count": bullet_point_count or 0,
                "used_fetch_tool": used_fetch_tool or False,
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
        """Calculate additional statistics specific to restaurant research.
        
        Args:
            runs: List of run results from analyze_eval
            
        Returns:
            Dict[str, Any]: Dictionary containing:
                - success_rate: Rate of successful runs
                - avg_correctness_score: Average correctness score
                - markdown_success_rate: Rate of valid markdown format
                - fetch_success_rate: Rate of fetch tool usage
                - avg_bullet_points: Average number of bullet points
                - avg_openai_score: Average OpenAI evaluation score
        """
        if not runs:
            return {
                "success_rate": 0,
                "avg_correctness_score": 0,
                "markdown_success_rate": 0,
                "fetch_success_rate": 0,
                "avg_bullet_points": 0,
                "avg_openai_score": 0
            }
        
        # Count runs by individual metrics
        markdown_success_count = sum(1 for run in runs if run.get("valid_markdown_format") is True)
        fetch_success_count = sum(1 for run in runs if run.get("used_fetch_tool") is True)
        success_count = sum(1 for run in runs if run.get("correct_results") is True)
        correctness_sum = sum(run.get("correctness_score", 0) for run in runs)
        bullet_points_sum = sum(run.get("bullet_point_count", 0) for run in runs)
        openai_sum = sum(run.get("openai_evaluation", 0) for run in runs)
        
        return {
            "success_rate": success_count / len(runs),
            "avg_correctness_score": correctness_sum / len(runs),
            "markdown_success_rate": markdown_success_count / len(runs),
            "fetch_success_rate": fetch_success_count / len(runs),
            "avg_bullet_points": bullet_points_sum / len(runs),
            "avg_openai_score": openai_sum / len(runs)
        }

def main() -> None:
    """Main entry point for the restaurant research analysis script."""
    parser = create_argparser("restaurant_research", "restaurant-research-analysis.csv")
    args = parser.parse_args()
    
    analyzer = RestaurantResearchAnalyzer()
    analyze_benchmark_results(
        benchmarks_dir=args.benchmarks_dir,
        eval_name="restaurant_research",
        eval_processor=analyzer.analyze_eval,
        output_csv=args.output_dir,
        metric_aggregator=analyzer.aggregate_metrics
    )

if __name__ == "__main__":
    main()