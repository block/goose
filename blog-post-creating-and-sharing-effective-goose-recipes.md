# Creating and Sharing Effective goose Recipes: A Comprehensive Guide

Recipes are one of goose's most powerful features for automating repetitive development tasks. They transform one-time workflows into reusable automation templates that can be shared with your team or the entire goose community. This comprehensive guide will teach you how to create, optimize, and share effective recipes that save time and boost productivity.

## What Are Recipes and Why They Matter

goose recipes are reusable configuration files that package up a specific setup so it can be easily shared and launched by others. Think of them as templates for common development workflows: code reviews, documentation generation, security audits, data analysis, or any task you find yourself doing repeatedly.

### The Power of Recipes

- **Time Savings**: Turn a 30-minute repetitive task into a single command
- **Consistency**: Ensure the same process is followed every time
- **Knowledge Sharing**: Capture best practices and make them accessible to your team
- **Automation**: Build complex workflows that combine multiple steps
- **Community**: Contribute to the open-source ecosystem and help others

## Creating Your First Recipe

### Start from a Successful Session

The best recipes often come from real work you've already done. When you've had a successful goose session that you'd want to repeat, convert it into a recipe.

#### Step 1: Identify Your Workflow

Think about:
- What was the core task you accomplished?
- What inputs did goose need?
- What was the expected output?
- What made this session successful?

#### Step 2: Extract the Core Elements

A recipe needs:
- **Title**: Clear, descriptive name
- **Description**: What the recipe does
- **Instructions**: High-level guidance for the agent
- **Prompt**: The specific task to accomplish
- **Parameters**: Dynamic inputs that make it reusable

#### Step 3: Write Your First Recipe

Here's a minimal recipe example:

```yaml
version: 1.0.0
title: "Code Documentation Generator"
description: "Automatically generates code documentation from source files"
instructions: "You are a documentation specialist focused on creating clear, comprehensive code documentation."
prompt: "Generate documentation for all functions in {{ source_directory }}"
parameters:
  - key: source_directory
    input_type: string
    requirement: required
    description: "Path to the directory containing source code"
extensions:
  - type: builtin
    name: developer
```

### Best Practices for Recipe Creation

#### 1. Write Clear Instructions

The `instructions` field sets the context and role for the agent. Be specific:

```yaml
instructions: |
  You are a security analyst specializing in static code analysis.
  Your expertise includes:
  - Identifying OWASP Top 10 vulnerabilities
  - Detecting insecure coding patterns
  - Analyzing authentication and authorization flaws
  - Providing actionable remediation guidance
```

#### 2. Craft Effective Prompts

The `prompt` field contains the actual task. Good prompts:
- Are specific and actionable
- Include context about inputs
- Define expected outputs
- Break down complex tasks into steps

**Good prompt:**
```yaml
prompt: |
  Analyze {{ project_path }} for security vulnerabilities:
  1. Scan for SQL injection patterns
  2. Check for hardcoded credentials
  3. Review authentication mechanisms
  4. Generate a report with severity levels
```

**Weak prompt:**
```yaml
prompt: "Check code for security issues"
```

#### 3. Parameterize Thoughtfully

Parameters make recipes flexible. Identify what should vary between uses:

```yaml
parameters:
  - key: target_file
    input_type: string
    requirement: required
    description: "File to analyze"
  
  - key: analysis_depth
    input_type: string
    requirement: optional
    default: "comprehensive"
    description: "Analysis depth: 'quick', 'comprehensive', or 'deep'"
  
  - key: output_format
    input_type: string
    requirement: optional
    default: "markdown"
    description: "Output format: 'markdown' or 'json'"
```

**Parameter Best Practices:**
- Use `required` for essential inputs
- Use `optional` with `default` for convenience
- Use `user_prompt` for interactive scenarios
- Use `input_type: file` to read file contents directly

#### 4. Use Template Syntax

goose uses Jinja-style templating. Reference parameters with `{{ parameter_name }}`:

```yaml
prompt: |
  Review the code in {{ file_path }}.
  
  Focus on:
  - Code quality
  - Security concerns
  - Performance optimizations
  
  Generate a report in {{ output_format }} format.
```

You can also use conditional logic:

```yaml
prompt: |
  Analyze {{ project_path }}.
  {% if include_tests == "true" %}
  Include test files in the analysis.
  {% endif %}
  
  {% if language_focus %}
  Prioritize {{ language_focus }}-specific best practices.
  {% endif %}
```

## Optimizing Your Recipes

### Iterative Improvement

Recipes should evolve. Start simple, then refine:

1. **Create a basic version** that works
2. **Use it several times** to identify pain points
3. **Add parameters** for variations you find yourself needing
4. **Improve prompts** based on outcomes
5. **Test edge cases** and handle them

