#!/usr/bin/env python3
"""
Diagnostic Log Message Categorizer & Session Reconstructor

Parses JSONL diagnostic logs from goose sessions and categorizes every
message/event into the same UI rendering zones used by the desktop app.

The classification mirrors the TypeScript pipeline:
  assistantWorkBlocks.ts → ProgressiveMessageList.tsx → GooseMessage/WorkBlockIndicator

Categories:
  MAIN PANEL (rendered directly to user):
    - USER_INPUT          Real user text → UserMessage component
    - ASSISTANT_TEXT      Final answer text → GooseMessage with markdown
    - STREAMING_CHUNK     SSE text delta → accumulated into ASSISTANT_TEXT

  WORK BLOCK (collapsed in WorkBlockIndicator, expandable in ReasoningDetailPanel):
    - TOOL_REQUEST        toolRequest content → ActivityStep in detail panel
    - TOOL_RESULT         toolResponse content → paired with request
    - INTERMEDIATE_TEXT   Assistant text preceding tool calls (thinking aloud)

  REASONING PANEL (expandable ThinkingSection):
    - THINKING            thinking/redactedThinking content

  HIDDEN (never rendered):
    - SYSTEM_INFO         info-msg timestamps injected by system
    - TITLE_GENERATION    Internal LLM call for session title
    - USAGE_STATS         Token accounting lines

Usage:
    python scripts/categorize_diagnostic_logs.py <logs_dir> [--json] [--timeline] [--validate]

Examples:
    python scripts/categorize_diagnostic_logs.py diagnostics_20260219_19/logs/
    python scripts/categorize_diagnostic_logs.py diagnostics_20260219_19/logs/ --json
    python scripts/categorize_diagnostic_logs.py diagnostics_20260219_19/logs/ --timeline
"""

import json
import sys
import os
from dataclasses import dataclass, field
from enum import Enum
from typing import Optional
from collections import Counter


# ═══════════════════════════════════════════════════════════════════════
# Categories & Zones
# ═══════════════════════════════════════════════════════════════════════

class Zone(Enum):
    MAIN_PANEL = "main_panel"
    WORK_BLOCK = "work_block"
    REASONING = "reasoning"
    HIDDEN = "hidden"


class Category(Enum):
    # Main panel
    USER_INPUT = ("user_input", Zone.MAIN_PANEL)
    ASSISTANT_TEXT = ("assistant_text", Zone.MAIN_PANEL)
    STREAMING_CHUNK = ("streaming_chunk", Zone.MAIN_PANEL)
    # Work block
    TOOL_REQUEST = ("tool_request", Zone.WORK_BLOCK)
    TOOL_RESULT = ("tool_result", Zone.WORK_BLOCK)
    INTERMEDIATE_TEXT = ("intermediate_text", Zone.WORK_BLOCK)
    # Reasoning
    THINKING = ("thinking", Zone.REASONING)
    # Hidden
    SYSTEM_INFO = ("system_info", Zone.HIDDEN)
    TITLE_GENERATION = ("title_generation", Zone.HIDDEN)
    USAGE_STATS = ("usage_stats", Zone.HIDDEN)

    def __init__(self, label, zone):
        self.label = label
        self.zone = zone


# ═══════════════════════════════════════════════════════════════════════
# Data structures
# ═══════════════════════════════════════════════════════════════════════

@dataclass
class Item:
    category: Category
    source: str        # filename:line
    role: str
    summary: str
    tool_name: Optional[str] = None
    text: Optional[str] = None
    is_streaming: bool = False
    token_usage: Optional[dict] = None

    @property
    def zone(self):
        return self.category.zone


@dataclass
class WorkBlock:
    """Mirrors the TS WorkBlock from assistantWorkBlocks.ts"""
    items: list = field(default_factory=list)
    tool_count: int = 0
    has_final_answer: bool = False
    is_streaming: bool = False

    @property
    def tool_names(self):
        names = []
        for item in self.items:
            if item.tool_name and item.tool_name not in names:
                names.append(item.tool_name)
        return names


