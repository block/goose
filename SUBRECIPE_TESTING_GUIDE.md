# SubRecipe In-Process Execution Testing Guide

**Date**: 2025-01-08  
**Purpose**: Comprehensive testing guide for SubRecipe in-process execution  
**Changes**: SubRecipes now execute in-process instead of spawning CLI processes

---

## Overview

SubRecipes have been migrated from CLI process spawning to in-process execution, using the same code path as InlineRecipes. This guide covers all test cases to verify the implementation works correctly.

---

## Test Setup

### Prerequisites

1. Build goose with the new changes:
```bash
cd /path/to/goose
cargo build --release
```

2. Set up test environment:
```bash
# Create a test directory
mkdir -p ~/goose-subrecipe-tests
cd ~/goose-subrecipe-tests

# Create a recipes directory
mkdir -p recipes
```

---

## Test Categories

### 1. Basic SubRecipe Execution

#### Test 1.1: Simple SubRecipe
**Purpose**: Verify basic SubRecipe execution works

**Setup**: Create `recipes/hello.yaml`
```yaml
version: 1.0.0
title: Hello SubRecipe
description: Simple test recipe
prompt: "Say hello and return the word 'SUCCESS'"
```

**Parent Recipe**: Create `test_basic.yaml`
```yaml
version: 1.0.0
title: Test Basic SubRecipe
description: Test basic SubRecipe execution
prompt: "Execute the hello sub-recipe"
sub_recipes:
  - name: hello
    path: recipes/hello.yaml
```

**Test Command**:
```bash
goose run --recipe test_basic.yaml --no-session
```

**Expected Result**:
- ✅ SubRecipe executes without spawning a separate process
- ✅ Output contains "SUCCESS"
- ✅ No CLI process spawn messages in logs
- ✅ Execution completes successfully

---

### 2. SubRecipe with Parameters

#### Test 2.1: Required Parameters
**Purpose**: Verify parameter passing works correctly

**Setup**: Create `recipes/greet.yaml`
```yaml
version: 1.0.0
title: Greeting Recipe
description: Greet someone by name
parameters:
  - key: name
    input_type: string
    requirement: required
    description: Name to greet
prompt: "Say hello to {{ name }} and confirm you received the name correctly"
```

**Parent Recipe**: Create `test_params.yaml`
```yaml
version: 1.0.0
title: Test Parameters
description: Test SubRecipe with parameters
prompt: "Execute the greet sub-recipe with name=Alice"
sub_recipes:
  - name: greet
    path: recipes/greet.yaml
    values:
      name: Alice
```

**Test Command**:
```bash
goose run --recipe test_params.yaml --no-session
```

**Expected Result**:
- ✅ SubRecipe receives parameter "Alice"
- ✅ Output mentions "Alice"
- ✅ No parameter resolution errors

#### Test 2.2: Optional Parameters with Defaults
**Purpose**: Verify optional parameters work

**Setup**: Create `recipes/greet_optional.yaml`
```yaml
version: 1.0.0
title: Greeting with Default
description: Greet someone with optional title
parameters:
  - key: name
    input_type: string
    requirement: required
    description: Name to greet
  - key: title
    input_type: string
    requirement: optional
    description: Title (Mr, Ms, Dr, etc)
    default: Friend
prompt: "Greet {{ title }} {{ name }}"
```

**Test Cases**:

A. With both parameters:
```yaml
sub_recipes:
  - name: greet_optional
    path: recipes/greet_optional.yaml
    values:
      name: Bob
      title: Dr
```

B. With only required parameter (default should be used):
```yaml
sub_recipes:
  - name: greet_optional
    path: recipes/greet_optional.yaml
    values:
      name: Bob
```

**Expected Result**:
- ✅ Case A: Output contains "Dr Bob"
- ✅ Case B: Output contains "Friend Bob"

#### Test 2.3: Multiple Parameters
**Purpose**: Verify multiple parameter passing

