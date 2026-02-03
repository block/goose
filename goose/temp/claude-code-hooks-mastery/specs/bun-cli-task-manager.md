# Plan: Bun CLI Task Manager with SQLite Storage

## Task Description
Build a command-line task manager application using Bun runtime with SQLite as the persistent storage backend. The application will support full CRUD operations for tasks (add, list, complete, delete), priority levels (low/medium/high), due dates with validation, colored terminal output using chalk, and a statistics command showing task completion rates. The implementation must include comprehensive error handling and input validation throughout.

## Objective
Deliver a fully functional, production-ready CLI task manager that demonstrates best practices in TypeScript development with Bun, SQLite database operations, CLI argument parsing, and terminal UI presentation. Upon completion, users will be able to manage their tasks efficiently from the command line with visual feedback and meaningful statistics.

## Problem Statement
Users need a lightweight, fast CLI tool to manage tasks with priorities and due dates. Existing solutions are often overly complex, slow to start, or lack essential features like completion statistics. A Bun-based solution offers near-instant startup times and native SQLite support, making it ideal for a productivity tool that users interact with frequently throughout their workday.

## Solution Approach
Build a modular CLI application with clear separation of concerns:
1. **Database Layer**: Use Bun's native SQLite driver (`bun:sqlite`) for zero-dependency persistence
2. **CLI Layer**: Use a lightweight argument parser (yargs or commander) for command handling
3. **Presentation Layer**: Use chalk for colored output with semantic meaning (red=high priority, yellow=medium, green=low)
4. **Validation Layer**: Centralized input validation for dates, priorities, and required fields
5. **Statistics Engine**: Calculate and display completion rates, overdue tasks, and priority distribution

## Relevant Files
Use these files to complete the task:

- `apps/` - Target directory for the new CLI application (existing directory in project)
- `.gitignore` - May need updates for SQLite database files

### New Files to Create

- `apps/task-manager/src/index.ts` - Main entry point and CLI command router
- `apps/task-manager/src/db/database.ts` - SQLite database initialization and connection
- `apps/task-manager/src/db/migrations.ts` - Database schema migrations
- `apps/task-manager/src/db/repository.ts` - Task CRUD operations (repository pattern)
- `apps/task-manager/src/commands/add.ts` - Add task command handler
- `apps/task-manager/src/commands/list.ts` - List tasks command handler
- `apps/task-manager/src/commands/complete.ts` - Complete task command handler
- `apps/task-manager/src/commands/delete.ts` - Delete task command handler
- `apps/task-manager/src/commands/stats.ts` - Statistics command handler
- `apps/task-manager/src/utils/validation.ts` - Input validation utilities
- `apps/task-manager/src/utils/formatter.ts` - Output formatting with chalk
- `apps/task-manager/src/types.ts` - TypeScript interfaces and types
- `apps/task-manager/package.json` - Project dependencies and scripts
- `apps/task-manager/tsconfig.json` - TypeScript configuration for Bun
- `apps/task-manager/README.md` - Usage documentation

## Implementation Phases

### Phase 1: Foundation
- Initialize Bun project with TypeScript configuration
- Set up SQLite database with proper schema
- Create base types and interfaces
- Implement database repository layer with CRUD operations
- Set up basic CLI structure

### Phase 2: Core Implementation
- Implement all command handlers (add, list, complete, delete, stats)
- Add input validation for all user inputs
- Implement chalk-based colored output
- Add proper error handling with user-friendly messages
- Implement date parsing and validation for due dates

### Phase 3: Integration & Polish
- Integration testing of all commands
- Edge case handling (empty database, invalid IDs, etc.)
- Performance optimization for large task lists
- Final validation and documentation

## Team Orchestration

- You operate as the team lead and orchestrate the team to execute the plan.
- You're responsible for deploying the right team members with the right context to execute the plan.
- IMPORTANT: You NEVER operate directly on the codebase. You use `Task` and `Task*` tools to deploy team members to do the building, validating, testing, deploying, and other tasks.
- Communication is paramount. You'll use the Task* Tools to communicate with the team members and ensure they're on track to complete the plan.
- Take note of the session id of each team member. This is how you'll reference them.

### Team Members

- Builder
  - Name: foundation-builder
  - Role: Project initialization, database setup, and core infrastructure
  - Agent Type: general-purpose
  - Resume: true

- Builder
  - Name: command-builder
  - Role: Implement CLI commands (add, list, complete, delete, stats)
  - Agent Type: general-purpose
  - Resume: true