# ═══════════════════════════════════════════════════════════════════════
# Content inspection helpers (mirror TS helpers)
# ═══════════════════════════════════════════════════════════════════════

def _normalize_content(content):
    """Ensure content is always a list of dicts."""
    if isinstance(content, str):
        return [{"type": "text", "text": content}]
    if isinstance(content, list):
        return content
    return []


def _content_types(content):
    return [c.get("type", "?") for c in content if isinstance(c, dict)]


def _is_info_msg(content):
    for c in content:
        if isinstance(c, dict) and c.get("type") == "text":
            t = c.get("text", "")
            if t.strip().startswith("<info-msg>") or "It is currently" in t[:60]:
                return True
    return False


def _has_only_tool_responses(content):
    """Mirrors TS: message.content.every(c => c.type === 'toolResponse')"""
    if not content:
        return False
    return all(
        isinstance(c, dict) and c.get("type") in ("tool_result", "toolResponse")
        for c in content
    )


def _has_tool_requests(content):
    """Mirrors TS: hasToolRequests"""
    return any(
        isinstance(c, dict) and c.get("type") in ("tool_use", "toolRequest")
        for c in content
    )


def _has_display_text(content):
    """Mirrors TS: hasDisplayText - has text content that isn't empty/info-msg"""
    for c in content:
        if isinstance(c, dict) and c.get("type") == "text":
            t = c.get("text", "").strip()
            if t and not t.startswith("<info-msg>"):
                return True
    return False


def _has_thinking(content):
    return any(
        isinstance(c, dict) and c.get("type") in ("thinking", "redactedThinking")
        for c in content
    )


def _has_tool_confirmation(content):
    return any(
        isinstance(c, dict) and c.get("type") == "toolConfirmationRequest"
        for c in content
    )


def _has_elicitation(content):
    return any(
        isinstance(c, dict)
        and c.get("type") == "actionRequired"
        and isinstance(c.get("data"), dict)
        and c["data"].get("actionType") == "elicitation"
        for c in content
    )


def _count_tool_requests(content):
    return sum(
        1 for c in content
        if isinstance(c, dict) and c.get("type") in ("tool_use", "toolRequest")
    )


def _get_tool_names(content):
    names = []
    for c in content:
        if not isinstance(c, dict):
            continue
        if c.get("type") == "tool_use":
            names.append(c.get("name", "?"))
        elif c.get("type") == "toolRequest":
            tc = c.get("toolCall", {})
            if isinstance(tc, dict):
                val = tc.get("value", {})
                if isinstance(val, dict):
                    names.append(val.get("name", "?"))
    return names


def _text_preview(content, max_len=120):
    parts = []
    for c in content:
        if isinstance(c, dict) and c.get("type") == "text":
            t = c.get("text", "").strip()
            if t and not t.startswith("<info-msg>"):
                parts.append(t)
    combined = " ".join(parts)
    return (combined[:max_len - 3] + "...") if len(combined) > max_len else combined


def _is_real_user_message(msg_idx, messages):
    """
    Mirrors TS isRealUserMessage from assistantWorkBlocks.ts.
    A "real" user message is one typed by the human, not a system-injected
    tool result following an assistant tool call.
    """
    msg = messages[msg_idx]
    if msg.get("role") != "user":
        return False

    content = _normalize_content(msg.get("content", []))

    # Pure tool responses are never real
    if _has_only_tool_responses(content):
        return False

    # Walk backwards to find preceding assistant
    for i in range(msg_idx - 1, -1, -1):
        prev = messages[i]
        if prev.get("role") == "assistant":
            prev_content = _normalize_content(prev.get("content", []))
            return not _has_tool_requests(prev_content)
        if prev.get("role") == "user":
            prev_content = _normalize_content(prev.get("content", []))
            if _has_only_tool_responses(prev_content):
                continue
            return True
    return True  # First message


# ═══════════════════════════════════════════════════════════════════════
# Work block identification (mirrors identifyWorkBlocks from TS)
# ═══════════════════════════════════════════════════════════════════════

