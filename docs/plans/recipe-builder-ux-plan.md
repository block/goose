# Recipe Builder UX Plan

## Overview

Design a recipe creation and editing experience for non-engineer users that:
- Makes recipes discoverable and understandable
- Supports users from beginner to power user
- Respects users by teaching, not hiding complexity

---

## Phase 1: Discovery

**Goal:** User goes from "I don't know recipes exist" → "Oh, this could help me"

### 1.1 Entry Points

| Path | Trigger | User Intent |
|------|---------|-------------|
| Passive Browse | User clicks "Recipes" in sidebar | Low (exploring) |
| In-Session Prompt | AI detects detailed instructions in chat | High (just did the work) |
| Explicit Request | User says "save this" or clicks button | Highest (knows what they want) |

### 1.2 In-Session "Save as Recipe" Flow

**Trigger:** User gives detailed instructions during a chat session.

**AI Response:**
```
"Want to reuse these instructions later? Save as a Recipe -
next time just pick it and I'll already know your preferences."

[Save as Recipe]  [Maybe Later]
```

**Save Dialog:**

```
┌─────────────────────────────────────────────────────────────┐
│ Create Recipe from This Chat                                │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│ Name: [_________________________________]                   │
│                                                             │
│ ┌───────────────────────────────────────────────────────┐   │
│ │ What it does:                                         │   │
│ │ • [extracted behavior 1]                              │   │
│ │ • [extracted behavior 2]                              │   │
│ │ • [extracted behavior 3]                              │   │
│ └───────────────────────────────────────────────────────┘   │
│                                                             │
│ Preview - how it will start:                                │
│ ┌───────────────────────────────────────────────────────┐   │
│ │ AI: "[auto-generated first message]"                  │   │
│ │                                                       │   │
│ │ Try it (optional):                                    │   │
│ │ [_________________________________] [→]               │   │
│ └───────────────────────────────────────────────────────┘   │
│                                                             │
│ [Save]    [Edit]    [Cancel]                                │
└─────────────────────────────────────────────────────────────┘
```

**Key Features:**
- AI extracts recipe from conversation (user doesn't fill a form)
- Auto-preview: AI generates what the first message would be
- Optional one-turn test: User can type one message to verify behavior
- Edit button leads to full edit UI

### 1.3 Verification Strategy

| Method | User Effort | Confidence |
|--------|-------------|------------|
| Auto-preview (first message) | None | Medium |
| One-turn test (optional) | 10 seconds | High |
| Full test chat | Minutes | Highest |

**Default:** Auto-preview shown automatically. One-turn test available but optional.

### 1.4 Edit UI

**Design Principles:**
1. Show all fields (don't hide complexity)
2. Human-friendly names + explanations (translate, don't hide)
3. Users learn by using → can become power users
4. "View as YAML" escape hatch for power users

**Field Mapping:**

| Technical Field | Human Name | Explanation |
|-----------------|------------|-------------|
| `title` | Name | — |
| `description` | Short description | — |
| `instructions` | Behavior rules | "How the AI should act" |
| `prompt` | Opening message | "What AI says first when you start a chat" |
| `activities` | Capabilities | "What the AI can do" |
| `parameters` | Settings | "Extra configuration for this recipe" |

**Capabilities Mapping:**

| Technical Value | Human Label | Description |
|-----------------|-------------|-------------|
| `developer` | Run code and commands | Execute scripts and terminal commands |
| `computercontroller` | Control your computer | Click, type, and interact with apps |
| `web_search` | Search the web | Look up information online |
| `read_files` | Read your files | Access documents in your project |

**Edit UI Layout:**

```
┌─────────────────────────────────────────────────────────────┐
│ Edit Recipe                                                 │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│ Name                                                        │
│ [_____________________________________________]             │
│                                                             │
│ Short description                                           │
│ [_____________________________________________]             │
│                                                             │
│ ┌───────────────────────────────────────────────────────┐   │
│ │ Behavior rules                                        │   │
│ │ How the AI should act                                 │   │
│ │                                                       │   │
│ │ • [rule 1]                                       [×]  │   │
│ │ • [rule 2]                                       [×]  │   │
│ │ • [rule 3]                                       [×]  │   │
│ │                                                       │   │
│ │ [+ Add rule]                                          │   │
│ └───────────────────────────────────────────────────────┘   │
│                                                             │
│ ┌───────────────────────────────────────────────────────┐   │
│ │ Opening message                                       │   │
│ │ What AI says first when you start a chat              │   │
│ │                                                       │   │
│ │ [_____________________________________________]       │   │
│ │                                                       │   │
│ │ Leave empty for AI to decide                          │   │
│ └───────────────────────────────────────────────────────┘   │
│                                                             │
│ ┌───────────────────────────────────────────────────────┐   │
│ │ Capabilities                              [View YAML] │   │
│ │ What the AI can do                                    │   │
│ │                                                       │   │
│ │ ☐ Read your files                                     │   │
│ │   Access documents in your project                    │   │
│ │                                                       │   │
│ │ ☐ Run code and commands                               │   │
│ │   Execute scripts and terminal commands               │   │
│ │                                                       │   │
│ │ ☐ Control your computer                               │   │
│ │   Click, type, and interact with apps                 │   │
│ │                                                       │   │
│ │ ☐ Search the web                                      │   │
│ │   Look up information online                          │   │
│ └───────────────────────────────────────────────────────┘   │
│                                                             │
│ ┌───────────────────────────────────────────────────────┐   │
│ │ Settings (optional)                                   │   │
│ │ Extra configuration for this recipe                   │   │
│ │                                                       │   │
│ │ No settings configured                                │   │
│ │ [+ Add setting]                                       │   │
│ └───────────────────────────────────────────────────────┘   │
│                                                             │
│ [Save]    [Test]                                [Cancel]    │
└─────────────────────────────────────────────────────────────┘
```

### 1.5 User Growth Path

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  Beginner                                                   │
│  • Sees human-friendly labels                               │
│  • Reads explanations                                       │
│  • Learns what each field does by using                     │
│                                                             │
│           ↓                                                 │
│                                                             │
│  Familiar User                                              │
│  • Understands the fields                                   │
│  • Skips reading explanations                               │
│  • Edits confidently                                        │
│                                                             │
│           ↓                                                 │
│                                                             │
│  Power User                                                 │
│  • Clicks "View YAML"                                       │
│  • Edits raw format directly                                │
│  • Understands technical field names                        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

**Key Principle:** The UI is the teacher. Users graduate themselves.

---

## Phase 2: Intent

*To be designed: User clicks "Create Your Own" from Recipes page*

---

## Phase 3: Conversation

*To be designed: AI guides user through recipe creation*

---

## Phase 4: Preview

*To be designed: User sees behavior summary*

---

## Phase 5: Test

*To be designed: User tries recipe in sandbox*

---

## Phase 6: Save

*To be designed: Final save and naming*

---

## Phase 7: Daily Use

*To be designed: Using recipes in regular workflow*
