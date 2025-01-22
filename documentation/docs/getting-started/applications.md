---
sidebar_position: 3
title: Applications of Goose
---

# Uses of Goose so Far

We've been using Goose to help us with a variety of tasks. Here are some examples:

- Conduct code migrations like:
    - Ember to React
    - Ruby to Kotlin
    - Prefect-1 to Prefect-2
- Dive into a new project in an unfamiliar coding language
- Transition a code-base from field-based injection to constructor-based injection in a dependency injection framework
- Conduct performance benchmarks for a build command using a build automation tool
- Increasing code coverage above a specific threshold
- Scaffolding an API for data retention
- Creating Datadog monitors
- Removing or adding feature flags

# Goose in action

This page is frequently updated with the latest use-cases and applications of Goose!

## Goose as a Github Action

**What it does**: 

An early version of a GitHub action that uses Goose to automatically address issues in your repository. It operates in the background to attempt fixes or enhancements based on issue descriptions.

The action attempts to fix issues described in GitHub. It takes the issue's title and body as input and tries to resolve the issue programmatically.

If the action successfully fixes the issue, it will automatically create a pull request with the fix. If it cannot confidently fix the issue, no pull request is created.

**Where you can find it**: https://github.com/marketplace/actions/goose-ai-developer-agent

**How you can do something similar**:

1. Decide what specific task you want Goose to automate. This could be anything from auto-linting code, updating dependencies, auto-merging approved pull requests, or even automating responses to issue comments.
2. In the `action.yml`, specify any inputs your action needs (like GitHub tokens, configuration files, specific command inputs) and outputs it may produce.
3. Write the script (e.g., Python or JavaScript) that Goose will use to perform the tasks. This involves setting up the Goose environment, handling GitHub API requests, and processing the task-specific logic.