def identify_work_blocks(messages, is_streaming_last=False):
    """
    Python port of identifyWorkBlocks() from assistantWorkBlocks.ts.
    Returns a dict mapping message index → WorkBlock.
    """
    blocks = {}

    # Find assistant runs (consecutive assistant msgs, transparent user tool-results)
    runs = []
    block_start = -1

    for i, msg in enumerate(messages):
        is_assistant = msg.get("role") == "assistant"
        if is_assistant and block_start == -1:
            block_start = i
        elif not is_assistant and block_start != -1:
            if _is_real_user_message(i, messages):
                runs.append((block_start, i - 1))
                block_start = -1

    if block_start != -1:
        runs.append((block_start, len(messages) - 1))

    for run_start, run_end in runs:
        assistant_indices = [
            i for i in range(run_start, run_end + 1)
            if messages[i].get("role") == "assistant"
        ]

        is_last_streaming = is_streaming_last and run_end == len(messages) - 1

        if len(assistant_indices) <= 1 and not is_last_streaming:
            continue

        # Find final answer
        skip_final = is_last_streaming and len(assistant_indices) > 1
        final_idx = -1

        if not skip_final:
            text_with_tools_idx = -1
            for ai in reversed(assistant_indices):
                content = _normalize_content(messages[ai].get("content", []))
                if not _has_display_text(content):
                    continue
                if _has_tool_confirmation(content) or _has_elicitation(content):
                    continue
                if not _has_tool_requests(content):
                    final_idx = ai
                    break
                elif text_with_tools_idx == -1:
                    text_with_tools_idx = ai

            if final_idx == -1 and text_with_tools_idx != -1:
                final_idx = text_with_tools_idx

        total_tools = 0
        intermediate = []
        for ai in assistant_indices:
            if ai == final_idx:
                continue
            intermediate.append(ai)
            total_tools += _count_tool_requests(
                _normalize_content(messages[ai].get("content", []))
            )

        if not intermediate:
            continue

        all_block = set()
        for i in range(run_start, run_end + 1):
            if i != final_idx:
                all_block.add(i)

        wb = WorkBlock(
            tool_count=total_tools,
            has_final_answer=(final_idx != -1),
            is_streaming=is_last_streaming,
        )

        for idx in all_block:
            blocks[idx] = wb

    return blocks


# ═══════════════════════════════════════════════════════════════════════
# Title generation detection
# ═══════════════════════════════════════════════════════════════════════

def _is_title_generation(entry):
    """Detect internal title/summary calls (not user-visible)."""
    input_data = entry.get("input", entry)
    sys_prompt = input_data.get("system", "")
    messages = input_data.get("messages", [])

    if isinstance(sys_prompt, str) and len(sys_prompt) < 200 and len(messages) <= 2:
        return True

    for msg in messages:
        content = msg.get("content", "")
        texts = []
        if isinstance(content, str):
            texts.append(content)
        elif isinstance(content, list):
            texts = [c.get("text", "") for c in content if isinstance(c, dict)]
        for t in texts:
            if "first few user messages" in t:
                return True
    return False


# ═══════════════════════════════════════════════════════════════════════
# Main categorizer
# ═══════════════════════════════════════════════════════════════════════

