import { useCallback, useEffect, useState } from 'react';
import { SessionInsights } from './sessions/SessionsInsights';
import { AppEvents } from '../constants/events';
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
import LoadingGoose from './LoadingGoose';
import { UserInput } from '../types/message';

export default function Hub({
  setView,
}: {
  setView: (view: View, viewOptions?: ViewOptions) => void;
}) {
  const { extensionsList } = useConfig();
  const [workingDir] = useState(getInitialWorkingDir());
  const [isCreatingSession, setIsCreatingSession] = useState(false);

  const handleSubmit = useCallback(async (input: UserInput) => {
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
  }, [extensionsList, workingDir, isCreatingSession, setView]);

  // Listen for PROMPT_BAR_SUBMIT events from the unified PromptBar
  useEffect(() => {
    const handlePromptBarSubmit = (e: Event) => {
      const message = (e as CustomEvent<string>).detail;
      if (message) {
        handleSubmit({ msg: message, images: [] });
      }
    };
    window.addEventListener('PROMPT_BAR_SUBMIT', handlePromptBarSubmit);
    return () => window.removeEventListener('PROMPT_BAR_SUBMIT', handlePromptBarSubmit);
  }, [handleSubmit]);

  return (
    <div className="flex flex-col h-full min-h-0 bg-background-muted">
      <div className="flex-1 flex flex-col overflow-hidden relative">
        <SessionInsights />
        {isCreatingSession && (
          <div className="absolute bottom-1 left-4 z-20 pointer-events-none">
            <LoadingGoose chatState={ChatState.LoadingConversation} />
          </div>
        )}
      </div>
    </div>
  );
}
