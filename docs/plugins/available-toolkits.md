# Available Toolkits in Goose

Goose provides a variety of toolkits designed to help developers with different tasks. Here's an overview of each available toolkit and its functionalities:

## 1. Developer Toolkit

The **Developer** toolkit offers general-purpose development capabilities, including:

- **System Configuration Details:** Retrieves system configuration details.
- **Task Management:** Update the plan by overwriting all current tasks.
- **File Operations:**
  - `patch_file`: Patch a file by replacing specific content.
  - `read_file`: Read the content of a specified file.
  - `write_file`: Write content to a specified file.
- **Shell Command Execution:** Execute shell commands with safety checks.

## 2. GitHub Toolkit

The **GitHub** toolkit provides detailed configuration and procedural guidelines for GitHub operations, including:

- **Pull Request Reviews:** View and analyze PR reviews and nested comments.
- **API Integration:** Access GitHub API for repository operations.
- **Command Line Interface:** Integration with `gh` CLI tool.

## 3. JIRA Toolkit

The **JIRA** toolkit facilitates interaction with JIRA issues and projects through:

- **Issue Management:** View and interact with JIRA issues.
- **Command Line Integration:** Integration with `jira` CLI tool.
- **Authentication:** Handle JIRA authentication and initialization.

## 4. Memory Toolkit

The **Memory** toolkit provides persistent storage capabilities:

- **Local and Global Storage:** Store memories in both local (.goose/memory) and global (~/.config/goose/memory) locations.
- **Categorization:** Organize memories with categories and tags.
- **Natural Language Format:** Store and retrieve memories in natural language format.
- **Template Integration:** Use memories in system prompts via Jinja templates.

## 5. RepoContext Toolkit

The **RepoContext** toolkit provides context about the current repository:

- **Repository Size:** Get the size of the repository.
- **Monorepo Check:** Determine if the repository is a monorepo.
- **Project Summarization:** Summarize the current project based on the repository or project directory.

## 6. Screen Toolkit

The **Screen** toolkit assists users in taking screenshots for debugging or designing purposes:

- **Take Screenshot:** Capture a screenshot and provide the path to the screenshot file.
- **System Instructions:** Instructions on how to work with screenshots.

## 7. Summarization Toolkits

Goose includes several summarization-focused toolkits:

### 7.1 SummarizeRepo Toolkit
- **Repository Analysis:** Clone and summarize repositories based on specified extensions.

### 7.2 SummarizeProject Toolkit
- **Project Overview:** Generate or retrieve summaries of project directories based on specified file extensions.

### 7.3 SummarizeFile Toolkit
- **File Content Analysis:** Summarize specific files with optional custom instructions.

## 8. Web Browser Toolkit

The **Web Browser** toolkit provides web interaction capabilities:

- **Web Content Access:** Fetch and analyze web content.
- **URL Handling:** Process and validate URLs.
- **Content Extraction:** Extract relevant information from web pages.

## 9. Reasoner Toolkit

The **Reasoner** toolkit enhances decision-making capabilities:

- **Logical Analysis:** Apply reasoning to complex problems.
- **Decision Support:** Help evaluate options and make informed choices.
- **Pattern Recognition:** Identify patterns and relationships in data.

## 10. Synopsis Toolkit

The **Synopsis** toolkit provides core development and system interaction capabilities. Note that this toolkit requires the Synopsis moderator to be enabled to function properly.

- **Bash Operations:** Execute shell commands with working directory and source file support.
- **Text Editing:** View, create, replace, and insert content in files with undo support.
- **Process Management:** Start, list, view output, and cancel background processes.
- **Web Content:** Fetch and analyze web content with HTML and text extraction.

> **Important:** This toolkit only works when used with the Synopsis moderator. Make sure the Synopsis moderator is enabled in your configuration to use these capabilities.