def categorize_input_message(msg, msg_idx, all_msgs, source, work_blocks):
    """Categorize a message from an LLM request's input."""
    role = msg.get("role", "?")
    content = _normalize_content(msg.get("content", []))
    in_work_block = msg_idx in work_blocks

    if role == "user":
        # Info-msg only
        if _is_info_msg(content) and not _has_display_text(content):
            return Item(Category.SYSTEM_INFO, source, role, "System timestamp injection")

        # Pure tool results
        if _has_only_tool_responses(content):
            return Item(
                Category.TOOL_RESULT, source, role,
                f"Tool result ({len(content)} items)",
            )

        # Real user input
        if _is_real_user_message(msg_idx, all_msgs):
            preview = _text_preview(content)
            return Item(
                Category.USER_INPUT, source, role,
                preview or "User message",
                text=preview,
            )

        # User message that's actually a tool result (after tool call)
        return Item(Category.TOOL_RESULT, source, role, "Summarized tool result")

    elif role == "assistant":
        has_text = _has_display_text(content)
        has_tools = _has_tool_requests(content)
        has_think = _has_thinking(content)

        if has_think:
            return Item(Category.THINKING, source, role, "Chain-of-thought reasoning")

        if has_tools and has_text:
            # Mixed: text + tools → if in work block, it's intermediate thinking
            tool_names = _get_tool_names(content)
            preview = _text_preview(content, 80)
            if in_work_block:
                return Item(
                    Category.INTERMEDIATE_TEXT, source, role,
                    f"Thinking: {preview}",
                    tool_name=tool_names[0] if tool_names else None,
                    text=preview,
                )
            else:
                # Final answer that also has tools (fallback case in TS)
                return Item(
                    Category.ASSISTANT_TEXT, source, role,
                    preview or "Assistant response",
                    text=preview,
                )

        if has_tools:
            tool_names = _get_tool_names(content)
            count = _count_tool_requests(content)
            return Item(
                Category.TOOL_REQUEST, source, role,
                f"{', '.join(tool_names)} ({count} call{'s' if count > 1 else ''})",
                tool_name=tool_names[0] if tool_names else None,
            )

        if has_text:
            preview = _text_preview(content)
            if in_work_block:
                return Item(
                    Category.INTERMEDIATE_TEXT, source, role,
                    f"Intermediate: {preview}",
                    text=preview,
                )
            return Item(
                Category.ASSISTANT_TEXT, source, role,
                preview or "Assistant response",
                text=preview,
            )

        return Item(Category.SYSTEM_INFO, source, role, "Empty assistant message")

    return Item(Category.SYSTEM_INFO, source, role, f"Unknown role: {role}")


def categorize_response(line_data, source, is_title_gen=False):
    """Categorize a response line."""
    items = []

    if not isinstance(line_data, dict):
        return items

    data = line_data.get("data")
    usage = line_data.get("usage")

    # Usage-only
    if data is None and usage:
        return [Item(
            Category.USAGE_STATS, source, "system",
            f"in={usage.get('input_tokens', '?')} out={usage.get('output_tokens', '?')} total={usage.get('total_tokens', '?')}",
            token_usage=usage,
        )]

    if not isinstance(data, dict):
        return items

    content = data.get("content", [])
    if not isinstance(content, list):
        return items

    for c in content:
        if not isinstance(c, dict):
            continue
        ctype = c.get("type", "?")

        if ctype == "text":
            text = c.get("text", "")
            if is_title_gen:
                items.append(Item(
                    Category.TITLE_GENERATION, source, "assistant",
                    f"Title: {text[:80]}",
                    text=text,
                ))
            else:
                items.append(Item(
                    Category.STREAMING_CHUNK, source, "assistant",
                    f"Chunk: {repr(text[:60])}",
                    text=text, is_streaming=True,
                ))

        elif ctype == "toolRequest":
            tc = c.get("toolCall", {})
            name = "?"
            if isinstance(tc, dict):
                val = tc.get("value", {})
                if isinstance(val, dict):
                    name = val.get("name", "?")
            items.append(Item(
                Category.TOOL_REQUEST, source, "assistant",
                f"Tool: {name}",
                tool_name=name,
            ))

        elif ctype in ("thinking", "redactedThinking"):
            items.append(Item(Category.THINKING, source, "assistant", "Reasoning"))

    return items or [Item(Category.USAGE_STATS, source, "system", "Response metadata")]


# ═══════════════════════════════════════════════════════════════════════
# Session reconstruction
# ═══════════════════════════════════════════════════════════════════════

def _sort_key(filename):
    base = filename.replace("llm_request.", "").replace(".jsonl", "")
    try:
        return (0, int(base))
    except ValueError:
        return (1, 0)


