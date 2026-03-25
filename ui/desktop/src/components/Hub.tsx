import { AppEvents } from '../constants/events';
/**
 * Hub Component
 *
 * The Hub is the main landing page and entry point for the Goose Desktop application.
 * It serves as the welcome screen where users can start new conversations.
 *
 * Key Responsibilities:
 * - Displays SessionInsights to show session statistics and recent chats
 * - Provides a ChatInput for users to start new conversations
 * - Creates a new session and navigates to Pair with the session ID
 * - Shows loading state while session is being created
 * - Supports switching between Classic and Active Agent modes
 *
 * Navigation Flow:
 * Hub (input submission) → Create Session → Pair (with session ID and initial message)
 */

import { useEffect, useRef, useState } from 'react';
import ChatInput from './ChatInput';
import { ChatState } from '../types/chatState';
import 'react-toastify/dist/ReactToastify.css';
import { View, ViewOptions } from '../utils/navigationUtils';
import { useConfig } from './ConfigContext';
import {
  getExtensionConfigsWithOverrides,
  clearExtensionOverrides,
} from '../store/extensionOverrides';
import { getInitialWorkingDir } from '../utils/workingDir';
import { createSession } from '../sessions';

import { UserInput } from '../types/message';
import ActiveAgentView from './ActiveAgentView';
import { HubMode } from '../utils/settings';

function ModeToggle({
  mode,
  onChange,
}: {
  mode: HubMode;
  onChange: (mode: HubMode) => void;
}) {
  return (
    <div className="flex items-center justify-center py-2 relative" style={{ zIndex: 51 }}>
      <div className="no-drag inline-flex rounded-lg bg-background-primary border border-border-primary p-0.5">
        <button
          className={`px-3 py-1 text-xs font-medium rounded-md transition-colors ${
            mode === 'classic'
              ? 'bg-background-secondary text-text-primary shadow-sm'
              : 'text-text-tertiary hover:text-text-secondary'
          }`}
          onClick={() => onChange('classic')}
        >
          Classic
        </button>
        <button
          className={`px-3 py-1 text-xs font-medium rounded-md transition-colors ${
            mode === 'active'
              ? 'bg-background-secondary text-text-primary shadow-sm'
              : 'text-text-tertiary hover:text-text-secondary'
          }`}
          onClick={() => onChange('active')}
        >
          Active
        </button>
      </div>
    </div>
  );
}

export default function Hub({
  setView,
}: {
  setView: (view: View, viewOptions?: ViewOptions) => void;
}) {
  const { extensionsList } = useConfig();
  const [workingDir, setWorkingDir] = useState(getInitialWorkingDir());
  const [isCreatingSession, setIsCreatingSession] = useState(false);
  const [mode, setMode] = useState<HubMode | null>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  // Load persisted hub mode on mount
  useEffect(() => {
    window.electron.getSetting('hubMode').then((saved) => {
      setMode(saved ?? 'classic');
    });
  }, []);

  const handleModeChange = (newMode: HubMode) => {
    setMode(newMode);
    window.electron.setSetting('hubMode', newMode);
  };

  // rAF is more reliable than autoFocus across async render boundaries (Suspense, OnboardingGuard, etc.)
  useEffect(() => {
    if (mode !== 'classic') return;
    const frameId = requestAnimationFrame(() => {
      inputRef.current?.focus();
    });
    return () => cancelAnimationFrame(frameId);
  }, [mode]);

  const handleSubmit = async (input: UserInput) => {
    const { msg: userMessage, images } = input;
    if ((images.length > 0 || userMessage.trim()) && !isCreatingSession) {
      const extensionConfigs = getExtensionConfigsWithOverrides(extensionsList);
      clearExtensionOverrides();
      setIsCreatingSession(true);

      try {
        const session = await createSession(workingDir, {
          extensionConfigs,
          allExtensions: extensionConfigs.length > 0 ? undefined : extensionsList,
        });

        window.dispatchEvent(new CustomEvent(AppEvents.SESSION_CREATED));
        window.dispatchEvent(
          new CustomEvent(AppEvents.ADD_ACTIVE_SESSION, {
            detail: { sessionId: session.id, initialMessage: { msg: userMessage, images } },
          })
        );

        setView('pair', {
          disableAnimation: true,
          resumeSessionId: session.id,
          initialMessage: { msg: userMessage, images },
        });
      } catch (error) {
        console.error('Failed to create session:', error);
        setIsCreatingSession(false);
      }
    }
  };

  if (!mode) {
    return <div className="flex flex-col h-full min-h-0 bg-background-secondary" />;
  }

  return (
    <div className="flex flex-col h-full min-h-0 bg-background-secondary">
      <ModeToggle mode={mode} onChange={handleModeChange} />

      {mode === 'active' ? (
        <div className="flex-1 min-h-0">
          <ActiveAgentView setView={setView} />
        </div>
      ) : (
        <div className="flex-shrink-0 max-h-[50vh] min-h-0 overflow-hidden flex flex-col">
          <ChatInput
            sessionId={null}
            handleSubmit={handleSubmit}
            chatState={isCreatingSession ? ChatState.LoadingConversation : ChatState.Idle}
            onStop={() => {}}
            initialValue=""
            setView={setView}
            totalTokens={0}
            accumulatedInputTokens={0}
            accumulatedOutputTokens={0}
            droppedFiles={[]}
            onFilesProcessed={() => {}}
            messages={[]}
            disableAnimation={false}
            sessionCosts={undefined}
            toolCount={0}
            onWorkingDirChange={setWorkingDir}
            inputRef={inputRef}
          />
        </div>
      )}
    </div>
  );
}
