# goose Documentation Map

> Auto-generated. Last updated: 2026-02-23

## Getting Started

### [Install goose](docs/getting-started/installation.md)

* Set LLM Provider
* Update Provider
* Running goose
* Shared Configuration Settings
* Pin a goose version in CI/CD
* Additional Resources

### [Configure LLM Provider](docs/getting-started/providers.md)

* Available Providers
  * CLI Providers
* Configure Provider and Model
  * Using Custom OpenAI Endpoints
    * Configuration Parameters
    * Example Configurations
    * Setup Instructions
* Configure Custom Provider
* Using goose for Free
  * Groq
  * Google Gemini
  * Local LLMs
* GitHub Copilot Authentication
* Azure OpenAI Credential Chain
* Multi-Model Configuration
* Gemini 3 Thinking Levels
* Viewing Model Reasoning

### [Using Extensions](docs/getting-started/using-extensions.md)

* Built-in Extensions
  * Built-in Platform Extensions
  * Toggling Built-in Extensions
* Discovering Extensions
* Adding Extensions
  * MCP Servers
  * Deeplinks
  * Config Entry
* Enabling/Disabling Extensions
  * Set Default Extensions for New Sessions
  * Change Extensions Mid-Session
* Automatically Enabled Extensions
  * Automatic Detection
    * goose Prompt
    * goose Output
    * goose Prompt
    * goose Output
  * Direct Request
    * goose Prompt
    * goose Output
    * goose Prompt
    * goose Output
* Updating Extension Properties
* Removing Extensions
* Starting Session with Extensions
  * Built-in Extensions
  * External Extensions
    * Environment Variables
  * Remote Extensions over Streamable HTTP
  * Extensions in Containers
* Developing Extensions

## Guides

### [Using goose in ACP Clients](docs/guides/acp-clients.md)

* How It Works
* Setup in ACP Clients
  * Example: Zed Editor Setup
    * 1. Prerequisites
    * 2. Configure goose as a Custom Agent
    * 3. Start Using goose in Zed
    * Advanced Configuration
      * Overriding Provider and Model
* Using MCP Servers from ACP Clients
* Additional Resources

### [goose Extension Allowlist](docs/guides/allowlist.md)

* How It Works
* Configuration
  * 1. Create and Deploy Allowlist
    * Example
  * 2. Set Environment Variable
* Best Practices
* Troubleshooting

### [CLI Providers](docs/guides/cli-providers.md)

* Why Use CLI Providers?
  * Benefits
    * Session Management
    * Workflow Integration  
    * Interface Consistency
* Available CLI Providers
  * Claude Code
  * OpenAI Codex
  * Cursor Agent
  * Gemini CLI
* Setup Instructions
  * Claude Code
  * OpenAI Codex
  * Cursor Agent
  * Gemini CLI
* Usage Examples
  * Basic Usage
  * Combining with Other Models
* Configuration Options
  * Claude Code Configuration
  * Cursor Agent Configuration
  * OpenAI Codex Configuration
  * Gemini CLI Configuration
* How It Works
  * System Prompt Filtering
  * Message Translation
  * Response Processing
* Error Handling

### [Codebase Analysis](docs/guides/codebase-analysis.md)

    * Function Definition
    * Incoming Calls (Functions that call authenticate)
    * Outgoing Calls (Functions that authenticate calls)
* Analysis Modes
  * Understanding Project Organization
  * Inspecting a File
  * Tracking a Symbol Across Files
* Common Parameters
* Best Practices
  * Handling Large Outputs
  * Performance Tips

### [Configuration Files](docs/guides/config-files.md)

* Configuration Files
* Global Settings
* Example Configuration
* Extensions Configuration
* Search Path Configuration
* Observability Configuration
* Recipe Command Configuration
* Configuration Priority
* Security Considerations
* Updating Configuration
* See Also

### [Context Engineering](docs/guides/context-engineering/index.md)

### [Custom Slash Commands](docs/guides/context-engineering/slash-commands.md)

* Create Slash Commands
* Use Slash Commands
* Limitations
* Additional Resources

### [Providing Hints to goose](docs/guides/context-engineering/using-goosehints.md)

* Creating Your Hints File
* Setting Up Hints
  * Example Global `.goosehints` File
  * Example Local `.goosehints` File
  * Nested `.goosehints` Files
* Common Use Cases
* Best Practices
* Custom Context Files
  * Configuration