def load_and_categorize(logs_dir):
    """
    Load all JSONL files, reconstruct the session, and categorize.

    Strategy: Each file has the full conversation up to that point. The file
    with the most input messages has the most complete history. We use that
    file to reconstruct the conversation, then add unique responses from
    each file.
    """
    files = sorted(
        [f for f in os.listdir(logs_dir) if f.endswith('.jsonl')],
        key=_sort_key,
    )

    per_file = {}  # filename → {input_msgs, items, is_title_gen}

    for fname in files:
        path = os.path.join(logs_dir, fname)
        with open(path) as f:
            lines = [l.strip() for l in f if l.strip()]

        if not lines:
            per_file[fname] = {"input_msgs": [], "items": [], "is_title_gen": False}
            continue

        input_msgs = []
        response_items = []
        is_title_gen = False

        for line_num, raw in enumerate(lines):
            try:
                entry = json.loads(raw)
            except json.JSONDecodeError:
                continue

            # Input line
            if "model" in entry or "input" in entry:
                input_data = entry.get("input", entry)
                msgs = input_data.get("messages", [])
                is_title_gen = _is_title_generation(entry)
                input_msgs = msgs

            # Response line
            elif "data" in entry or "usage" in entry:
                source = f"{fname}:L{line_num}"
                resp = categorize_response(entry, source, is_title_gen)
                response_items.extend(resp)

        per_file[fname] = {
            "input_msgs": input_msgs,
            "items": response_items,
            "is_title_gen": is_title_gen,
        }

    # Find the most complete non-title-gen file
    best_file = None
    best_count = 0
    for fname, data in per_file.items():
        if data["is_title_gen"]:
            continue
        count = len(data["input_msgs"])
        if count > best_count:
            best_count = count
            best_file = fname

    # Categorize the canonical conversation
    conversation_items = []
    if best_file:
        msgs = per_file[best_file]["input_msgs"]
        work_blocks = identify_work_blocks(msgs)
        for idx, msg in enumerate(msgs):
            source = f"{best_file}:msg[{idx}]"
            item = categorize_input_message(msg, idx, msgs, source, work_blocks)
            conversation_items.append(item)

    # Collect all response items (each file has unique LLM output)
    all_response_items = []
    title_items = []
    for fname in files:
        data = per_file[fname]
        for item in data["items"]:
            if item.category == Category.TITLE_GENERATION:
                title_items.append(item)
            else:
                all_response_items.append(item)

    return conversation_items, all_response_items, title_items, per_file


# ═══════════════════════════════════════════════════════════════════════
# Timeline reconstruction
# ═══════════════════════════════════════════════════════════════════════

def build_timeline(conversation_items, response_items):
    """
    Build a UI-aligned timeline: interleave conversation history with
    responses, grouping work block items and accumulating streaming chunks
    into complete messages (mirroring SSE accumulation in the real UI).
    """
    timeline = []
    current_block = []

    def flush_block():
        if not current_block:
            return
        tools = sum(1 for i in current_block if i.category == Category.TOOL_REQUEST)
        results = sum(1 for i in current_block if i.category == Category.TOOL_RESULT)
        intermediate = sum(1 for i in current_block if i.category == Category.INTERMEDIATE_TEXT)
        names = []
        for i in current_block:
            if i.tool_name and i.tool_name not in names:
                names.append(i.tool_name)
        timeline.append({
            "type": "work_block",
            "tool_calls": tools,
            "tool_results": results,
            "intermediate_text": intermediate,
            "tool_names": names,
            "items": list(current_block),
        })
        current_block.clear()

    for item in conversation_items:
        if item.zone == Zone.WORK_BLOCK:
            current_block.append(item)
        elif item.zone == Zone.HIDDEN:
            continue
        else:
            flush_block()
            timeline.append({"type": "message", "item": item})

    flush_block()

    # Process response items: accumulate streaming chunks into a single
    # assembled message (mirrors SSE accumulation in the real UI)
    streaming_chunks = []
    response_block = []

    def flush_streaming():
        if not streaming_chunks:
            return
        full_text = "".join(c.text or "" for c in streaming_chunks)
        timeline.append({
            "type": "message",
            "item": Item(
                Category.ASSISTANT_TEXT, streaming_chunks[0].source, "assistant",
                full_text[:120] + ("..." if len(full_text) > 120 else ""),
                text=full_text,
            ),
            "chunk_count": len(streaming_chunks),
        })
        streaming_chunks.clear()

    def flush_response_block():
        if not response_block:
            return
        tools = sum(1 for i in response_block if i.category == Category.TOOL_REQUEST)
        names = [i.tool_name for i in response_block if i.tool_name]
        timeline.append({
            "type": "work_block",
            "tool_calls": tools,
            "tool_results": 0,
            "intermediate_text": 0,
            "tool_names": list(dict.fromkeys(names)),
            "items": list(response_block),
        })
        response_block.clear()

    for item in response_items:
        if item.category == Category.STREAMING_CHUNK:
            if response_block:
                flush_response_block()
            streaming_chunks.append(item)
        elif item.zone == Zone.MAIN_PANEL:
            flush_streaming()
            if response_block:
                flush_response_block()
            timeline.append({"type": "message", "item": item})
        elif item.zone == Zone.WORK_BLOCK:
            flush_streaming()
            response_block.append(item)

    flush_streaming()
    flush_response_block()

    return timeline


