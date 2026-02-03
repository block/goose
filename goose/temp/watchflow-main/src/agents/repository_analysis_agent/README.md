# Repository Analysis Agent

the Repository Analysis Agent analyzes GitHub repositories to generate personalized Watchflow rule recommendations based on repository structure, contributing guidelines, and development patterns.

## Overview

this agent performs comprehensive analysis of repositories and provides actionable governance rule recommendations with confidence scores and reasoning.

## Features

- **Repository Structure Analysis**: Examines workflows, branch protection, contributors, and repository metadata
- **Contributing Guidelines Parsing**: Uses LLM analysis to extract requirements from CONTRIBUTING.md files
- **Pattern-Based Recommendations**: Generates rules based on detected repository characteristics
- **Confidence Scoring**: Each recommendation includes a confidence score (0.0-1.0) and reasoning
- **Valid YAML Generation**: All recommendations are valid Watchflow rule YAML

## Usage

### Direct Agent Usage

```python
from src.agents import get_agent


agent = get_agent("repository_analysis")


result = await agent.execute(
    repository_full_name="owner/repo-name",
    installation_id=12345
)


response = result.data["analysis_response"]
for recommendation in response.recommendations:
    print(f"Confidence: {recommendation.confidence}")
    print(f"Category: {recommendation.category}")
    print(f"Reasoning: {recommendation.reasoning}")
    print(f"YAML:\n{recommendation.yaml_content}")
```



## Recommendation Categories

The agent generates recommendations in the following categories:

- **Quality**: Code quality rules (linting, testing, CI/CD)
- **Security**: Security-focused rules (dependency scanning, secrets detection)
- **Process**: Development process rules (reviews, approvals, branch protection)
- **Documentation**: Documentation-related rules (README updates, CHANGELOG)

## Analysis Workflow

The agent follows a multi-step LangGraph workflow:

1. **Repository Structure Analysis**: Gathers basic repository metadata
2. **Contributing Guidelines Analysis**: Parses CONTRIBUTING.md for requirements
3. **Rule Generation**: Creates recommendations based on detected patterns
4. **Validation**: Ensures all recommendations contain valid YAML
5. **Summarization**: Provides analysis summary and statistics

## Configuration

The agent can be configured with the following parameters:

- `max_retries`: Maximum retry attempts for LLM calls (default: 3)
- `timeout`: Maximum execution time in seconds (default: 120.0)

```python
agent = get_agent("repository_analysis", max_retries=5, timeout=300.0)
```

## Caching and Rate Limiting

The API endpoint includes:
- **Caching**: Successful analyses are cached for 1 hour
- **Rate Limiting**: Basic rate limiting to prevent abuse
- **Error Handling**: Comprehensive error handling with structured logging




## Integration with Watchflow.dev

This agent provides the backend for watchflow.dev's onboarding flow, automatically suggesting appropriate governance rules based on repository analysis.
