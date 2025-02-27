---
title: Running Goose in CI/CD Environments
description: Learn how to set up Goose in your CI/CD pipeline. Automate Goose interactions for tasks like code review, documentation checks, and other automated workflows.
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

With the same way we use Goose to resolve issues on our local machine, we can also use Goose in CI/CD environments to automate tasks like code review, documentation checks, and other automated workflows. This tutorial will guide you through setting up Goose in your CI/CD pipeline.

## Common Use Cases

Here are some common ways to use Goose in your CI/CD pipeline:

**Code Review Assistant**: Analyze pull request changes, provide suggestions for improvements, and check for common issues or anti-patterns.

**Documentation Validator**: Ensure documentation is up-to-date, consistent, and follows best practices.

**Test Coverage Analysis**: Review test coverage reports, suggest areas needing more tests, and ensure test quality and effectiveness.

**Summary Generation**: Generate summaries of changes, issues, or other relevant information for pull requests.


## Using Goose with GitHub CLI

#### Install and Authenticate GitHub CLI (`gh`)

Install the [GitHub CLI](https://cli.github.com/) `gh` to enable Goose to run the commands locally and access your CI details.  

```bash
brew install gh
gh auth login
```

Follow the prompts to authenticate your account.

:::info
You can use either the Goose CLI or Desktop app to interact with the GitHub CLI.
:::

With the GitHub CLI, you can do the following:
- Find and investigate specific failed tests and runs
- Monitor multiple pull requests and active workflows
- Access build logs, compare branches and review code changes.

#### Example Request

Navigate to the repository folder and run:

```
Using the local GitHub CLI `gh` command, can you tell me about the CI failures on PR #11111 in the organization/repo repository? The check run ID is 123456.
```

#### Example Response
```
I can help by using the shell command to run the GitHub CLI. Let me try that:

Shell
command
gh run view 123456 --repo organization/repo

Output
Let me get the failed logs to see more details about what went wrong:

Shell
command
gh run view 123456 --log-failed --repo organization/repo

Output
Based on the CI logs, I can see that the documentation site preview is failing during the build process. Here's what's happening:

The job docs/goose-cicd Documentation Site Preview failed
The failure occurred in the "Install dependencies and build docs" step
The specific error is a parse error in the Docusaurus configuration file:

[ERROR] Error: Docusaurus could not load module at path "/home/runner/work/goose/goose/documentation/docusaurus.config.ts"
Cause: ParseError: Unexpected token  
/home/runner/work/goose/goose/documentation/docusaurus.config.ts:98:8
The main issues appear to be:

There's a syntax error in the docusaurus.config.ts file at line 98, character 8
There are also some dependency warnings about React version conflicts:
The project is using React 19.0.0
Some dependencies require React versions < 19.0.0:
react-json-view-lite requires ^16.13.1 || ^17.0.0 || ^18.0.0
@docsearch/react requires >= 16.8.0 < 19.0.0
To fix this:

The syntax error in docusaurus.config.ts needs to be corrected
The React version conflicts should be addressed, possibly by downgrading to React 18.x
Would you like me to help you examine the docusaurus.config.ts file to identify the specific syntax error?
```

#### Request Fixes from Goose
With Goose providing the details of the CI failures, you can now ask Goose to help you fix the issues. For example, you can ask Goose to:

```
I'm in the current repo folder. Can you fix these issues and stage the changes so I can review them? Do not add unnecessary code comments.
```

**Example Response:**

```
I'll help fix the Docusaurus config file issue. First, let me check the current content:

Text Editor

/Users/example/documentation/docusaurus.config.ts

I can see the syntax error now. There are two issues around line 98:

- An invalid `as` type assertion without a type
- A typo ],ds which appears to be a corrupted array closing

Let me fix these issues:

Text Editor
str_replace
...

Now let me stage the changes:

Shell

git add documentation/docusaurus.config.ts

I've fixed the issues in the Docusaurus config file and staged the changes. Here's what I did:

...

The changes have been staged and are ready for you to review. You can now:

- Review the changes with git diff --cached
- Commit the changes if they look good
- Push the changes to update the PR

Would you like me to show you the staged changes with git diff --cached to review them?

```



## Using Goose with GitHub Actions

You can also use Goose directly in your GitHub Actions workflow, follow these steps:

:::info TLDR
<details>
   <summary>Copy the GitHub Workflow</summary>
   ```yaml

   name: Goose

   on:
      pull_request:
         types: [opened, synchronize, reopened, labeled]

   permissions:
      contents: write
      pull-requests: write
      issues: write

   env:
      PROVIDER_API_KEY: ${{ secrets.REPLACE_WITH_PROVIDER_API_KEY }}
      PR_NUMBER: ${{ github.event.pull_request.number }}

   jobs:
      goose-comment:
         runs-on: ubuntu-latest

         steps:
               - name: Check out repository
               uses: actions/checkout@v4
               with:
                     fetch-depth: 0

               - name: Gather PR information
               run: |
                     {
                     echo "# Files Changed"
                     gh pr view $PR_NUMBER --json files \
                        -q '.files[] | "* " + .path + " (" + (.additions|tostring) + " additions, " + (.deletions|tostring) + " deletions)"'
                     echo ""
                     echo "# Changes Summary"
                     gh pr diff $PR_NUMBER
                     } > changes.txt

               - name: Install Goose CLI
               run: |
                     mkdir -p /home/runner/.local/bin
                     curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh \
                     | CONFIGURE=false INSTALL_PATH=/home/runner/.local/bin bash
                     echo "/home/runner/.local/bin" >> $GITHUB_PATH

               - name: Configure Goose
               run: |
                     mkdir -p ~/.config/goose
                     cat <<EOF > ~/.config/goose/config.yaml
                     GOOSE_PROVIDER: REPLACE_WITH_PROVIDER
                     GOOSE_MODEL: REPLACE_WITH_MODEL
                     keyring: false
                     EOF

               - name: Create instructions for Goose
               run: |
                     cat <<EOF > instructions.txt
                     Create a summary of the changes provided. Don't provide any session or logging details.
                     The summary for each file should be brief and structured as:
                     <filename/path (wrapped in backticks)>
                        - dot points of changes
                     You don't need any extensions, don't mention extensions at all.
                     The changes to summarise are:
                     $(cat changes.txt)
                     EOF

               - name: Test
               run: cat instructions.txt

               - name: Run Goose and filter output
               run: |
                     goose run --instructions instructions.txt | \
                     # Remove ANSI color codes
                     sed -E 's/\x1B\[[0-9;]*[mK]//g' | \
                     # Remove session/logging lines
                     grep -v "logging to /home/runner/.config/goose/sessions/" | \
                     grep -v "^starting session" | \
                     grep -v "^Closing session" | \
                     # Trim trailing whitespace
                     sed 's/[[:space:]]*$//' \
                     > pr_comment.txt

               - name: Post comment to PR
               run: |
                     cat -A pr_comment.txt
                     gh pr comment $PR_NUMBER --body-file pr_comment.txt
   ```
</details>

:::

#### Create the Workflow File

Create a new file in your repository at `.github/workflows/goose.yml`. This will contain your GitHub Actions workflow configuration.

#### Configure Basic Workflow Structure

Here's a basic workflow structure that triggers Goose on pull requests:

```yaml
name: Goose

on:
    pull_request:
        types: [opened, synchronize, reopened, labeled]

permissions:
    contents: write
    pull-requests: write
    issues: write

env:
   PROVIDER_API_KEY: ${{ secrets.REPLACE_WITH_PROVIDER_API_KEY }}
   PR_NUMBER: ${{ github.event.pull_request.number }}
```

This configuration:
- Triggers the workflow on pull request events
- Sets necessary permissions for GitHub Actions
- Configures environment variables for your chosen Goose provider

#### Install and Configure Goose

The workflow needs to install and configure Goose in the CI environment. Here's how to do it:

```yaml
steps:
    - name: Install Goose CLI
      run: |
          mkdir -p /home/runner/.local/bin
          curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh \
            | CONFIGURE=false INSTALL_PATH=/home/runner/.local/bin bash
          echo "/home/runner/.local/bin" >> $GITHUB_PATH

    - name: Configure Goose
      run: |
          mkdir -p ~/.config/goose
          cat <<EOF > ~/.config/goose/config.yaml
          GOOSE_PROVIDER: REPLACE_WITH_PROVIDER
          GOOSE_MODEL: REPLACE_WITH_MODEL
          keyring: false
          EOF
```

Replace `REPLACE_WITH_PROVIDER` and `REPLACE_WITH_MODEL` with your Goose provider and model names and add any other necessary configuration required.

#### Prepare Instructions for Goose

Create instructions for Goose to follow based on the PR changes:

```yaml
    - name: Create instructions for Goose
      run: |
          cat <<EOF > instructions.txt
          Create a summary of the changes provided. Don't provide any session or logging details.
          The summary for each file should be brief and structured as:
            <filename/path (wrapped in backticks)>
              - dot points of changes
          You don't need any extensions, don't mention extensions at all.
          The changes to summarise are:
          $(cat changes.txt)
          EOF
```

#### Run Goose and Filter Output

Run Goose with the prepared instructions and filter the output for clean results:

```yaml
    - name: Run Goose and filter output
      run: |
          goose run --instructions instructions.txt | \
            # Remove ANSI color codes
            sed -E 's/\x1B\[[0-9;]*[mK]//g' | \
            # Remove session/logging lines
            grep -v "logging to /home/runner/.config/goose/sessions/" | \
            grep -v "^starting session" | \
            grep -v "^Closing session" | \
            # Trim trailing whitespace
            sed 's/[[:space:]]*$//' \
            > pr_comment.txt
```

#### Post Comment to PR

Finally, post the Goose output as a comment on the pull request:

```yaml
    - name: Post comment to PR
      run: |
          cat -A pr_comment.txt
          gh pr comment $PR_NUMBER --body-file pr_comment.txt
```

With this workflow, Goose will run on pull requests, analyze the changes, and post a summary as a comment on the PR.

## Using CI specific MCP servers as Goose extensions

There might also be cases where you want to use Goose with other environment, custom setups etc. In such cases, you can use Goose extensions to interact with these environments. 

You can find related extensions as MCP Server on [PulseMCP](https://www.pulsemcp.com/servers) and interact with them using Goose.

Process Goose's output to ensure it's clean and useful:

```yaml
    - name: Run Goose and filter output
      run: |
          goose run --instructions instructions.txt | \
            # Remove ANSI color codes
            sed -E 's/\x1B\[[0-9;]*[mK]//g' | \
            # Remove session/logging lines
            grep -v "logging to /home/runner/.config/goose/sessions/" | \
            grep -v "^starting session" | \
            grep -v "^Closing session" | \
            # Trim trailing whitespace
            sed 's/[[:space:]]*$//' \
            > pr_comment.txt
```


## Security Considerations

When running Goose in CI/CD, keep these security practices in mind:

1. **Secret Management**: Store your sensitive credentials (like API tokens) as 'Secrets' that you can pass to GOose as environment variables. Never expose these credentials in logs or PR comments

2. **Permissions**: When using a script or workflow, ensure you follow the principle of least privilege. Only grant necessary permissions in the workflow and regularly audit workflow permissions.

3. **Input Validation**: Validate and sanitize inputs before passing to Goose. Consider using action inputs with specific types and implement appropriate error handling.