### Advanced Optimization Techniques

#### 1. Use Subrecipes for Complex Workflows

Break complex recipes into reusable subrecipes:

```yaml
version: 1.0.0
title: "Full Stack Project Initializer"
description: "Creates a complete project with frontend, backend, and infrastructure"
sub_recipes:
  - name: "frontend_setup"
    path: "./subrecipes/frontend-setup.yaml"
    values:
      framework: "{{ frontend_framework }}"
  
  - name: "backend_setup"
    path: "./subrecipes/backend-setup.yaml"
    values:
      language: "{{ backend_language }}"
```

#### 2. Configure Model Settings

Different tasks benefit from different model configurations:

```yaml
settings:
  goose_provider: "anthropic"
  goose_model: "claude-sonnet-4-20250514"
  temperature: 0.7  # Higher for creative tasks, lower for precise tasks
```

- **Low temperature (0.0-0.3)**: Code generation, analysis, structured tasks
- **Medium temperature (0.4-0.7)**: Documentation, explanations, reviews
- **High temperature (0.8-1.0)**: Creative writing, brainstorming, ideation

#### 3. Add Retry Logic for Reliability

Automatically retry if success criteria aren't met:

```yaml
retry:
  max_retries: 3
  timeout_seconds: 30
  checks:
    - type: shell
      command: "test -f {{ output_file }}"
  on_failure: "echo 'Retry attempt failed, cleaning up...'"
```

#### 4. Use Structured Output for Automation

Enforce JSON schema output for scriptable results:

```yaml
response:
  json_schema:
    type: object
    properties:
      summary:
        type: string
        description: "Brief summary of findings"
      issues:
        type: array
        items:
          type: object
          properties:
            severity:
              type: string
            description:
              type: string
    required:
      - summary
      - issues
```

#### 5. Leverage Extensions (MCP Servers)

Connect recipes to external tools and data sources:

```yaml
extensions:
  - type: stdio
    name: codesearch
    cmd: uvx
    args:
      - mcp_codesearch@latest
    timeout: 300
    description: "Search code repositories"
  
  - type: stdio
    name: github-mcp
    cmd: github-mcp-server
    args: []
    env_keys:
      - GITHUB_PERSONAL_ACCESS_TOKEN
    timeout: 60
    description: "GitHub repository operations"
```

## Sharing Recipes with the Community

### Contributing to the goose Recipe Cookbook

