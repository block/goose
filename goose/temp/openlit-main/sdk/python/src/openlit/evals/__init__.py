"""
openlit.evals

This module provides a set of classes for analyzing text for various types of
content-based vulnerabilities,
such as Hallucination, Bias, and Toxicity detection.
"""

from openlit.evals.hallucination import Hallucination
from openlit.evals.bias_detection import BiasDetector
from openlit.evals.toxicity import ToxicityDetector
from openlit.evals.all import All
