import React from 'react';
import { Box, Text } from 'ink';
import Spinner from 'ink-spinner';
import { Workstream, WorkstreamStatus } from '../types.js';

interface WorkstreamRowProps {
  workstream: Workstream;
  index: number;
  isSelected: boolean;
}

const STATUS_ICONS: Record<WorkstreamStatus, string> = {
  starting: '◐',
  running: '●',
  waiting: '◉',
  reviewing: '◈',
  paused: '◌',
  completed: '✓',
  error: '✗'
};

const STATUS_COLORS: Record<WorkstreamStatus, string> = {
  starting: 'yellow',
  running: 'green',
  waiting: 'magenta',
  reviewing: 'cyan',
  paused: 'gray',
  completed: 'green',
  error: 'red'
};

export const WorkstreamRow: React.FC<WorkstreamRowProps> = ({ 
  workstream, 
  index, 
  isSelected 
}) => {
  const statusIcon = STATUS_ICONS[workstream.status];
  const statusColor = STATUS_COLORS[workstream.status];
  const unreadCount = workstream.notifications.filter(n => !n.read).length;

  return (
    <Box>
      <Box width={4}>
        <Text color={isSelected ? 'cyan' : undefined} bold={isSelected}>
          [{index + 1}]
        </Text>
      </Box>
      <Box width={20}>
        <Text color={isSelected ? 'cyan' : undefined} bold={isSelected}>
          {workstream.name.slice(0, 18)}
        </Text>
      </Box>
      <Box width={12}>
        {workstream.status === 'running' || workstream.status === 'starting' ? (
          <Text color={statusColor}>
            <Spinner type="dots" /> {workstream.status}
          </Text>
        ) : (
          <Text color={statusColor}>
            {statusIcon} {workstream.status}
          </Text>
        )}
      </Box>
      <Box flexGrow={1}>
        <Text dimColor wrap="truncate-end">
          {workstream.currentActivity.slice(0, 50)}
        </Text>
      </Box>
      {unreadCount > 0 && (
        <Box width={4}>
          <Text color="red" bold>
            ({unreadCount})
          </Text>
        </Box>
      )}
    </Box>
  );
};

interface DashboardProps {
  workstreams: Workstream[];
  selectedIndex: number;
}

export const Dashboard: React.FC<DashboardProps> = ({ 
  workstreams, 
  selectedIndex 
}) => {
  return (
    <Box flexDirection="column" flexGrow={1}>
      {/* Header */}
      <Box borderStyle="single" borderColor="cyan" paddingX={1} marginBottom={1}>
        <Text bold color="cyan">🪿 Goose Orchestrator</Text>
        <Text dimColor> | {workstreams.length} workstream{workstreams.length !== 1 ? 's' : ''}</Text>
      </Box>

      {/* Workstream list */}
      {workstreams.length === 0 ? (
        <Box paddingX={1} paddingY={1}>
          <Text dimColor>No active workstreams. Press 'n' to create one.</Text>
        </Box>
      ) : (
        <Box flexDirection="column" paddingX={1}>
          {/* Column headers */}
          <Box marginBottom={1}>
            <Box width={4}><Text dimColor>#</Text></Box>
            <Box width={20}><Text dimColor>Name</Text></Box>
            <Box width={12}><Text dimColor>Status</Text></Box>
            <Box flexGrow={1}><Text dimColor>Activity</Text></Box>
          </Box>
          
          {/* Workstream rows */}
          {workstreams.map((ws, i) => (
            <WorkstreamRow 
              key={ws.id} 
              workstream={ws} 
              index={i}
              isSelected={i === selectedIndex}
            />
          ))}
        </Box>
      )}
    </Box>
  );
};
