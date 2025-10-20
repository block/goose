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

export default function Hub({
  setView,
  setIsGoosehintsModalOpen,
  isExtensionsLoading,
  resetChat,
}: {
  setView: (view: View, viewOptions?: ViewOptions) => void;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
  isExtensionsLoading: boolean;
  resetChat: () => void;
}) {
  // Handle chat input submission - create new chat and navigate to pair
  const handleSubmit = (e: React.FormEvent) => {
    const customEvent = e as unknown as CustomEvent;
    const combinedTextFromInput = customEvent.detail?.value || '';

    if (combinedTextFromInput.trim()) {
      // Navigate to pair page with the message to be submitted
      // Pair will handle creating the new chat session
      resetChat();
      setView('pair', {
        disableAnimation: true,
        initialMessage: combinedTextFromInput,
      });
    }

    e.preventDefault();
  };

  return (
    <div className="flex flex-col h-full bg-background-muted">
      <div className="flex-1 flex flex-col mb-0.5">
        <SessionInsights />
      </div>

      <ChatInput
        sessionId={null}
        handleSubmit={handleSubmit}
        autoSubmit={false}
        chatState={ChatState.Idle}
        onStop={() => {}}
        commandHistory={[]}
        initialValue=""
        setView={setView}
        numTokens={0}
        inputTokens={0}
        outputTokens={0}
        droppedFiles={[]}
        onFilesProcessed={() => {}}
        messages={[]}
        disableAnimation={false}
        sessionCosts={undefined}
        setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
        isExtensionsLoading={isExtensionsLoading}
        toolCount={0}
      />
    </div>
  );
}
