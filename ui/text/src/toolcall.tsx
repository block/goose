import React from "react";
import { Box, Text } from "ink";
import type {
  ToolCallContent,
  ToolCallStatus,
  ToolKind,
} from "@agentclientprotocol/sdk";
import { CRANBERRY, TEAL, GOLD, TEXT_SECONDARY, TEXT_DIM } from "./colors.js";

export interface ToolCallInfo {
  toolCallId: string;
  title: string;
  status: ToolCallStatus;
  kind?: ToolKind;
  rawInput?: unknown;
  rawOutput?: unknown;
  content?: ToolCallContent[];
  locations?: Array<{ path: string; line?: number | null }>;
}

const CEDAR = "#6B5344";

const KIND_ICONS: Record<string, string> = {
  read: "📖",
  edit: "✏️",
  delete: "🗑",
  move: "📦",
  search: "🔍",
  execute: "▶",
  think: "💭",
  fetch: "🌐",
  switch_mode: "🔀",
  other: "⚙",
};

const STATUS_INDICATORS: Record<string, { icon: string; color: string }> = {
  pending: { icon: "○", color: TEXT_DIM },
  in_progress: { icon: "◑", color: GOLD },
  completed: { icon: "●", color: TEAL },
  failed: { icon: "✗", color: CRANBERRY },
};

function formatJsonCompact(value: unknown, maxWidth: number): string[] {
  if (value === undefined || value === null) return [];
  let raw: string;
  try {
    raw = JSON.stringify(value, null, 2);
  } catch {
    raw = String(value);
  }
  const lines = raw.split("\n");
  const result: string[] = [];
  for (const line of lines) {
    if (line.length <= maxWidth) {
      result.push(line);
    } else {
      let remaining = line;
      while (remaining.length > maxWidth) {
        result.push(remaining.slice(0, maxWidth));
        remaining = remaining.slice(maxWidth);
      }
      if (remaining) result.push(remaining);
    }
  }
  return result;
}

function extractTextFromContent(content: ToolCallContent[]): string[] {
  const lines: string[] = [];
  for (const item of content) {
    if (item.type === "content" && item.content) {
      const block = item.content as any;
      if (block.type === "text" && block.text) {
        lines.push(...block.text.split("\n"));
      }
    } else if (item.type === "diff") {
      const diff = item as any;
      lines.push(`diff: ${diff.path || "unknown"}`);
    } else if (item.type === "terminal") {
      const term = item as any;
      lines.push(`terminal: ${term.terminalId || "unknown"}`);
    }
  }
  return lines;
}

function summarizeContent(info: ToolCallInfo): string {
  const parts: string[] = [];

  if (info.locations && info.locations.length > 0) {
    for (const loc of info.locations) {
      parts.push(loc.path + (loc.line ? `:${loc.line}` : ""));
    }
  }

  if (info.content && info.content.length > 0) {
    const textLines = extractTextFromContent(info.content);
    if (textLines.length > 0) {
      const first = textLines[0]!.trim();
      if (first.length > 60) {
        parts.push(first.slice(0, 57) + "…");
      } else if (first) {
        parts.push(first);
      }
    }
  }

  if (parts.length === 0 && info.rawOutput !== undefined && info.rawOutput !== null) {
    const raw = String(
      typeof info.rawOutput === "string" ? info.rawOutput : JSON.stringify(info.rawOutput),
    );
    const firstLine = raw.split("\n")[0] ?? "";
    if (firstLine.length > 60) {
      parts.push(firstLine.slice(0, 57) + "…");
    } else if (firstLine) {
      parts.push(firstLine);
    }
  }

  return parts.join(" · ");
}

const MAX_PREVIEW_LINES = 8;