- Builder
  - Name: ui-builder
  - Role: Implement chalk formatting, validation utilities, and polished output
  - Agent Type: general-purpose
  - Resume: true

- Validator
  - Name: integration-validator
  - Role: Test all commands, verify acceptance criteria, validate edge cases
  - Agent Type: general-purpose
  - Resume: false

## Step by Step Tasks

- IMPORTANT: Execute every step in order, top to bottom. Each task maps directly to a `TaskCreate` call.
- Before you start, run `TaskCreate` to create the initial task list that all team members can see and execute.

### 1. Initialize Project Structure
- **Task ID**: init-project
- **Depends On**: none
- **Assigned To**: foundation-builder
- **Agent Type**: general-purpose
- **Parallel**: false
- Create `apps/task-manager/` directory structure
- Initialize `package.json` with Bun configuration and dependencies (chalk, yargs)
- Create `tsconfig.json` configured for Bun runtime
- Set up `.gitignore` entries for SQLite database files (`*.db`, `*.sqlite`)

### 2. Create TypeScript Types and Interfaces
- **Task ID**: create-types
- **Depends On**: init-project
- **Assigned To**: foundation-builder
- **Agent Type**: general-purpose
- **Parallel**: false
- Define `Task` interface with fields: id, title, description, priority (enum), dueDate, completed, createdAt, completedAt
- Define `Priority` enum: LOW, MEDIUM, HIGH
- Define `TaskStats` interface for statistics data
- Define CLI argument types for each command

### 3. Implement Database Layer
- **Task ID**: impl-database
- **Depends On**: create-types
- **Assigned To**: foundation-builder
- **Agent Type**: general-purpose
- **Parallel**: false
- Create database initialization using `bun:sqlite`
- Implement schema migration with tasks table (id INTEGER PRIMARY KEY, title TEXT NOT NULL, description TEXT, priority TEXT, due_date TEXT, completed INTEGER DEFAULT 0, created_at TEXT, completed_at TEXT)
- Implement repository pattern with methods: create, findAll, findById, update, delete, getStats
- Add proper error handling for database operations

### 4. Implement Validation Utilities
- **Task ID**: impl-validation
- **Depends On**: create-types
- **Assigned To**: ui-builder
- **Agent Type**: general-purpose
- **Parallel**: true
- Create date validation (must be valid date format, optionally in future)
- Create priority validation (must be low/medium/high, case-insensitive)
- Create title validation (required, non-empty, reasonable length limit)
- Create ID validation (must be positive integer)
- Return structured validation results with error messages

### 5. Implement Output Formatter with Chalk
- **Task ID**: impl-formatter
- **Depends On**: create-types
- **Assigned To**: ui-builder
- **Agent Type**: general-purpose
- **Parallel**: true
- Create color scheme: HIGH priority = red, MEDIUM = yellow, LOW = green
- Implement task list formatter with aligned columns
- Implement success/error message formatters
- Implement stats display formatter with visual bars/percentages
- Add overdue task highlighting (tasks past due date)

### 6. Implement Add Command
- **Task ID**: impl-add-cmd
- **Depends On**: impl-database, impl-validation, impl-formatter
- **Assigned To**: command-builder
- **Agent Type**: general-purpose
- **Parallel**: false
- Parse arguments: --title (required), --description (optional), --priority (default: medium), --due (optional date)
- Validate all inputs using validation utilities
- Create task in database via repository
- Display success message with created task details (colored by priority)
- Handle and display validation/database errors gracefully

### 7. Implement List Command
- **Task ID**: impl-list-cmd
- **Depends On**: impl-database, impl-formatter
- **Assigned To**: command-builder
- **Agent Type**: general-purpose
- **Parallel**: false
- Parse optional filter arguments: --priority, --completed, --pending, --overdue
- Fetch tasks from database with applied filters
- Format and display tasks using chalk formatter
- Show helpful message if no tasks found
- Display count summary at bottom

### 8. Implement Complete Command
- **Task ID**: impl-complete-cmd
- **Depends On**: impl-database, impl-validation, impl-formatter
- **Assigned To**: command-builder
- **Agent Type**: general-purpose
- **Parallel**: false
- Parse argument: task ID (required, positional or --id flag)
- Validate ID exists in database
- Update task: set completed=true, completedAt=current timestamp
- Display success message with completed task
- Handle "task not found" error gracefully

