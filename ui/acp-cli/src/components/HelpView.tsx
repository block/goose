import React from 'react';
import { Box, Text } from 'ink';

interface HelpViewProps {
  onClose: () => void;
}

export const HelpView: React.FC<HelpViewProps> = ({ onClose }) => {
  return (
    <Box flexDirection="column" padding={1}>
      <Box borderStyle="single" borderColor="cyan" paddingX={1} marginBottom={1}>
        <Text bold color="cyan">🪿 Goose Orchestrator - Help</Text>
      </Box>

      <Box flexDirection="column" paddingX={1}>
        <Text bold color="yellow">Dashboard View</Text>
        <Box marginLeft={2} flexDirection="column" marginBottom={1}>
          <Text><Text color="cyan">n</Text>        Create new workstream</Text>
          <Text><Text color="cyan">↑/↓ or j/k</Text>  Navigate workstreams</Text>
          <Text><Text color="cyan">Enter or f</Text>  Focus on selected workstream</Text>
          <Text><Text color="cyan">s</Text>        Stop selected workstream</Text>
          <Text><Text color="cyan">q</Text>        Quit orchestrator</Text>
          <Text><Text color="cyan">?</Text>        Show this help</Text>
        </Box>

        <Text bold color="yellow">Focus View</Text>
        <Box marginLeft={2} flexDirection="column" marginBottom={1}>
          <Text><Text color="cyan">b or Esc</Text>  Back to dashboard</Text>
          <Text><Text color="cyan">m</Text>        Send message to workstream</Text>
          <Text><Text color="cyan">p</Text>        Pause/resume workstream</Text>
          <Text><Text color="cyan">s</Text>        Stop workstream</Text>
          <Text><Text color="cyan">d</Text>        Show git diff</Text>
          <Text><Text color="cyan">g</Text>        Show git status</Text>
          <Text><Text color="cyan">c</Text>        Commit changes</Text>
        </Box>

        <Text bold color="yellow">Concepts</Text>
        <Box marginLeft={2} flexDirection="column" marginBottom={1}>
          <Text wrap="wrap">
            <Text bold>Workstreams</Text> are independent goose agents working on tasks.
          </Text>
          <Text wrap="wrap">
            <Text bold>Worktrees</Text> are git worktrees that isolate each workstream's changes.
          </Text>
          <Text wrap="wrap">
            Each workstream runs in its own branch (goose/workstream-name).
          </Text>
        </Box>

        <Text bold color="yellow">Status Icons</Text>
        <Box marginLeft={2} flexDirection="column">
          <Text><Text color="yellow">◐</Text> Starting  - Setting up workstream</Text>
          <Text><Text color="green">●</Text> Running   - Agent is actively working</Text>
          <Text><Text color="magenta">◉</Text> Waiting   - Needs user input</Text>
          <Text><Text color="cyan">◈</Text> Reviewing - Work ready for review</Text>
          <Text><Text color="gray">◌</Text> Paused    - Paused by user</Text>
          <Text><Text color="green">✓</Text> Completed - Task finished</Text>
          <Text><Text color="red">✗</Text> Error     - Something went wrong</Text>
        </Box>
      </Box>

      <Box marginTop={2} paddingX={1}>
        <Text dimColor>Press any key to close</Text>
      </Box>
    </Box>
  );
};