**Setup**: Create `recipes/multi_param.yaml`
```yaml
version: 1.0.0
title: Multiple Parameters
description: Test multiple parameters
parameters:
  - key: param1
    input_type: string
    requirement: required
    description: First parameter
  - key: param2
    input_type: string
    requirement: required
    description: Second parameter
  - key: param3
    input_type: string
    requirement: required
    description: Third parameter
prompt: "Confirm you received: param1={{ param1 }}, param2={{ param2 }}, param3={{ param3 }}"
```

**Expected Result**:
- ✅ All three parameters are passed correctly
- ✅ Output confirms all parameter values

---

### 3. SubRecipe with Extensions

#### Test 3.1: SubRecipe with Developer Extension
**Purpose**: Verify extensions work in SubRecipes

**Setup**: Create `recipes/file_check.yaml`
```yaml
version: 1.0.0
title: File Check Recipe
description: Check if a file exists
extensions:
  - type: builtin
    name: developer
    description: Developer tools
prompt: "List the files in the current directory and confirm you can see them"
```

**Test Command**:
```bash
goose run --recipe test_file_check.yaml --no-session
```

**Expected Result**:
- ✅ Developer extension is loaded
- ✅ SubRecipe can use file system tools
- ✅ Output shows file listing

#### Test 3.2: SubRecipe with Custom Extension
**Purpose**: Verify custom extensions work

**Setup**: Create a simple MCP extension and test it in a SubRecipe

**Expected Result**:
- ✅ Custom extension loads correctly
- ✅ SubRecipe can call extension tools
- ✅ No extension initialization errors

---

### 4. Nested SubRecipes

#### Test 4.1: SubRecipe Calling SubRecipe
**Purpose**: Verify nested SubRecipe execution

**Setup**: Create three recipes:

`recipes/level3.yaml`:
```yaml
version: 1.0.0
title: Level 3
description: Deepest level
prompt: "Say 'LEVEL_3_SUCCESS' and nothing else"
```

`recipes/level2.yaml`:
```yaml
version: 1.0.0
title: Level 2
description: Middle level
prompt: "Execute the level3 sub-recipe"
sub_recipes:
  - name: level3
    path: recipes/level3.yaml
```

`test_nested.yaml`:
```yaml
version: 1.0.0
title: Nested Test
description: Test nested SubRecipes
prompt: "Execute the level2 sub-recipe"
sub_recipes:
  - name: level2
    path: recipes/level2.yaml
```

**Test Command**:
```bash
goose run --recipe test_nested.yaml --no-session
```

**Expected Result**:
- ✅ All three levels execute successfully
- ✅ Output contains "LEVEL_3_SUCCESS"
- ✅ No stack overflow or recursion errors
- ✅ All execute in-process (no CLI spawns)

---

### 5. Parallel SubRecipe Execution

#### Test 5.1: Multiple SubRecipes in Parallel
**Purpose**: Verify parallel SubRecipe execution

**Setup**: Create multiple simple recipes:

`recipes/task1.yaml`:
```yaml
version: 1.0.0
title: Task 1
description: First parallel task
prompt: "Say 'TASK_1_COMPLETE'"
```

`recipes/task2.yaml`:
```yaml
version: 1.0.0
title: Task 2
description: Second parallel task
prompt: "Say 'TASK_2_COMPLETE'"
```

`recipes/task3.yaml`:
```yaml
version: 1.0.0
title: Task 3
description: Third parallel task
prompt: "Say 'TASK_3_COMPLETE'"
```

**Test**: Use dynamic task creation to execute in parallel:
```yaml
version: 1.0.0
title: Parallel SubRecipes
description: Test parallel execution
prompt: "Execute task1, task2, and task3 sub-recipes in parallel"
sub_recipes:
  - name: task1
    path: recipes/task1.yaml
  - name: task2
    path: recipes/task2.yaml
  - name: task3
    path: recipes/task3.yaml
```

**Expected Result**:
- ✅ All three tasks execute
- ✅ Output contains all three "COMPLETE" messages
- ✅ No database locking errors
- ✅ Execution is faster than sequential