### [Using Skills](docs/guides/context-engineering/using-skills.md)

* Skill Locations
* Creating a Skill
  * Skill File Structure
* Functionality
* Code Quality
* Testing
* Security
  * Supporting Files
* Steps
* Configuration
* Verification
* Common Use Case Examples
* Pre-deployment
* Deploy
* Rollback
* Unit Tests
* Integration Tests
* Running Tests
* Authentication
* Common Operations
  * Create a customer
  * Handle webhooks
* Error Handling
* Best Practices
* Additional Resources

### [Creating Plans Before Working](docs/guides/creating-plans.md)

* Set your planner provider and model
  * Set goose planner environment variables
* Describe your project
* A simple construction plan example
  * Introduction to Building a 4,000 Sq Ft Rambler
  * Step 1: Design and Planning
  * Step 2: Energy Efficiency and Solar Power
  * Step 3: Budgeting
  * Step 4: Timeline
  * Conclusion
  * Create a separate plan for plan sub-steps
* A development project example
* Clarifying questions
* Step 1: Set up the React project and dependencies
* Step 2: Implement authentication with Okta
* Step 3: Create the layout and UI components with Material UI
* Step 4: Implement Contentful service
* Step 5: Implement local storage service for history
* Step 6: Implement validation utilities
* Step 7: Implement main pages
* Step 8: Set up routing and main application
* Step 9: Implement error handling and loading states
* Step 10: Set up deployment configuration
* Step 11: Testing and quality assurance
* Basic usage
* Additional Resources

### [Custom Distributions](docs/guides/custom-distributions.md)

* What you can customize
* Getting started
* Quick example: ship goose with a local model

### [Enhanced Code Editing with AI Models](docs/guides/enhanced-code-editing.md)

* Configuration
  * Supported Providers
* How It Works

### [Environment Variables](docs/guides/environment-variables.md)

* Model Configuration
  * Basic Provider Configuration
  * Advanced Provider Configuration
  * Custom Model Definitions
  * Lead/Worker Model Configuration
  * Claude Extended Thinking
  * Planning Mode Configuration
  * Provider Retries
    * AWS Bedrock
    * Databricks
* Session Management
  * Model Context Limit Overrides
* Tool Configuration
  * Enhanced Code Editing
* Security and Privacy
  * macOS Sandbox for goose Desktop
* Network Configuration
  * HTTP Proxy
* Observability
  * Observability Configuration
  * Langfuse Integration
* Recipe Configuration
* Development & Testing
* Variables Controlled by goose
  * Customizing Shell Behavior
  * Using Session IDs in Workflows
* Environment Variable Passthrough
* Enterprise Environments
* Notes

### [File Access and Management](docs/guides/file-management.md)

* File Access
  * Quick File Search in goose Desktop
* File Management Best Practices
  * Version Control
  * Validation and Testing
  * Change Review
  * Codebase Organization

### [CLI Commands](docs/guides/goose-cli-commands.md)

* Flag Naming Conventions
  * Core Commands
    * help
    * configure
    * info [options]
    * version
    * update [options]
    * completion
  * Session Management
    * session [options]
    * session list [options]
    * session remove [options]
    * session export [options]
    * session diagnostics [options]
  * Task Execution
    * run [options]
    * bench
    * recipe
    * schedule
    * mcp
    * acp
  * Project Management
    * project
    * projects
  * Interface
    * web
  * Terminal Integration
    * @goose / @g
* Interactive Session Features
  * Slash Commands
  * Themes
* Navigation and Controls
  * Keyboard Shortcuts
  * External Editor Mode
  * Command History Search

### [goose Permission Modes](docs/guides/goose-permissions.md)

* Permission Modes
* Configuring goose mode

### [Set LLM Rate Limits](docs/guides/handling-llm-rate-limits-with-goose.md)

### [Rich Interactive Chat with MCP Apps and MCP-UI](docs/guides/interactive-chat/index.md)

### [Using MCP Apps and MCP-UI](docs/guides/interactive-chat/mcp-ui.md)

* MCP Apps
  * Launching Apps in Standalone Windows
    * Import an HTML App
  * Using Apps in Chat Windows
* MCP-UI
  * Try It Out
* For Extension Developers

### [goose Logging System](docs/guides/logs.md)

* Command History
* Session Records
* System Logs
  * Desktop Application Log
  * CLI Logs 
  * Server Logs
  * LLM Request Logs

