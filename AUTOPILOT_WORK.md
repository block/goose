# AutoPilot Flexible Model Switching System

## Overview
Enhanced the existing AutoPilot system to support flexible, rule-based model switching with pre-defined roles and customizable triggers. The system automatically switches between different AI models based on conversation context, user input patterns, and machine activity.

## Files Created/Modified

### Core Implementation
- **`crates/goose/src/agents/autopilot_flexible.rs`** (NEW)
  - Main implementation of the flexible autopilot system
  - Contains trigger evaluation logic, model switching logic, and state management
  - ~900 lines including comprehensive tests

- **`crates/goose/src/agents/premade_roles.yaml`** (NEW)
  - Pre-defined role configurations with default trigger rules
  - Provider/model agnostic - only defines behaviors
  - Includes 13 pre-made roles including backward-compatible "oracle" and "second-opinion"

### Integration Changes
- **`crates/goose/src/agents/agent.rs`** (MODIFIED)
  - Updated to use `autopilot_flexible` instead of original `autopilot`
  - Added event emission for model switches (`AgentEvent::ModelChange`)
  - Added user-friendly messages when models switch
  - Fixed typo: `max_turn_turn` → `max_turns`

- **`crates/goose/src/agents/mod.rs`** (MODIFIED)
  - Removed old `autopilot` module
  - Added `autopilot_flexible` module

### Removed Files
- **`crates/goose/src/agents/autopilot.rs`** (DELETED)
  - Original hardcoded implementation with only "oracle" and "second-opinion" roles

## Design Intent

### 1. Separation of Concerns
- **Pre-made roles** define default behaviors (triggers, cooldowns, priorities)
- **User configuration** specifies which provider/model to use for each role
- Users can override default rules if needed

### 2. Flexible Trigger System
Multiple trigger types can be combined:
- **Keywords**: Match specific words with "any" or "all" logic
- **Source awareness**: Differentiate human vs machine triggers
- **Tool failures**: Detect actual failures via `ToolResult::is_err()`
- **Autonomous work monitoring**:
  - `consecutive_tools`: N+ tools used in sequence
  - `tools_since_human`: Total tools since last human input
  - `messages_since_human`: Messages generated since human input
  - `machine_messages_without_human`: Consecutive machine-only messages
- **Complexity analysis**: Simple text analysis for query complexity

### 3. Control Mechanisms
- **Cooldown**: Prevent rapid switching between models
- **Max invocations**: Limit expensive model usage
- **Priority system**: Resolve conflicts when multiple models match

### 4. User Experience
- Visual notifications when switching (🚀 for role switch, 🔄 for return)
- `AgentEvent::ModelChange` events for programmatic handling
- User-friendly role descriptions in messages

## Pre-made Roles

1. **deep-thinker**: Complex reasoning and analysis
2. **debugger**: Error recovery and debugging
3. **coder**: Code implementation
4. **reviewer**: Code/work review
5. **helper**: General assistance
6. **mathematician**: Mathematical calculations
7. **creative**: Creative brainstorming
8. **quick-responder**: Simple queries
9. **researcher**: Research and fact-checking
10. **recovery-specialist**: System recovery after failures
11. **work-reviewer**: Reviews after autonomous work
12. **progress-checker**: Monitors progress during extended work
13. **intensive-work-monitor**: Monitors intensive tool usage
14. **oracle**: Backward compatible - triggers on "think"
15. **second-opinion**: Backward compatible - triggers on "help"

## Configuration Example

```yaml
# User's config.yaml
models:
  # Basic usage - inherit default rules
  - provider: "openai"
    model: "o1-preview"
    role: "deep-thinker"
    
  # Override rules for a role
  - provider: "anthropic"
    model: "claude-3-5-sonnet"
    role: "helper"
    rules:  # Custom rules override defaults
      triggers:
        keywords: ["help", "please"]
        match_type: "any"
      cooldown_turns: 2
      priority: 10
      
  # Monitor autonomous work
  - provider: "openai"
    model: "gpt-4o"
    role: "work-reviewer"
    # Inherits: triggers after 5+ tools since human input
```

## Tests

### Location
`crates/goose/src/agents/autopilot_flexible.rs` (in `#[cfg(test)]` module)

### Test Coverage
- **test_keyword_matching_any**: Keyword matching with "any" logic ✅
- **test_keyword_matching_all**: Keyword matching with "all" logic ✅
- **test_complexity_analysis**: Text complexity detection ⚠️
- **test_config_merging**: User config + premade rules merging ✅
- **test_cooldown_mechanism**: Cooldown period enforcement ✅
- **test_source_filtering**: Human vs machine source detection ⚠️
- **test_consecutive_failures_trigger**: Consecutive failure detection ⚠️
- **test_tool_failure_detection**: Tool failure detection ⚠️
- **test_premade_rules_loading**: Loading embedded YAML ✅

Note: Some tests (⚠️) need adjustment due to conversation validation constraints.

### Running Tests
```bash
cargo test -p goose autopilot_flexible::tests --lib
```

## Key Design Decisions

1. **Pre-made roles are provider-agnostic**: Users choose the model, system provides the behavior
2. **Rules are optional in user config**: Inherit from pre-made or customize
3. **Tool failures are properly typed**: No heuristics needed - uses `Result` type
4. **Backward compatible**: Includes original "oracle" and "second-opinion" roles
5. **Transparent to users**: Shows switching messages in conversation
6. **Extensible**: Easy to add new trigger types or roles

## Future Enhancements

- Regex patterns for more sophisticated text matching
- Token counting triggers for long conversations
- Performance metrics to adjust priorities dynamically
- Model chaining (one model explicitly requests another)
- User preferences for model selection
- Context-aware triggers that look at full conversation history