---

### 6. Error Handling

#### Test 6.1: Missing Recipe File
**Purpose**: Verify error handling for missing files

**Test**: Reference a non-existent recipe:
```yaml
version: 1.0.0
title: Missing Recipe Test
description: Test missing recipe handling
prompt: "Execute the missing sub-recipe"
sub_recipes:
  - name: missing
    path: recipes/does_not_exist.yaml
```

**Expected Result**:
- ✅ Clear error message: "Failed to load recipe file"
- ✅ Error includes the path that was not found
- ✅ No crash or panic
- ✅ Parent recipe fails gracefully

#### Test 6.2: Invalid Recipe Syntax
**Purpose**: Verify error handling for malformed recipes

**Setup**: Create `recipes/invalid.yaml`
```yaml
version: 1.0.0
title: Invalid Recipe
description: Missing required fields
# No prompt or instructions - should fail
```

**Expected Result**:
- ✅ Clear error message about missing prompt/instructions
- ✅ No crash or panic
- ✅ Parent recipe fails gracefully

#### Test 6.3: Missing Required Parameter
**Purpose**: Verify parameter validation

**Setup**: Create recipe requiring a parameter, but don't provide it:
```yaml
version: 1.0.0
title: Missing Param Test
description: Test missing parameter handling
prompt: "Execute greet without providing the name parameter"
sub_recipes:
  - name: greet
    path: recipes/greet.yaml
    # Missing required 'name' parameter
```

**Expected Result**:
- ✅ Clear error message about missing required parameter
- ✅ Error specifies which parameter is missing
- ✅ No crash or panic

#### Test 6.4: Invalid Parameter Type
**Purpose**: Verify type validation

**Setup**: Create recipe with number parameter, provide string:
```yaml
version: 1.0.0
title: Type Test
description: Test parameter type validation
parameters:
  - key: count
    input_type: number
    requirement: required
    description: A number
prompt: "The count is {{ count }}"
```

**Test**: Provide non-numeric value:
```yaml
sub_recipes:
  - name: type_test
    path: recipes/type_test.yaml
    values:
      count: "not_a_number"
```

**Expected Result**:
- ✅ Recipe attempts to use the value (MiniJinja handles conversion)
- ✅ No crash or panic

---

### 7. Performance Tests

#### Test 7.1: SubRecipe Execution Speed
**Purpose**: Verify in-process execution is faster than CLI spawn

**Setup**: Create a simple recipe and execute it multiple times

**Test Script**:
```bash
# Time the execution
time goose run --recipe test_basic.yaml --no-session
```

**Expected Result**:
- ✅ Execution completes in < 5 seconds (vs 10-30s for CLI spawn)
- ✅ No process spawn overhead visible in logs
- ✅ Memory usage is stable

#### Test 7.2: Multiple SubRecipes Performance
**Purpose**: Verify performance with many SubRecipes

**Setup**: Create 10 simple SubRecipes and execute them sequentially

**Expected Result**:
- ✅ All complete successfully
- ✅ No memory leaks
- ✅ Performance is consistent across all executions

---

### 8. Integration Tests

#### Test 8.1: SubRecipe with InlineRecipe
**Purpose**: Verify SubRecipes work alongside InlineRecipes

**Test**: Use both in the same parent recipe:
```yaml
version: 1.0.0
title: Mixed Execution
description: Test SubRecipe and InlineRecipe together
prompt: |
  1. Execute the hello sub-recipe
  2. Then create a dynamic task with instructions: "Say DYNAMIC_SUCCESS"
sub_recipes:
  - name: hello
    path: recipes/hello.yaml
```

**Expected Result**:
- ✅ SubRecipe executes successfully
- ✅ InlineRecipe (dynamic task) executes successfully
- ✅ Both use the same in-process execution path
- ✅ No conflicts or errors

#### Test 8.2: SubRecipe with Context
**Purpose**: Verify context is passed correctly

**Setup**: Create recipe with context:
```yaml
version: 1.0.0
title: Context Test
description: Test context passing
context:
  - "Important context: The answer is 42"
prompt: "What is the answer from the context?"
```

