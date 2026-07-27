import React from "react";
import { Box, Text } from "ink";
import { TEAL, GOLD, CRANBERRY, TEXT_DIM } from "../colors.js";
import {
  CONTEXT_BAR_WIDTH,
  CONTEXT_WARNING_THRESHOLD,
  CONTEXT_CRITICAL_THRESHOLD,
} from "../constants.js";

interface ContextBarProps {
  used: number;
  size: number;
  width: number;
  marginTop: number;
}

function formatTokenCount(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${Math.round(n / 1_000)}k`;
  return `${n}`;
}

export const ContextBar = React.memo(function ContextBar({
  used,
  size,
  width,
  marginTop,
}: ContextBarProps) {
  const constrainedWidth = Math.max(width, 1);

  // Mirrors crates/goose-cli/src/session/output.rs::display_context_usage
  // so the TUI and CLI agree on what "context usage" means for the same session.
  let line: string;
  let color: string;
  if (size === 0) {
    line = "  context usage unavailable";
    color = TEXT_DIM;
  } else {
    const percentage = Math.min(Math.round((used / size) * 100), 100);
    const filled = Math.min(
      Math.round((percentage / 100) * CONTEXT_BAR_WIDTH),
      CONTEXT_BAR_WIDTH,
    );
    const empty = CONTEXT_BAR_WIDTH - filled;
    const bar = "━".repeat(filled) + "╌".repeat(empty);
    color =
      percentage < CONTEXT_WARNING_THRESHOLD
        ? TEAL
        : percentage < CONTEXT_CRITICAL_THRESHOLD
          ? GOLD
          : CRANBERRY;
    line = `  ${bar} ${percentage}% ${formatTokenCount(used)}/${formatTokenCount(size)}`;
  }

  return (
    <Box width={constrainedWidth} marginTop={marginTop} flexShrink={0}>
      <Text color={color} wrap="truncate-end">
        {line}
      </Text>
    </Box>
  );
});
