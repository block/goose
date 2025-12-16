/**
 * Hub Component
 *
 * The Hub is the main landing page and entry point for the Goose Desktop application.
 * It serves as the welcome screen where users can start new conversations.
 *
 * Key Responsibilities:
 * - Displays SessionInsights to show session statistics and recent chats
 * - Provides a ChatInput for users to start new conversations
 * - Navigates to Pair with the submitted message to start a new conversation
 * - Ensures each submission from Hub always starts a fresh conversation
 *
 * Navigation Flow:
 * Hub (input submission) → Pair (new conversation with the submitted message)
 */

import { SessionInsights } from './sessions/SessionsInsights';
import ChatInput from './ChatInput';
import { ChatState } from '../types/chatState';
import 'react-toastify/dist/ReactToastify.css';
import { View, ViewOptions } from '../utils/navigationUtils';
import { startNewSession } from '../sessions';

export default function Hub({
  setView,
  isExtensionsLoading,
}: {
  setView: (view: View, viewOptions?: ViewOptions) => void;
  isExtensionsLoading: boolean;
}) {
  const handleSubmit = async (e: React.FormEvent) => {
    const customEvent = e as unknown as CustomEvent;
    const combinedTextFromInput = customEvent.detail?.value || '';

    if (combinedTextFromInput.trim()) {
      await startNewSession(combinedTextFromInput, setView);
      e.preventDefault();
    }
  };

  return (
    <div className="flex flex-col h-full bg-background-muted">
      <div className="flex-1 flex flex-col mb-0.5">
        <SessionInsights />
      </div>

      <ChatInput
        sessionId={null}
        handleSubmit={handleSubmit}
        chatState={ChatState.Idle}
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
        isExtensionsLoading={isExtensionsLoading}
        toolCount={0}
      />
    </div>
  );
}
