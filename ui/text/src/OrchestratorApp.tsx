import React, { useState, useEffect, useCallback } from 'react';
import { Box, Text, useInput, useApp } from 'ink';
import Spinner from 'ink-spinner';
import { 
  Dashboard, 
  FocusView, 
  NewTaskView, 
  MessageInput, 
  HelpView,
  DiffView,
  StatusView,
  PermissionDialog
} from './components/index.js';
import { WorkstreamManager, WorkstreamEvent } from './workstream-manager.js';
import { Workstream, ViewMode, ToolCallInfo } from './types.js';
import { GitWorktreeManager } from './worktree.js';
import { PermissionRequestParams } from './client.js';

interface OrchestratorAppProps {
  serverUrl: string;
  repoPath: string;
  transportType?: 'http' | 'websocket';
}

type OverlayMode = 'none' | 'message' | 'diff' | 'status' | 'help' | 'permission';

interface PendingPermission {
  workstreamId: string;
  workstreamName: string;
  toolTitle: string;
  toolInput?: unknown;
  options: Array<{ id: string; label: string; kind: string }>;
}

export const OrchestratorApp: React.FC<OrchestratorAppProps> = ({
  serverUrl,
  repoPath,
  transportType = 'http'
}) => {
  const { exit } = useApp();

  // Core state
  const [manager] = useState(() => new WorkstreamManager({
    serverUrl,
    repoPath,
    useWorktrees: true,
    transportType
  }));
  const [workstreams, setWorkstreams] = useState<Workstream[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [focusedId, setFocusedId] = useState<string | null>(null);
  const [activeTools, setActiveTools] = useState<ToolCallInfo[]>([]);
  
  // View state
  const [viewMode, setViewMode] = useState<ViewMode>('dashboard');
  const [overlay, setOverlay] = useState<OverlayMode>('none');
  const [overlayData, setOverlayData] = useState<string>('');
  
  // Status
  const [isGitRepo, setIsGitRepo] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [connecting, setConnecting] = useState(true);

  // Check git repo status
  useEffect(() => {
    const gitManager = new GitWorktreeManager(repoPath);
    setIsGitRepo(gitManager.isGitRepo());
    setConnecting(false);
  }, [repoPath]);

  // Subscribe to workstream events
  useEffect(() => {
    const unsubscribe = manager.onEvent((workstreamId, event) => {
      // Refresh workstreams list
      setWorkstreams([...manager.getAllWorkstreams()]);
      
      // Update active tools if focused
      if (focusedId === workstreamId) {
        setActiveTools(manager.getActiveTools(workstreamId));
      }

      // Handle specific events
      if (event.type === 'error') {
        setError(event.error);
      }
    });

    return unsubscribe;
  }, [manager, focusedId]);

  // Refresh workstreams periodically
  useEffect(() => {
    const interval = setInterval(() => {
      setWorkstreams([...manager.getAllWorkstreams()]);
      if (focusedId) {
        setActiveTools(manager.getActiveTools(focusedId));
      }
    }, 500);
    return () => clearInterval(interval);
  }, [manager, focusedId]);

  // Create new workstream
  const handleCreateWorkstream = useCallback(async (name: string, task: string) => {
    try {
      const ws = await manager.createWorkstream(name, task);
      setWorkstreams([...manager.getAllWorkstreams()]);
      setViewMode('dashboard');
      
      // Start the task
      await manager.startTask(ws.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create workstream');
    }
  }, [manager]);

  // Send message to focused workstream
  const handleSendMessage = useCallback(async (message: string) => {
    if (!focusedId) return;
    setOverlay('none');
    try {
      await manager.sendPrompt(focusedId, message);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to send message');
    }
  }, [manager, focusedId]);

  // Stop workstream
  const handleStopWorkstream = useCallback(async (id: string) => {
    try {
      await manager.stopWorkstream(id, false);
      setWorkstreams([...manager.getAllWorkstreams()]);
      if (focusedId === id) {
        setFocusedId(null);
        setViewMode('dashboard');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to stop workstream');
    }
  }, [manager, focusedId]);

  // Show diff
  const handleShowDiff = useCallback(() => {
    if (!focusedId) return;
    const diff = manager.getWorkstreamDiff(focusedId);
    setOverlayData(diff);
    setOverlay('diff');
  }, [manager, focusedId]);

  // Show status
  const handleShowStatus = useCallback(() => {
    if (!focusedId) return;
    const status = manager.getWorkstreamStatus(focusedId);
    setOverlayData(status);
    setOverlay('status');
  }, [manager, focusedId]);

  // Input handling
  useInput((input, key) => {
    // Global: quit
    if (key.ctrl && input === 'c') {
      // Cleanup all workstreams
      for (const ws of workstreams) {
        manager.stopWorkstream(ws.id, false).catch(() => {});
      }
      exit();
      return;
    }

    // Handle overlays
    if (overlay !== 'none') {
      if (key.escape || key.return || input) {
        setOverlay('none');
      }
      return;
    }

    // New task view
    if (viewMode === 'new-task') {
      if (key.escape) {
        setViewMode('dashboard');
      }
      return;
    }

    // Help view
    if (viewMode === 'help') {
      setViewMode('dashboard');
      return;
    }

    // Focus view
    if (viewMode === 'focus' && focusedId) {
      if (key.escape || input === 'b') {
        setFocusedId(null);
        setViewMode('dashboard');
      } else if (input === 'm') {
        setOverlay('message');
      } else if (input === 'd') {
        handleShowDiff();
      } else if (input === 'g') {
        handleShowStatus();
      } else if (input === 's') {
        handleStopWorkstream(focusedId);
      } else if (input === 'p') {
        const ws = manager.getWorkstream(focusedId);
        if (ws?.status === 'paused') {
          manager.resumeWorkstream(focusedId);
        } else {
          manager.pauseWorkstream(focusedId);
        }
      }
      return;
    }

    // Dashboard view
    if (viewMode === 'dashboard') {
      if (input === 'n') {
        setViewMode('new-task');
      } else if (input === '?') {
        setViewMode('help');
      } else if (input === 'q') {
        for (const ws of workstreams) {
          manager.stopWorkstream(ws.id, false).catch(() => {});
        }
        exit();
      } else if (key.upArrow || input === 'k') {
        setSelectedIndex(Math.max(0, selectedIndex - 1));
      } else if (key.downArrow || input === 'j') {
        setSelectedIndex(Math.min(workstreams.length - 1, selectedIndex + 1));
      } else if (key.return || input === 'f') {
        if (workstreams[selectedIndex]) {
          setFocusedId(workstreams[selectedIndex].id);
          setActiveTools(manager.getActiveTools(workstreams[selectedIndex].id));
          setViewMode('focus');
        }
      } else if (input === 's') {
        if (workstreams[selectedIndex]) {
          handleStopWorkstream(workstreams[selectedIndex].id);
        }
      } else if (input >= '1' && input <= '9') {
        const idx = parseInt(input) - 1;
        if (idx < workstreams.length) {
          setFocusedId(workstreams[idx].id);
          setActiveTools(manager.getActiveTools(workstreams[idx].id));
          setViewMode('focus');
        }
      }
    }
  });

  // Loading state
  if (connecting) {
    return (
      <Box flexDirection="column" padding={1}>
        <Text><Spinner type="dots" /> Initializing...</Text>
      </Box>
    );
  }

  // Error display
  if (error) {
    return (
      <Box flexDirection="column" padding={1}>
        <Text color="red">Error: {error}</Text>
        <Text dimColor>Press Ctrl+C to exit</Text>
      </Box>
    );
  }

  // Get focused workstream
  const focusedWorkstream = focusedId ? manager.getWorkstream(focusedId) : undefined;

  // Render overlays
  if (overlay === 'message' && focusedWorkstream) {
    return (
      <Box flexDirection="column" padding={1}>
        <MessageInput
          workstreamName={focusedWorkstream.name}
          onSubmit={handleSendMessage}
          onCancel={() => setOverlay('none')}
        />
      </Box>
    );
  }

  if (overlay === 'diff' && focusedWorkstream) {
    return (
      <DiffView
        workstreamName={focusedWorkstream.name}
        diff={overlayData}
        onClose={() => setOverlay('none')}
      />
    );
  }

  if (overlay === 'status' && focusedWorkstream) {
    return (
      <StatusView
        workstreamName={focusedWorkstream.name}
        status={overlayData}
        onClose={() => setOverlay('none')}
      />
    );
  }

  // Render main views
  return (
    <Box flexDirection="column" height="100%">
      {viewMode === 'dashboard' && (
        <>
          <Dashboard 
            workstreams={workstreams} 
            selectedIndex={selectedIndex}
          />
          <Box paddingX={1} marginTop={1}>
            <Text dimColor>
              {isGitRepo ? '📁 Git repo detected - worktrees enabled' : '📁 Not a git repo - working in place'}
              {' | '}Server: {serverUrl}
            </Text>
          </Box>
          <Box paddingX={1}>
            <Text dimColor>
              n: new | ↑↓: select | Enter: focus | s: stop | q: quit | ?: help
            </Text>
          </Box>
        </>
      )}

      {viewMode === 'new-task' && (
        <NewTaskView
          onSubmit={handleCreateWorkstream}
          onCancel={() => setViewMode('dashboard')}
        />
      )}

      {viewMode === 'focus' && focusedWorkstream && (
        <FocusView
          workstream={focusedWorkstream}
          activeTools={activeTools}
          currentInput=""
        />
      )}

      {viewMode === 'help' && (
        <HelpView onClose={() => setViewMode('dashboard')} />
      )}
    </Box>
  );
};