# ═══════════════════════════════════════════════════════════════════════
# Validation
# ═══════════════════════════════════════════════════════════════════════

def validate(conversation_items, per_file):
    """
    Validate categorization against the TS work block algorithm.
    Checks that our Python identify_work_blocks matches expected behavior.
    """
    print("=" * 70)
    print("VALIDATION: Python vs TypeScript work block algorithm")
    print("=" * 70)
    print()

    # Pick the most complete file
    best_file = None
    best_count = 0
    for fname, data in per_file.items():
        if data["is_title_gen"]:
            continue
        count = len(data["input_msgs"])
        if count > best_count:
            best_count = count
            best_file = fname

    if not best_file:
        print("  No conversation data to validate.")
        return

    msgs = per_file[best_file]["input_msgs"]
    work_blocks = identify_work_blocks(msgs)

    print(f"  Source: {best_file} ({len(msgs)} messages)")
    print(f"  Work block indices: {sorted(work_blocks.keys())}")
    print(f"  Non-block indices:  {sorted(set(range(len(msgs))) - set(work_blocks.keys()))}")
    print()

    # Check each message
    issues = []
    for idx, msg in enumerate(msgs):
        role = msg.get("role", "?")
        content = _normalize_content(msg.get("content", []))
        ctypes = _content_types(content)
        in_block = idx in work_blocks
        is_real = _is_real_user_message(idx, msgs) if role == "user" else None

        status = "BLOCK" if in_block else "VISIBLE"
        extra = f" (real={is_real})" if is_real is not None else ""
        print(f"  [{idx:2d}] {status:7s} {role:10s} types={ctypes}{extra}")

        # Sanity checks
        if role == "user" and is_real and in_block:
            issues.append(f"  ⚠️  msg[{idx}]: Real user message in work block!")
        if role == "assistant" and not in_block and _has_tool_requests(content) and not _has_display_text(content):
            issues.append(f"  ⚠️  msg[{idx}]: Tool-only assistant not in work block!")

    if issues:
        print()
        print("Issues found:")
        for issue in issues:
            print(issue)
    else:
        print()
        print("  ✅ All categorizations consistent with TS algorithm")


# ═══════════════════════════════════════════════════════════════════════
# Output formatters
# ═══════════════════════════════════════════════════════════════════════

def print_report(conversation_items, response_items, title_items):
    all_items = conversation_items + response_items + title_items
    total = len(all_items)

    print("=" * 70)
    print("DIAGNOSTIC LOG CATEGORIZATION REPORT")
    print("=" * 70)
    print()

    # Category counts
    counts = Counter(item.category for item in all_items)
    print("Category Breakdown:")
    print("-" * 55)
    for cat in Category:
        c = counts.get(cat, 0)
        if c > 0:
            print(f"  {cat.label:25s}  {c:4d}  [{cat.zone.value}]")

    # Zone summary
    print()
    zone_counts = Counter(item.zone for item in all_items)
    icons = {
        Zone.MAIN_PANEL: "📋",
        Zone.WORK_BLOCK: "⚙️ ",
        Zone.REASONING: "🧠",
        Zone.HIDDEN: "👻",
    }
    print("UI Zone Summary:")
    print("-" * 45)
    for zone in Zone:
        c = zone_counts.get(zone, 0)
        pct = 100 * c / total if total else 0
        print(f"  {icons[zone]} {zone.value:20s}  {c:4d}  ({pct:.1f}%)")

    print()
    print(f"Total items: {total}")
    print(f"  Conversation history: {len(conversation_items)}")
    print(f"  LLM responses:       {len(response_items)}")
    print(f"  Title generation:    {len(title_items)}")


