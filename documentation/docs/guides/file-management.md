---
title: File Access and Management
sidebar_position: 70
sidebar_label: File Management
description: Efficiently find and reference files in goose Desktop and follow best practices for safe file operations
---

As an autonomous agent, goose is designed to carry out tasks following specified instructions. This often involves working with local files - both finding the right files to work with and modifying them safely.

This guide covers how to efficiently access and reference files in goose. It also includes essential best practices for safe file operations, such as monitoring changes and reverting them when necessary, to maintain the integrity of your codebase.

## File Access

### Quick File Search in goose Desktop

goose Desktop includes a fuzzy file search feature that makes it easy to reference files from within the chat interface without manually navigating through file system dialogs. This feature helps you quickly find and include files in your messages to goose.

1. Type `@` in the chat input to open the file search box
2. Continue typing to filter files using case-insensitive, fuzzy matching (e.g., `@readme`, `@config.js`, `@src/main`)

   Navigate the results:
   - Use arrow keys (↑/↓) to move through the search results
   - Click or press `Enter` to insert the selected file path into your message
   
3. That's it! When you're ready, send your message to goose

:::info
To close the search box without selecting a file, press `Esc` or click in the chat input.
:::

**Smart features:**
- **Fuzzy matching**: Intelligently matches partial text and prioritizes matches at word boundaries
- **Highlighted results**: Shows matched characters highlighted in the search results
- **Performance optimized**: Scans up to 5 directory levels deep with intelligent filtering
- **Auto-filtering**: Automatically excludes common directories like `.git`, `node_modules`, `__pycache__`, `.vscode`, `.idea`, `target`, `dist`, and `build`
- **Cross-platform**: Searches from user directories (`/Users` on macOS, `C:\Users` on Windows, `/home` on Linux)
- **Visual indicators**: Distinguishes between files and directories with clear icons

## File Management Best Practices

### Version Control

Always use a version control system like Git to track changes to your codebase. This prevents accidental overwriting and allows you to revert back to previous states easily. Ensure you commit changes before running goose on your codebase. Use branches to separate experimental changes from the main codebase.

### Validation and Testing

Implement validation and testing steps before and after goose modifies any files. Run your unit tests to verify changes made by goose. Use a staging environment to ensure changes integrate well with the entire system.

### Change Review

Manually review or use automated code reviews to ensure the quality of generated code or changes. Integrate tools such as diff tools to visualize changes made by goose. Implement a review process with team members or CI/CD pipelines.

### Codebase Organization

Structure your codebase into well-defined modules or subdirectories to manage them efficiently. Use a modular approach to isolate parts of the code goose needs to access. You can also provide specific directories or file paths you want goose to work on.