The goose community maintains a [Recipe Cookbook](https://block.github.io/goose/recipes/) where you can share your creations. Approved submissions receive $10 in OpenRouter LLM credits!

#### Step 1: Prepare Your Recipe

Before submitting:
- **Test thoroughly**: Run your recipe multiple times with different inputs
- **Validate syntax**: Use `goose recipe validate your-recipe.yaml`
- **Check for sensitive data**: Remove API keys, personal paths, secrets
- **Write clear documentation**: Good description and parameter descriptions
- **Choose a unique name**: Check existing recipes to avoid conflicts

#### Step 2: Submit via Pull Request

1. **Fork the repository**: [Fork goose on GitHub](https://github.com/block/goose/fork)

2. **Add your recipe file**:
   - Navigate to: `documentation/src/pages/recipes/data/recipes/`
   - Create a new file: `your-recipe-name.yaml`
   - Use a descriptive, unique filename

3. **Follow the recipe format**:
   ```yaml
   version: 1.0.0
   title: "Your Recipe Name"
   description: "Brief description of what your recipe does"
   instructions: "Detailed instructions for what the recipe should accomplish"
   author:
     contact: "your-github-username"
   extensions:
     - type: builtin
       name: developer
   activities:
     - "Main activity 1"
     - "Main activity 2"
   prompt: |
     Detailed prompt describing the task step by step.
     
     Use {{ parameter_name }} to reference parameters.
   ```

4. **Create the pull request**:
   - Include your email in the PR description for credits
   - Follow conventional commits format
   - Add sign-off to commits: `git commit --signoff`

#### Step 3: Review Process

The goose team will:
1. ✅ Validate your recipe automatically
2. 👀 Review for quality and usefulness
3. 🔒 Security scan (if approved for review)
4. 🎉 Merge and send you $10 credits!

### Recipe Requirements

Your recipe should:
- ✅ **Work correctly** - Test it before submitting
- ✅ **Be useful** - Solve a real problem or demonstrate a valuable workflow
- ✅ **Follow the format** - Refer to the [Recipe Reference Guide](https://block.github.io/goose/docs/guides/recipes/recipe-reference)
- ✅ **Have a unique filename** - No conflicts with existing recipe files

### Recipe Ideas

Looking for inspiration? Consider recipes for:

- **Web scraping** workflows
- **Data processing** pipelines
- **API integration** tasks
- **File management** automation
- **Code generation** helpers
- **Testing** and validation
- **Deployment** processes
- **Documentation** generation
- **Security** audits
- **Performance** analysis

## Practical Example: Building a Recipe from Scratch

Let's walk through creating a real recipe: a "Security Audit Recipe" that analyzes code for vulnerabilities.

### Step 1: Start with the Goal

"I want to quickly scan any project for common security issues and get a report."

### Step 2: Create the Basic Structure

```yaml
version: 1.0.0
title: "Security Audit"
description: "Scans codebase for common security vulnerabilities"
instructions: |
  You are a security analyst specializing in static code analysis.
  Focus on finding OWASP Top 10 vulnerabilities and insecure patterns.
prompt: "Analyze {{ project_path }} for security vulnerabilities"
parameters:
  - key: project_path
    input_type: string
    requirement: required
    description: "Path to project directory"
extensions:
  - type: builtin
    name: developer
```

### Step 3: Add More Functionality

Let's enhance it with more parameters and better instructions:

```yaml
version: 1.0.0
title: "Security Audit"
description: "Comprehensive security audit for codebases"
instructions: |
  You are a security analyst specializing in static code analysis.
  
  Your expertise includes:
  - OWASP Top 10 vulnerabilities
  - CWE (Common Weakness Enumeration) patterns
  - Language-specific security issues
  - Authentication and authorization flaws
  - Input validation problems
  
  Provide actionable remediation guidance for each finding.
parameters:
  - key: project_path
    input_type: string
    requirement: required
    description: "Path to project directory to analyze"
  
  - key: analysis_depth
    input_type: string
    requirement: optional
    default: "comprehensive"
    description: "Analysis depth: 'quick', 'comprehensive', or 'deep'"
  
  - key: language_focus
    input_type: string
    requirement: optional
    default: "auto"
    description: "Programming language focus (auto-detects if not specified)"
  
  - key: include_tests
    input_type: string
    requirement: optional
    default: "false"
    description: "Whether to include test files in analysis"
extensions:
  - type: builtin
    name: developer
  - type: builtin
    name: memory
prompt: |
  Perform a {{ analysis_depth }} security audit on {{ project_path }}.
  
  1. Language Detection
     - Identify primary programming languages
     - Exclude test files unless {{ include_tests }} == "true"
  
  2. Security Analysis
     - Scan for injection vulnerabilities (SQL, NoSQL, OS command)
     - Check for XSS risks
     - Identify authentication flaws
     - Detect hardcoded credentials
     - Review encryption implementations
     {% if language_focus != "auto" %}
     - Focus on {{ language_focus }}-specific security patterns
     {% endif %}
  
  3. Report Generation
     Create a detailed report with:
     - Vulnerability type and severity
     - File path and line numbers
     - Risk assessment
     - Remediation recommendations
  
  4. Store findings in memory for tracking over time
```

### Step 4: Test and Refine

Run the recipe with different projects, adjust prompts based on results, and add parameters as you discover variations you need.

## Tips for Recipe Success

1. **Start Simple**: Begin with basic recipes, add complexity gradually
2. **Document Well**: Clear descriptions help users understand what the recipe does
3. **Test Edge Cases**: Handle missing files, empty parameters, various inputs
4. **Use Examples**: Include example values in parameter descriptions
5. **Iterate**: Recipes should improve over time based on real usage
6. **Share Early**: Get community feedback to improve your recipes
7. **Follow Patterns**: Study existing recipes for proven patterns
8. **Keep Updated**: Maintain recipes as goose features evolve

## Resources

- [Recipe Reference Guide](https://block.github.io/goose/docs/guides/recipes/recipe-reference): Complete technical reference
- [Recipe Tutorial](https://block.github.io/goose/docs/tutorials/recipes-tutorial): Step-by-step tutorial
- [Contributing Recipes Guide](https://github.com/block/goose/blob/main/CONTRIBUTING_RECIPES.md): How to contribute
- [Recipe Cookbook](https://block.github.io/goose/recipes/): Browse community recipes
- [goose Documentation](https://block.github.io/goose): Full documentation

## Conclusion

Recipes transform goose from a powerful AI assistant into a customizable automation platform. By creating, optimizing, and sharing recipes, you're not just saving your own time—you're contributing to a growing library of automation tools that benefit the entire development community.

Start small with a recipe for a task you do frequently, refine it through use, and consider sharing it. The goose community is always excited to see new recipes and the creative ways people are automating their workflows!

---

**Ready to create your first recipe?** Start by identifying a repetitive task from your recent goose sessions, extract the core workflow, and package it into a reusable recipe. Then test it, refine it, and share it with the community to earn credits and help others!