**Expected Result**:
- ✅ SubRecipe has access to context
- ✅ Output references the context
- ✅ Context is properly isolated from parent

---

### 9. Edge Cases

#### Test 9.1: Empty SubRecipe
**Purpose**: Verify handling of minimal recipes

**Setup**: Create `recipes/minimal.yaml`
```yaml
version: 1.0.0
title: Minimal
description: Minimal recipe
prompt: "OK"
```

**Expected Result**:
- ✅ Executes successfully
- ✅ No errors or warnings

#### Test 9.2: Very Long SubRecipe Output
**Purpose**: Verify handling of large outputs

**Setup**: Create recipe that generates lots of output:
```yaml
version: 1.0.0
title: Long Output
description: Generate long output
prompt: "Generate a list of 100 items numbered 1-100"
```

**Expected Result**:
- ✅ All output is captured
- ✅ No truncation errors
- ✅ Memory usage is reasonable

#### Test 9.3: SubRecipe with Special Characters in Path
**Purpose**: Verify path handling

**Setup**: Create recipe in path with spaces:
```bash
mkdir -p "recipes/my recipes"
```

Create `recipes/my recipes/special.yaml`

**Expected Result**:
- ✅ Path is handled correctly
- ✅ Recipe loads successfully
- ✅ No path parsing errors

#### Test 9.4: Circular SubRecipe Reference
**Purpose**: Verify circular reference detection

**Setup**: Create two recipes that reference each other:

`recipes/circular_a.yaml`:
```yaml
version: 1.0.0
title: Circular A
description: References B
prompt: "Execute circular_b"
sub_recipes:
  - name: circular_b
    path: recipes/circular_b.yaml
```

`recipes/circular_b.yaml`:
```yaml
version: 1.0.0
title: Circular B
description: References A
prompt: "Execute circular_a"
sub_recipes:
  - name: circular_a
    path: recipes/circular_a.yaml
```

**Expected Result**:
- ✅ Circular reference is detected OR
- ✅ Max recursion depth prevents infinite loop
- ✅ Clear error message
- ✅ No stack overflow

#### Test 9.5: SubRecipe with Unicode Characters
**Purpose**: Verify Unicode handling

**Setup**: Create `recipes/unicode.yaml`
```yaml
version: 1.0.0
title: Unicode Test 🎉
description: Test Unicode support
parameters:
  - key: emoji
    input_type: string
    requirement: required
    description: An emoji
prompt: "Echo this emoji: {{ emoji }}"
```

**Test**: Pass Unicode parameter:
```yaml
sub_recipes:
  - name: unicode
    path: recipes/unicode.yaml
    values:
      emoji: "🚀"
```

**Expected Result**:
- ✅ Unicode characters are preserved
- ✅ Output contains "🚀"
- ✅ No encoding errors

---

### 10. Regression Tests

#### Test 10.1: Verify Old Behavior Still Works
**Purpose**: Ensure InlineRecipes still work as before

**Test**: Create and execute an InlineRecipe using dynamic_task__create_task

**Expected Result**:
- ✅ InlineRecipes work exactly as before
- ✅ No behavior changes
- ✅ Same performance characteristics

#### Test 10.2: Extension Inheritance
**Purpose**: Verify extension inheritance works

**Test**: Parent with extensions, SubRecipe without:
```yaml
version: 1.0.0
title: Extension Inheritance
description: Test extension inheritance
extensions:
  - type: builtin
    name: developer
    description: Developer tools
prompt: "Execute the hello sub-recipe (which has no extensions)"
sub_recipes:
  - name: hello
    path: recipes/hello.yaml
```

**Expected Result**:
- ✅ SubRecipe inherits parent extensions OR
- ✅ SubRecipe uses its own extensions (depending on design)
- ✅ Behavior is consistent and documented

---

## Test Execution Checklist

