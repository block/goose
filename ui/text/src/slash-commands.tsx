import React from "react";
import { Text, Box } from "ink";
import { TEXT_PRIMARY, TEXT_SECONDARY, TEXT_DIM, GOLD, RULE_COLOR } from "./colors.js";

export interface SlashCommand {
  name: string;
  description: string;
  handler: () => void | Promise<void>;
}

interface SlashCommandMenuProps {
  commands: SlashCommand[];
  filter: string;
  selectedIdx: number;
  width: number;
}

export function SlashCommandMenu({
  commands,
  filter,
  selectedIdx,
  width,
}: SlashCommandMenuProps) {
  const filtered = filterCommands(commands, filter);
  
  if (filtered.length === 0) {
    return null;
  }

  const menuWidth = Math.min(width - 6, 50);

  return (
    <Box
      flexDirection="column"
      borderStyle="round"
      borderColor={GOLD}
      width={menuWidth}
      paddingX={1}
    >
      {filtered.map((cmd, i) => {
        const active = i === selectedIdx;
        return (
          <Box key={cmd.name}>
            <Text color={active ? GOLD : RULE_COLOR}>
              {active ? "▸ " : "  "}
            </Text>
            <Text color={active ? TEXT_PRIMARY : TEXT_SECONDARY} bold={active}>
              /{cmd.name}
            </Text>
            <Text color={TEXT_DIM}> — {cmd.description}</Text>
          </Box>
        );
      })}
    </Box>
  );
}

export function filterCommands(
  commands: SlashCommand[],
  filter: string,
): SlashCommand[] {
  const searchTerm = filter.toLowerCase();
  return commands.filter(
    (cmd) =>
      cmd.name.toLowerCase().includes(searchTerm) ||
      cmd.description.toLowerCase().includes(searchTerm),
  );
}

export function detectSlashCommand(input: string): {
  isSlashCommand: boolean;
  commandText: string;
} {
  if (input.startsWith("/")) {
    return {
      isSlashCommand: true,
      commandText: input.slice(1),
    };
  }
  return {
    isSlashCommand: false,
    commandText: "",
  };
}