### 9. Implement Delete Command
- **Task ID**: impl-delete-cmd
- **Depends On**: impl-database, impl-validation, impl-formatter
- **Assigned To**: command-builder
- **Agent Type**: general-purpose
- **Parallel**: false
- Parse argument: task ID (required)
- Validate ID exists in database
- Implement optional --force flag to skip confirmation
- Delete task from database
- Display success message
- Handle "task not found" error gracefully

### 10. Implement Stats Command
- **Task ID**: impl-stats-cmd
- **Depends On**: impl-database, impl-formatter
- **Assigned To**: command-builder
- **Agent Type**: general-purpose
- **Parallel**: false
- Fetch statistics from repository: total tasks, completed, pending, overdue
- Calculate completion rate percentage
- Calculate priority distribution (count per priority level)
- Display formatted statistics with colors and visual indicators
- Handle empty database case gracefully

### 11. Implement Main CLI Entry Point
- **Task ID**: impl-main-cli
- **Depends On**: impl-add-cmd, impl-list-cmd, impl-complete-cmd, impl-delete-cmd, impl-stats-cmd
- **Assigned To**: command-builder
- **Agent Type**: general-purpose
- **Parallel**: false
- Set up yargs with command routing
- Register all commands with their handlers
- Add --help with usage examples
- Add --version command
- Set up global error handler for unhandled exceptions
- Configure shebang for direct execution

### 12. Final Integration Validation
- **Task ID**: validate-all
- **Depends On**: impl-main-cli
- **Assigned To**: integration-validator
- **Agent Type**: general-purpose
- **Parallel**: false
- Run TypeScript compilation check
- Test add command with various inputs (valid, invalid, edge cases)
- Test list command with filters
- Test complete command (valid ID, invalid ID)
- Test delete command (valid ID, invalid ID)
- Test stats command (empty DB, populated DB)
- Verify colored output displays correctly
- Verify all error messages are user-friendly
- Confirm all acceptance criteria are met

## Acceptance Criteria

1. **Add Command**: Successfully adds tasks with title, optional description, priority (low/medium/high), and optional due date
2. **List Command**: Displays all tasks with colored priority indicators; supports filtering by status and priority
3. **Complete Command**: Marks a task as completed by ID with timestamp
4. **Delete Command**: Removes a task by ID with confirmation
5. **Stats Command**: Shows completion rate, task counts by status, priority distribution, and overdue count
6. **Priority Colors**: HIGH=red, MEDIUM=yellow, LOW=green consistently throughout the app
7. **Due Dates**: Validates date format, highlights overdue tasks
8. **Error Handling**: All invalid inputs produce clear, helpful error messages (not stack traces)
9. **Input Validation**: Empty titles rejected, invalid priorities rejected, invalid dates rejected, invalid IDs rejected
10. **Persistence**: Tasks persist across CLI invocations via SQLite
11. **Zero Dependencies Issues**: Application runs with `bun run` without errors

## Validation Commands
Execute these commands to validate the task is complete:

- `cd apps/task-manager && bun install` - Install dependencies
- `cd apps/task-manager && bun run build` - Verify TypeScript compiles (if build script exists)
- `cd apps/task-manager && bun run src/index.ts --help` - Verify CLI help displays
- `cd apps/task-manager && bun run src/index.ts add --title "Test Task" --priority high --due "2025-12-31"` - Test add command
- `cd apps/task-manager && bun run src/index.ts list` - Test list command shows added task
- `cd apps/task-manager && bun run src/index.ts complete 1` - Test complete command
- `cd apps/task-manager && bun run src/index.ts stats` - Test stats command
- `cd apps/task-manager && bun run src/index.ts delete 1 --force` - Test delete command
- `cd apps/task-manager && bun run src/index.ts add --title ""` - Test validation rejects empty title
- `cd apps/task-manager && bun run src/index.ts add --title "Test" --priority invalid` - Test validation rejects invalid priority

## Notes

- **Bun SQLite**: Use `bun:sqlite` built-in module - no external SQLite package needed
- **Dependencies to install**:
  - `bun add chalk` - For colored terminal output (use chalk v5+ for ESM)
  - `bun add yargs` - For CLI argument parsing
  - `bun add -d @types/yargs` - TypeScript types for yargs
- **Database Location**: Store SQLite database at `~/.task-manager/tasks.db` for user persistence, or locally at `./data/tasks.db` for development
- **Date Format**: Accept ISO format (YYYY-MM-DD) for due dates
- **Chalk v5**: Note that chalk v5+ is ESM-only, which works well with Bun's native ESM support