### [Managing Projects](docs/guides/managing-projects.md)

* Basic Usage
* Workflow Example
  * Morning: API Development
  * Mid-Morning: Mobile App Bug Fix  
  * Afternoon: Admin Dashboard
  * Next Day: Quick Resume
  * Later: Browse All Projects
* Benefits

### [Adjusting Tool Output Verbosity](docs/guides/managing-tools/adjust-tool-output.md)

  * Toggle Parameter Truncation

### [Code Mode](docs/guides/managing-tools/code-mode.md)

* How Code Mode Works
  * Traditional vs. Code Mode Tool Calling
* Additional Resources

### [Managing Tools](docs/guides/managing-tools/index.md)

### [Managing Tool Permissions](docs/guides/managing-tools/tool-permissions.md)

* Understanding Tools and Extensions
* Permission Levels
* Configuring Tool Permissions
* Benefits of Permission Management
* Example Permission Configuration
  * Task-Based Configuration

### [MCP Elicitation](docs/guides/mcp-elicitation.md)

* How MCP Elicitation Works
* For Extension Developers

### [MCP Sampling Extensions](docs/guides/mcp-sampling.md)

* How MCP Sampling Works
  * Use Cases
* For Extension Developers
* Additional Resources

### [Multi-Model Configuration](docs/guides/multi-model/index.md)

### [Customizing Prompt Templates](docs/guides/prompt-templates.md)

* How It Works
* Managing Prompt Templates
  * Available Prompt Templates
  * Template Variable Syntax
    * Escaping Template Variables
* Additional Resources

### [Recipes](docs/guides/recipes/index.md)

### [Recipe Reference Guide](docs/guides/recipes/recipe-reference.md)

* Recipe File Format
* Recipe Location
* Core Recipe Schema
* Field Specifications
  * Activities
    * Activity Types
    * Parameter Substitution
    * Example Configuration
  * Extensions
    * Extension Schema
    * Extension Types
    * Example Extensions Configuration
    * Extension Secrets
  * Parameters
    * Parameter Schema
    * Parameter Requirements
    * Input Types
    * Parameter Substitution in Desktop
  * Response
    * Response Schema
    * Basic Structure
    * Simple Example
  * Retry
    * Retry Schema
    * Success Check Configuration
    * How Retry Logic Works
    * Basic Retry Example
    * Advanced Retry Example
    * Environment Variables
  * Settings
    * Settings Schema
    * Understanding max_turns
    * Example Settings Configuration
  * Subrecipes
    * Subrecipe Schema
    * Example Subrecipe Configuration
* Desktop Metadata Fields
* Template Support
  * Escaping Template Variables
  * Template Inheritance
  * indent() Filter For Multi-Line Values
  * Built-in Parameters
* Validation Rules
  * Recipe-Level Validation
  * Parameter Validation
* Complete Recipe Example
* Error Handling
  * Retry-Specific Errors
* Learn More

### [Shareable Recipes](docs/guides/recipes/session-recipes.md)

* Create Recipe
* Edit Recipe
* Use Recipe
* Validate Recipe
* Share Recipe
  * Share via Recipe Link
  * Share via Recipe File
* Schedule Recipe
* Core Components
* Advanced Features
  * Automated Retry Logic
  * Structured Output for Automation
* What's Included
* Learn More

### [Saving Recipes](docs/guides/recipes/storing-recipes.md)

* Understanding Recipe Storage
  * Recipe Storage Locations
* Storing Recipes
  * Importing Recipes
* Finding Available Recipes
* Using Saved Recipes

### [Subrecipes For Specialized Tasks](docs/guides/recipes/subrecipes.md)

* How Subrecipes Work
  * Parameter Handling
* Examples
  * Sequential Processing
  * Conditional Processing
  * Context-Based Parameter Passing
* Best Practices
* Learn More

### [Running Tasks](docs/guides/running-tasks.md)

* Basic Usage
  * Text in the command
  * Using an instruction file
  * With stdin
    * Simple echo pipe
    * Multi-line instructions
* Key Features
  * Interactive Mode
  * Session Management
  * Set Provider and Model
  * Working with Extensions
  * Debug Mode
  * JSON Output Format
* Common Use Cases
  * Running Script Files
  * Quick Commands
  * Development Workflows
  * Combining Options

### [macOS Sandbox for goose Desktop](docs/guides/sandbox.md)

