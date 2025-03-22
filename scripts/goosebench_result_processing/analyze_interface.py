#!/usr/bin/env python3
"""
Interface definition for all analyze scripts.

This module defines the AnalyzeProtocol that all analyze scripts must implement.
"""

from typing import Dict, Any, List
from typing_extensions import Protocol

class AnalyzeProtocol(Protocol):
    """Protocol defining the interface for all analyze scripts."""
    
    def analyze_eval(self, file_path: str) -> Dict[str, Any]:
        """Load and analyze a single eval_result.json file.
        
        Args:
            file_path: Path to the eval_result.json file
            
        Returns:
            Dict[str, Any]: Dictionary containing analysis results including:
                - correct_results (bool): Whether all criteria were met
                - correctness_score (float): Normalized score (0-1)
                - Standard metrics (tokens, tool calls, execution time)
                
        Raises:
            FileNotFoundError: If the eval_result.json file doesn't exist
            json.JSONDecodeError: If the file contains invalid JSON
        """
        ...
    
    def aggregate_metrics(self, runs: List[Dict[str, Any]]) -> Dict[str, Any]:
        """Calculate additional statistics specific to the evaluation.
        
        Args:
            runs: List of run results from analyze_eval
            
        Returns:
            Dict[str, Any]: Dictionary containing:
                - success_rate: Rate of successful runs
                - avg_correctness_score: Average correctness score
                - Additional evaluation-specific metrics
        """
        ... 