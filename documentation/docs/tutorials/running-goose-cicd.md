---
title: Running Goose in CI/CD Environments
description: Learn how to set up Goose in your CI/CD pipeline. Automate Goose interactions for tasks like code review, documentation checks, and other automated workflows.
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

With the same way we use Goose to resolve issues on our local machine, we can also use Goose in CI/CD environments to automate tasks like code review, documentation checks, and other automated workflows. This tutorial will guide you through setting up Goose in your CI/CD pipeline.

## Common Use Cases

Here are some common ways to use Goose in your CI/CD pipeline:

- Automating Build and Deployment Tasks
- Infrastructure and Environment Management
- Automating Rollbacks and Recovery
- Intelligent Test Execution


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

There might also be cases where you want to use Goose with other environments, custom setups etc. In such cases, you can use Goose extensions to interact with these environments. 

You can find related extensions as MCP Servers on [PulseMCP](https://www.pulsemcp.com/servers) and interact with them using Goose.

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