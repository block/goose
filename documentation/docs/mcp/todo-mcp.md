---
title: Todo Extension
description: Use Todo MCP Server as a goose Extension for Task Management
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseBuiltinInstaller from '@site/src/components/GooseBuiltinInstaller';

TThe Todo extension helps goose stay organized by breaking complex work into a checklist and keeping you updated as it completes each step.

This tutorial will cover enabling and using the TODO MCP Server, which is a built-in Goose extension.

## Configuration

:::info
The Todo extension is already enabled by default when Goose is installed.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  <GooseBuiltinInstaller
    extensionName="TODO"
    description="Manage complex tasks with structured checklists and progress tracking"
  />
  </TabItem>
  <TabItem value="cli" label="goose CLI">

  1. Run the `configure` command:
  ```sh
  goose configure
  ```

  2. Choose to `Toggle Extensions`
  ```sh
  ┌   goose-configure 
  │
  ◇  What would you like to configure?
  │  Toggle Extensions 
  │
  ◆  Enable extensions: (use "space" to toggle and "enter" to submit)
  // highlight-start    
  │  ● todo
  // highlight-end  
  └  Extension settings updated successfully
  ```
  </TabItem>
</Tabs>

## Example Usage

In this example, I'll ask goose to help me add comprehensive error handling to an existing Python API. This demonstrates how the Todo extension helps goose stay organized during focused development work.

### goose Prompt

```
I have a Python FastAPI application with basic CRUD operations, but it lacks proper error handling. Please add comprehensive error handling, input validation, and logging throughout the API endpoints. The app has 5 endpoints: create user, get user, update user, delete user, and list users.
```

### goose Output

```
**Step 1: Analysis & Planning**

goose first analyzes the existing API and creates a focused improvement plan:

## FastAPI Error Handling Enhancement

### Phase 1: Code Analysis
- [ ] Review existing endpoints
  - [ ] Analyze current error handling patterns
  - [ ] Identify missing validation
  - [ ] Document current response formats
  - [ ] List potential error scenarios
- [ ] Plan error handling strategy
  - [ ] Define custom exception classes
  - [ ] Design error response format
  - [ ] Choose logging approach

### Phase 2: Custom Exceptions & Middleware
- [ ] Create custom exception classes
  - [ ] UserNotFoundError
  - [ ] ValidationError
  - [ ] DatabaseError
- [ ] Implement global exception handler
- [ ] Add request/response logging middleware

### Phase 3: Endpoint Updates
- [ ] Update user endpoints with proper error handling
  - [ ] POST /users (create user)
  - [ ] GET /users/{id} (get user)
  - [ ] PUT /users/{id} (update user)
  - [ ] DELETE /users/{id} (delete user)
  - [ ] GET /users (list users)
- [ ] Add input validation with Pydantic
- [ ] Implement proper HTTP status codes

### Phase 4: Testing & Documentation
- [ ] Add error handling tests
- [ ] Update API documentation
- [ ] Test error scenarios
- [ ] Verify logging works correctly

As goose works through the error handling implementation, it updates the Todo to show progress:

## FastAPI Error Handling Enhancement - IN PROGRESS

### Phase 1: Code Analysis ✅
- [x] Review existing endpoints
  - [x] Analyze current error handling patterns (basic try/catch only)
  - [x] Identify missing validation (no input validation on POST/PUT)
  - [x] Document current response formats (inconsistent error responses)
  - [x] List potential error scenarios (database errors, validation, not found)
- [x] Plan error handling strategy
  - [x] Define custom exception classes (3 main types identified)
  - [x] Design error response format (standardized JSON structure)
  - [x] Choose logging approach (structured logging with correlation IDs)

### Phase 2: Custom Exceptions & Middleware 🚧
- [x] Create custom exception classes
  - [x] UserNotFoundError
  - [x] ValidationError
  - [ ] DatabaseError (IN PROGRESS)
- [ ] Implement global exception handler
- [ ] Add request/response logging middleware

### Phase 3: Endpoint Updates
- [ ] Update user endpoints with proper error handling
  - [x] POST /users (create user) - added validation
  - [ ] GET /users/{id} (get user)
  - [ ] PUT /users/{id} (update user)
  - [ ] DELETE /users/{id} (delete user)
  - [ ] GET /users (list users)
- [ ] Add input validation with Pydantic
- [ ] Implement proper HTTP status codes

### Phase 4: Testing & Documentation
- [ ] Add error handling tests
- [ ] Update API documentation
- [ ] Test error scenarios
- [ ] Verify logging works correctly

### Completed This Session
- [x] Created custom exception hierarchy
- [x] Implemented UserNotFoundError and ValidationError classes
- [x] Added comprehensive validation to POST /users endpoint
- [x] Designed standardized error response format
- [x] Set up structured logging configuration

### Next Session Goals
- [ ] Complete DatabaseError exception class
- [ ] Implement global exception handler
- [ ] Update remaining 4 endpoints

**Step 3: Project Completion**

Final Todo state showing the completed error handling enhancement:

## FastAPI Error Handling Enhancement - COMPLETED ✅

### Phase 1: Code Analysis ✅
- [x] Complete analysis of existing API and error handling gaps

### Phase 2: Custom Exceptions & Middleware ✅
- [x] Created comprehensive custom exception classes
- [x] Implemented global exception handler with proper HTTP status mapping
- [x] Added request/response logging middleware with correlation IDs

### Phase 3: Endpoint Updates ✅
- [x] Updated all 5 user endpoints with proper error handling
- [x] Added comprehensive input validation with Pydantic models
- [x] Implemented consistent HTTP status codes and error responses

### Phase 4: Testing & Documentation ✅
- [x] Added 95% test coverage for error scenarios
- [x] Updated OpenAPI documentation with error response schemas
- [x] Verified logging works correctly across all endpoints

### Enhancement Results
- **Endpoints updated**: 5/5
- **Custom exceptions created**: 3
- **Test coverage**: 95% (up from 60%)
- **Error response consistency**: 100%
- **Average response time**: Improved by 15% due to better error handling

### Deliverables
- [x] Robust error handling across all API endpoints
- [x] Comprehensive test suite for error scenarios
- [x] Updated API documentation with error schemas
- [x] Structured logging with correlation tracking
- [x] Developer guide for extending error handling

```