export function findFeaturedToolCallId(
  toolCallOrder: string[],
  toolCalls: Map<string, ToolCallInfo>,
): string | undefined {
  for (let i = toolCallOrder.length - 1; i >= 0; i--) {
    const tc = toolCalls.get(toolCallOrder[i]!);
    if (tc && (tc.status === "pending" || tc.status === "in_progress")) {
      return toolCallOrder[i]!;
    }
  }
  return toolCallOrder[toolCallOrder.length - 1];
}

export function buildToolCallCardLines(
  info: ToolCallInfo,
  indent: number,
  totalWidth: number,
  expanded: boolean,
  keyPrefix: string = "card",
): React.ReactNode[] {
  const cardWidth = Math.min(totalWidth - indent - 2, 72);
  const innerWidth = cardWidth - 2;
  const contentWidth = innerWidth - 2;
  const kindIcon = KIND_ICONS[info.kind ?? "other"] ?? "⚙";
  const statusInfo = STATUS_INDICATORS[info.status] ?? STATUS_INDICATORS.pending!;
  const borderColor = info.status === "failed" ? CRANBERRY : CEDAR;
  const dimBorder = info.status !== "failed";

  const hasInput = info.rawInput !== undefined && info.rawInput !== null;
  const hasOutput = info.rawOutput !== undefined && info.rawOutput !== null;
  const hasContent = info.content && info.content.length > 0;
  const hasLocations = info.locations && info.locations.length > 0;

  const inputLines = hasInput ? formatJsonCompact(info.rawInput, contentWidth - 6) : [];
  const outputLines = hasOutput ? formatJsonCompact(info.rawOutput, contentWidth - 6) : [];
  const contentLines = hasContent ? extractTextFromContent(info.content!) : [];

  const shownInput = expanded ? inputLines : inputLines.slice(0, MAX_PREVIEW_LINES);
  const shownOutput = expanded ? outputLines : outputLines.slice(0, MAX_PREVIEW_LINES);
  const shownContent = expanded ? contentLines : contentLines.slice(0, MAX_PREVIEW_LINES);

  const hasTruncated =
    inputLines.length > MAX_PREVIEW_LINES ||
    outputLines.length > MAX_PREVIEW_LINES ||
    contentLines.length > MAX_PREVIEW_LINES;

  const bodyRows: Array<{ text: string; color?: string; italic?: boolean }> = [];

  const runningText = info.status === "in_progress" ? " running…" : "";
  const tabHint = "";
  bodyRows.push({ text: "__HEADER__" });

  if (hasLocations) {
    for (const loc of info.locations!) {
      bodyRows.push({ text: `  📁 ${loc.path}${loc.line ? `:${loc.line}` : ""}`, color: TEXT_DIM });
    }
  }

  function addSection(label: string, lines: string[], totalCount: number) {
    if (lines.length === 0) return;
    bodyRows.push({ text: `  ▸ ${label}:`, color: TEXT_DIM });
    for (const line of lines) {
      bodyRows.push({ text: `    ${line}`, color: TEXT_DIM });
    }
    if (!expanded && totalCount > MAX_PREVIEW_LINES) {
      const remaining = totalCount - MAX_PREVIEW_LINES;
      bodyRows.push({ text: `    ▸ ${remaining} more lines (tab to expand)`, color: GOLD, italic: true });
    }
  }

  addSection("input", shownInput, inputLines.length);
  addSection("output", shownOutput, outputLines.length);
  addSection("content", shownContent, contentLines.length);

  const result: React.ReactNode[] = [];
  const topBorder = "╭" + "─".repeat(innerWidth) + "╮";
  const botBorder = "╰" + "─".repeat(innerWidth) + "╯";

  result.push(
    <Box key={`${keyPrefix}-top`} marginLeft={indent} height={1}>
      <Text color={borderColor} dimColor={dimBorder}>{topBorder}</Text>
    </Box>,
  );

  for (let i = 0; i < bodyRows.length; i++) {
    const row = bodyRows[i]!;

    if (row.text === "__HEADER__") {
      result.push(
        <Box key={`${keyPrefix}-row-${i}`} marginLeft={indent} width={cardWidth} height={1}>
          <Text color={borderColor} dimColor={dimBorder}>│ </Text>
          <Box flexGrow={1} justifyContent="space-between">
            <Box>
              <Text color={statusInfo.color}>{statusInfo.icon} </Text>
              <Text>{kindIcon} </Text>
              <Text color={TEXT_SECONDARY} bold>{info.title}</Text>
              {runningText ? <Text color={TEXT_DIM} italic>{runningText}</Text> : null}
            </Box>
            {tabHint ? <Text color={TEXT_DIM} italic>{tabHint}</Text> : null}
          </Box>
          <Text color={borderColor} dimColor={dimBorder}> │</Text>
        </Box>,
      );
      continue;
    }

    result.push(
      <Box key={`${keyPrefix}-row-${i}`} marginLeft={indent} width={cardWidth} height={1}>
        <Text color={borderColor} dimColor={dimBorder}>│</Text>
        <Box flexGrow={1}>
          <Text color={row.color} italic={row.italic}> {row.text}</Text>
        </Box>
        <Text color={borderColor} dimColor={dimBorder}>│</Text>
      </Box>,
    );
  }

  result.push(
    <Box key={`${keyPrefix}-bot`} marginLeft={indent} height={1}>
      <Text color={borderColor} dimColor={dimBorder}>{botBorder}</Text>
    </Box>,
  );

  return result;
}