def print_timeline(timeline, title_items):
    print()
    print("=" * 70)
    print("SESSION TIMELINE (as rendered in UI)")
    print("=" * 70)
    print()

    for entry in timeline:
        if entry["type"] == "message":
            item = entry["item"]
            if item.category == Category.USER_INPUT:
                print(f"  👤 {item.summary}")
            elif item.category == Category.ASSISTANT_TEXT:
                print(f"  🤖 {item.summary}")
            elif item.category == Category.STREAMING_CHUNK:
                print(f"  🤖 ···{item.text}···" if item.text else f"  🤖 {item.summary}")
            elif item.category == Category.THINKING:
                print(f"  🧠 {item.summary}")

        elif entry["type"] == "work_block":
            tools = entry["tool_calls"]
            results = entry["tool_results"]
            intermediate = entry["intermediate_text"]
            names = entry["tool_names"]

            parts = []
            if tools:
                parts.append(f"{tools} tool call{'s' if tools != 1 else ''}")
            if results:
                parts.append(f"{results} result{'s' if results != 1 else ''}")
            if intermediate:
                parts.append(f"{intermediate} intermediate")

            names_str = f" [{', '.join(names)}]" if names else ""
            print(f"  ⚙️  WORK BLOCK: {', '.join(parts)}{names_str}")

    if title_items:
        print()
        print("  📝 Internal: ", end="")
        for item in title_items:
            print(item.summary)


def print_json(conversation_items, response_items, title_items, timeline):
    output = {
        "summary": {
            "total": len(conversation_items) + len(response_items) + len(title_items),
            "conversation": len(conversation_items),
            "responses": len(response_items),
            "title_gen": len(title_items),
        },
        "zones": {},
        "categories": {},
        "timeline": [],
    }

    all_items = conversation_items + response_items + title_items
    for item in all_items:
        z = item.zone.value
        output["zones"][z] = output["zones"].get(z, 0) + 1
        c = item.category.label
        output["categories"][c] = output["categories"].get(c, 0) + 1

    for entry in timeline:
        if entry["type"] == "message":
            item = entry["item"]
            output["timeline"].append({
                "type": "message",
                "category": item.category.label,
                "zone": item.zone.value,
                "role": item.role,
                "summary": item.summary,
                "tool_name": item.tool_name,
                "text": item.text,
            })
        elif entry["type"] == "work_block":
            output["timeline"].append({
                "type": "work_block",
                "tool_calls": entry["tool_calls"],
                "tool_results": entry["tool_results"],
                "intermediate_text": entry["intermediate_text"],
                "tool_names": entry["tool_names"],
            })

    print(json.dumps(output, indent=2))


# ═══════════════════════════════════════════════════════════════════════
# Entry point
# ═══════════════════════════════════════════════════════════════════════

def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    logs_dir = sys.argv[1]
    do_json = "--json" in sys.argv
    do_timeline = "--timeline" in sys.argv
    do_validate = "--validate" in sys.argv

    if not os.path.isdir(logs_dir):
        print(f"Error: {logs_dir} is not a directory", file=sys.stderr)
        sys.exit(1)

    conversation_items, response_items, title_items, per_file = load_and_categorize(logs_dir)

    if not conversation_items and not response_items:
        print("No data found.", file=sys.stderr)
        sys.exit(1)

    timeline = build_timeline(conversation_items, response_items)

    if do_json:
        print_json(conversation_items, response_items, title_items, timeline)
    else:
        print_report(conversation_items, response_items, title_items)
        if do_timeline or not do_validate:
            print_timeline(timeline, title_items)
        if do_validate:
            print()
            validate(conversation_items, per_file)


if __name__ == "__main__":
    main()