### Quick Smoke Test (5 minutes)
- [ ] Test 1.1: Simple SubRecipe
- [ ] Test 2.1: Required Parameters
- [ ] Test 6.1: Missing Recipe File
- [ ] Test 7.1: Performance Test

### Standard Test Suite (30 minutes)
- [ ] All Basic Execution tests (1.x)
- [ ] All Parameter tests (2.x)
- [ ] All Extension tests (3.x)
- [ ] All Error Handling tests (6.x)
- [ ] Performance test (7.1)

### Comprehensive Test Suite (2 hours)
- [ ] All tests in all categories
- [ ] All edge cases
- [ ] All regression tests

---

## Expected Improvements

### Performance
- **Before (CLI Spawn)**: 10-30 seconds per SubRecipe
- **After (In-Process)**: 1-5 seconds per SubRecipe
- **Speedup**: 10-100x faster

### Debugging
- **Before**: Separate process, harder to debug
- **After**: Same process, easier to debug with logs

### Consistency
- **Before**: Different behavior than InlineRecipes
- **After**: Identical behavior to InlineRecipes

---

## Troubleshooting

### Issue: SubRecipe fails to load
**Check**:
- Recipe file exists at specified path
- Recipe file has valid YAML syntax
- Recipe has required fields (version, title, description, prompt/instructions)

### Issue: Parameters not passed correctly
**Check**:
- Parameter names match exactly (case-sensitive)
- Required parameters are provided
- Parameter types are correct

### Issue: Extensions not working
**Check**:
- Extensions are properly configured in recipe
- Extensions are available in goose installation
- Extension initialization succeeds

### Issue: Performance is slow
**Check**:
- Verify in-process execution (no CLI spawn in logs)
- Check for database locking issues
- Monitor memory usage

---

## Validation Criteria

### Must Pass
- ✅ All basic execution tests pass
- ✅ Parameter passing works correctly
- ✅ Error handling is graceful
- ✅ No crashes or panics
- ✅ Performance is significantly improved

### Should Pass
- ✅ All extension tests pass
- ✅ Nested SubRecipes work
- ✅ Parallel execution works
- ✅ Edge cases are handled

### Nice to Have
- ✅ All edge cases pass
- ✅ Unicode support works
- ✅ Performance is optimal

---

## Reporting Results

### Success Report Template
```markdown
## SubRecipe Testing Results

**Date**: YYYY-MM-DD
**Tester**: [Your Name]
**Build**: [Commit Hash]

### Summary
- Tests Run: X
- Tests Passed: Y
- Tests Failed: Z
- Performance: [Improved/Same/Degraded]

### Passed Tests
- Test 1.1: Simple SubRecipe ✅
- Test 2.1: Required Parameters ✅
- ...

### Failed Tests
- Test X.Y: [Test Name] ❌
  - Error: [Error message]
  - Expected: [Expected behavior]
  - Actual: [Actual behavior]

### Performance Results
- Average SubRecipe execution time: X seconds
- Speedup vs CLI spawn: Xx faster
- Memory usage: X MB

### Recommendations
- [Any recommendations for improvements]
```

---

## Additional Test Ideas

### Stress Tests
1. Execute 100 SubRecipes sequentially
2. Execute 10 SubRecipes in parallel
3. Deeply nested SubRecipes (10+ levels)
4. SubRecipe with very large parameters (1MB+ string)

### Security Tests
1. SubRecipe trying to access parent's secrets
2. SubRecipe with malicious parameters
3. Path traversal attempts in recipe paths

### Compatibility Tests
1. Old recipe format compatibility
2. Mixed old/new recipe execution
3. Backwards compatibility with existing tools

---

## Conclusion

This testing guide covers all major use cases, edge cases, and potential issues with SubRecipe in-process execution. Following this guide will ensure the implementation is robust, performant, and ready for production use.

**Key Success Metrics**:
- ✅ All basic tests pass
- ✅ 10x+ performance improvement
- ✅ No regressions in existing functionality
- ✅ Graceful error handling
- ✅ Consistent behavior with InlineRecipes

Good luck with testing! 🚀