* Quick Start
* Configuration
  * Core
  * File System
    * Environment Variables
  * Direct Network Access
  * Process Restrictions
    * Environment Variables
  * Network Filtering
    * Environment Variables
    * Managing the Domain Blocklist
    * Using Git Over SSH
  * LaunchDarkly (Optional)
    * Environment Variables
* Example Configurations
  * Maximum security
  * Allow raw IP connections (e.g. for internal APIs)
  * Disable SSH entirely
  * Relaxed mode (fewer restrictions)
  * With LaunchDarkly egress control
* Troubleshooting

### [Classification API Specification](docs/guides/security/classification-api-spec.md)

* Security & Privacy Considerations
* Endpoint
  * POST /
    * Request
    * Response
    * Status Codes
    * Example

### [Staying Safe with goose](docs/guides/security/index.md)

### [Prompt Injection Detection](docs/guides/security/prompt-injection-detection.md)

* How Detection Works
* Enabling Detection
  * Configuring Detection Threshold
* Enhanced Detection with Machine Learning
    * Self-Hosting ML Detection Endpoints
* See Also

### [In-Session Actions](docs/guides/sessions/in-session-actions.md)

* Edit Message
  * Edit in Place
  * Fork Session
  * Editing Scenario Tips
* Queue Messages
* Interrupt Task
* Voice Dictation
* Spellcheck
* Share Files in Session
* Mid-Session Changes

### [Managing Sessions](docs/guides/sessions/index.md)

### [Session Management](docs/guides/sessions/session-management.md)

* Start Session 
* Name Session
* Exit Session
* Search Sessions
* Resume Session
  * Resume Project-Based Sessions
* Duplicate Sessions
* Delete Sessions
* Import Sessions
* Export Sessions

### [Smart Context Management](docs/guides/sessions/smart-context-management.md)

* How goose Manages Context
* Automatic Compaction
  * Manual Compaction
* Context Limit Strategies
* Maximum Turns
* Token Usage
* Model Context Limit Overrides
* Cost Tracking

### [Subagents](docs/guides/subagents.md)

* How to Use Subagents
* Monitoring Subagent Activity
* Internal Subagents
  * Direct Prompts
  * Recipes
* Code Review Results
  * Critical Issues Found:
  * Recommendations:
* External Subagents
* Suggested Use Cases
* Lifecycle and Cleanup
* Configuration
  * Default Settings
  * Customizing Settings in Prompts
  * Extension Control
  * Return Mode Control
* Security Constraints
  * Allowed Operations
  * Restricted Operations
* Additional Resources

### [Terminal Integration](docs/guides/terminal-integration.md)

* Setup
* Usage
* Named Sessions
* Show Context Status in Your Prompt
* Shell Completion for goose Commands
* Troubleshooting

### [Quick goose Tips](docs/guides/tips.md)

  * goose works on your behalf
  * Prompt goose using natural language
  * Extend goose's capabilities to any application
  * Choose how much control goose has
  * Choose the right LLM
  * Keep sessions short
  * Use Quick Launcher for faster session starts
  * Turn off unnecessary extensions or tool
  * Teach goose your preferences
  * Protect sensitive files
  * Version Control
  * Control which extensions goose can use
  * Set up starter templates
  * Embrace an experimental mindset
  * Keep goose updated
  * Pair Two Models to Save Money 
  * Make Recipes Safe to Re-run
  * Add Logging to Recipes

### [Updating goose](docs/guides/updating-goose.md)

### [Anonymous Usage Data](docs/guides/usage-data.md)

* Usage data collected
* Change Your Preference

### [Prevent goose from Accessing Files](docs/guides/using-gooseignore.md)

* Creating your `.gooseignore` file
* Example `.gooseignore` file
* Negation Patterns
* Ignore File Types and Priority
  * When You Have Ignore Files
  * Default Patterns (No Ignore Files)
* Common use cases

### [Persistent Instructions](docs/guides/using-persistent-instructions.md)

* How It Works
* Configuration
* Examples
  * Simple Text Reminder
  * File-Based Instructions
* Security Guidelines
* Code Quality
  * Combining Both
* Use Cases
  * Security Guardrails
  * Environment-Specific Behavior
  * Project-Specific Workflows
  * Temporary Reminders
* Persistent Instructions vs goosehints
* Best Practices

---

> Full docs: https://block.github.io/goose/