export function ToolCallCompact({
  info,
  indent,
  width,
  keyPrefix = "compact",
  showTabHint = false,
}: {
  info: ToolCallInfo;
  indent: number;
  width: number;
  keyPrefix?: string;
  showTabHint?: boolean;
}): React.ReactNode[] {
  const statusInfo = STATUS_INDICATORS[info.status] ?? STATUS_INDICATORS.pending!;
  const kindIcon = KIND_ICONS[info.kind ?? "other"] ?? "⚙";
  const summary = summarizeContent(info);
  const borderColor = info.status === "failed" ? CRANBERRY : CEDAR;
  const dimBorder = info.status !== "failed";
  
  const cardWidth = Math.min(width - indent - 2, 72);
  const innerWidth = cardWidth - 2;
  
  const tabHintText = "tab ↔";
  const maxSummaryWidth = innerWidth - info.title.length - 8 - (showTabHint ? tabHintText.length + 2 : 0);
  const trimmedSummary =
    summary.length > maxSummaryWidth && maxSummaryWidth > 3
      ? summary.slice(0, maxSummaryWidth - 1) + "…"
      : summary;

  const topBorder = "╭" + "─".repeat(innerWidth) + "╮";
  const botBorder = "╰" + "─".repeat(innerWidth) + "╯";

  return [
    <Box key={`${keyPrefix}-top`} marginLeft={indent} height={1}>
      <Text color={borderColor} dimColor={dimBorder}>{topBorder}</Text>
    </Box>,
    <Box key={`${keyPrefix}-content`} marginLeft={indent} width={cardWidth} height={1}>
      <Text color={borderColor} dimColor={dimBorder}>│ </Text>
      <Box flexGrow={1} justifyContent="space-between">
        <Box>
          <Text color={statusInfo.color}>{statusInfo.icon} </Text>
          <Text>{kindIcon} </Text>
          <Text color={TEXT_SECONDARY} bold>{info.title}</Text>
          {trimmedSummary ? (
            <Text color={TEXT_DIM}> — {trimmedSummary}</Text>
          ) : null}
        </Box>
        {showTabHint && <Text color={TEXT_DIM} italic>{tabHintText}</Text>}
      </Box>
      <Text color={borderColor} dimColor={dimBorder}> │</Text>
    </Box>,
    <Box key={`${keyPrefix}-bot`} marginLeft={indent} height={1}>
      <Text color={borderColor} dimColor={dimBorder}>{botBorder}</Text>
    </Box>,
  ];
}
