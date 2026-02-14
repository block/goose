import React from 'react';
import { Box, Text } from 'ink';
import Spinner from 'ink-spinner';
import { Workstream, ToolCallInfo } from '../types.js';

interface FocusViewProps {
  workstream: Workstream;
  activeTools: ToolCallInfo[];
  currentInput: string;
}

export const FocusView: React.FC<FocusViewProps> = ({ 
  workstream, 
  activeTools,
  currentInput 
}) => {
  // Show last N messages
  const recentMessages = workstream.messageHistory.slice(-10);

  return (
    <Box flexDirection="column" flexGrow={1}>
      {/* Header */}
      <Box borderStyle="single" borderColor="cyan" paddingX={1} marginBottom={1}>
        <Text bold color="cyan">🪿 {workstream.name}</Text>
        {workstream.branchName && (
          <Text dimColor> | 🌿 {workstream.branchName}</Text>
        )}
        <Text dimColor> | Press 'b' to go back</Text>
      </Box>

      {/* Status bar */}
      <Box paddingX={1} marginBottom={1}>
        <StatusIndicator status={workstream.status} />
        <Text dimColor> | </Text>
        <Text dimColor>{workstream.currentActivity}</Text>
      </Box>

      {/* Message history */}
      <Box flexDirection="column" flexGrow={1} paddingX={1}>
        {recentMessages.length === 0 ? (
          <Text dimColor>No messages yet. The agent is starting up...</Text>
        ) : (
          recentMessages.map((msg, i) => (
            <Box key={i} marginBottom={1} flexDirection="column">
              <Text>
                {msg.role === 'user' && <Text color="green" bold>You: </Text>}
                {msg.role === 'assistant' && <Text color="blue" bold>Goose: </Text>}
                {msg.role === 'system' && <Text color="yellow" bold>System: </Text>}
              </Text>
              <Box marginLeft={2}>
                <Text wrap="wrap">{msg.content}</Text>
              </Box>
            </Box>
          ))
        )}
      </Box>

      {/* Active tools */}
      {activeTools.length > 0 && (
        <Box flexDirection="column" paddingX={1} marginBottom={1}>
          <Text dimColor>Active tools:</Text>
          {activeTools.map(tool => (
            <Box key={tool.id} marginLeft={2}>
              <Text color="magenta">
                {tool.status === 'pending' && <Spinner type="dots" />}
                {tool.status === 'completed' && '✓'}
                {tool.status === 'failed' && '✗'}
                {' '}{tool.title}
              </Text>
            </Box>
          ))}
        </Box>
      )}

      {/* Notifications */}
      {workstream.notifications.filter(n => !n.read).length > 0 && (
        <Box borderStyle="round" borderColor="yellow" paddingX={1} marginBottom={1}>
          <Text color="yellow" bold>
            ⚠ {workstream.notifications.filter(n => !n.read).length} notification(s)
          </Text>
        </Box>
      )}

      {/* Input hint */}
      <Box paddingX={1}>
        <Text dimColor>
          Press 'm' to send a message, 'p' to pause, 's' to stop, 'd' for diff
        </Text>
      </Box>
    </Box>
  );
};

const StatusIndicator: React.FC<{ status: Workstream['status'] }> = ({ status }) => {
  const configs: Record<Workstream['status'], { icon: string; color: string; label: string }> = {
    starting: { icon: '◐', color: 'yellow', label: 'Starting' },
    running: { icon: '●', color: 'green', label: 'Running' },
    waiting: { icon: '◉', color: 'magenta', label: 'Waiting' },
    reviewing: { icon: '◈', color: 'cyan', label: 'Review' },
    paused: { icon: '◌', color: 'gray', label: 'Paused' },
    completed: { icon: '✓', color: 'green', label: 'Done' },
    error: { icon: '✗', color: 'red', label: 'Error' }
  };

  const config = configs[status];

  if (status === 'running' || status === 'starting') {
    return (
      <Text color={config.color}>
        <Spinner type="dots" /> {config.label}
      </Text>
    );
  }

  return (
    <Text color={config.color}>
      {config.icon} {config.label}
    </Text>
  );
};
