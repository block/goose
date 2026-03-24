#!/usr/bin/env python3
"""
Calculate final score for vibes evaluations.
This script combines the LLM judge score with other metrics to produce a final score.
"""

import json
import sys
from pathlib import Path


def get_metric_value(metrics, metric_name):
    """Extract a metric value from the metrics array."""
    for metric in metrics:
        if metric[0] == metric_name:
            value = metric[1]
            if "Float" in value:
                return float(value["Float"])
            elif "Integer" in value:
                return float(value["Integer"])
            elif "Boolean" in value:
                return 1.0 if value["Boolean"] else 0.0
    return None


def calculate_score(eval_name, metrics):
    """Calculate the final score based on the evaluation type."""
    llm_judge_score = get_metric_value(metrics, "llm_judge_score")
    used_fetch_tool = get_metric_value(metrics, "used_fetch_tool")
    valid_markdown_format = get_metric_value(metrics, "valid_markdown_format")
    
    if llm_judge_score is None:
        raise ValueError("llm_judge_score not found in metrics")
    
    # Convert boolean metrics to 0/1 if needed
    used_fetch_tool = 1.0 if used_fetch_tool else 0.0
    valid_markdown_format = 1.0 if valid_markdown_format else 0.0
    
    if eval_name == "blog_summary":
        # max score is 4.0 as llm_judge_score is between [0,2] and used_fetch_tool/valid_markedown_format have values [0,1]
        score = (llm_judge_score + used_fetch_tool + valid_markdown_format) / 4.0
    elif eval_name == "restaurant_research":
        score = (llm_judge_score + valid_markdown_format + used_fetch_tool) / 4.0
    else:
        raise ValueError(f"Unknown evaluation type: {eval_name}")
    
    return score


def main():
    if len(sys.argv) != 2:
        print("Usage: calculate_final_score.py <eval_name>")
        sys.exit(1)
    
    eval_name = sys.argv[1]
    
    # Load eval results from current directory
    eval_results_path = Path("eval-results.json")
    if not eval_results_path.exists():
        print(f"Error: eval-results.json not found in current directory")
        sys.exit(1)
    
    with open(eval_results_path, 'r') as f:
        eval_results = json.load(f)
    
    try:
        # Calculate the final score
        score = calculate_score(eval_name, eval_results["metrics"])
        
        # Add the score metric
        eval_results["metrics"].append([
            "score",
            {"Float": score}
        ])
        
        # Save updated results
        with open(eval_results_path, 'w') as f:
            json.dump(eval_results, f, indent=2)
        
        print(f"Successfully added final score: {score}")
        
    except Exception as e:
        print(f"Error calculating final score: {str(e)}")
        sys.exit(1)


if __name__ == "__main__":
    main()
