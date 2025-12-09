import { useState, useEffect } from 'react';
import { snakeToTitleCase } from '../utils';
import PermissionModal from './settings/permission/PermissionModal';
import { ChevronRight } from 'lucide-react';
import { confirmToolAction, ActionRequired } from '../api';
import { Button } from './ui/button';

const ALLOW_ONCE = 'allow_once';
const ALWAYS_ALLOW = 'always_allow';
const DENY = 'deny';

// Global state to track tool confirmation decisions
// This persists across navigation within the same session
const toolConfirmationState = new Map<
  string,
  {
    clicked: boolean;
    status: string;
    actionDisplay: string;
  }
>();

type ToolConfirmationData = Extract<ActionRequired['data'], { actionType: 'toolConfirmation' }>;

interface ToolConfirmationProps {
  sessionId: string;
  isCancelledMessage: boolean;
  isClicked: boolean;
  actionRequiredContent: ActionRequired & { type: 'actionRequired' };
}

export default function ToolConfirmation({
  sessionId,
  isCancelledMessage,
  isClicked,
  actionRequiredContent,
}: ToolConfirmationProps) {
  const data = actionRequiredContent.data as ToolConfirmationData;
  const { id: toolConfirmationId, toolName, prompt } = data;

  // Check if we have a stored state for this tool confirmation
  const storedState = toolConfirmationState.get(toolConfirmationId);

  // Initialize state from stored state if available, otherwise use props/defaults
  const [clicked, setClicked] = useState(storedState?.clicked ?? isClicked);
  const [status, setStatus] = useState(storedState?.status ?? 'unknown');
  const [actionDisplay, setActionDisplay] = useState(storedState?.actionDisplay ?? '');
  const [isModalOpen, setIsModalOpen] = useState(false);

  // Sync internal state with stored state and props
  useEffect(() => {
    const currentStoredState = toolConfirmationState.get(toolConfirmationId);

    // If we have stored state, use it
    if (currentStoredState) {
      setClicked(currentStoredState.clicked);
      setStatus(currentStoredState.status);
      setActionDisplay(currentStoredState.actionDisplay);
    } else if (isClicked && !clicked) {
      // Fallback to prop-based logic for historical confirmations
      setClicked(isClicked);
      if (status === 'unknown') {
        setStatus('confirmed');
        setActionDisplay('confirmed');

        // Store this state for future renders
        toolConfirmationState.set(toolConfirmationId, {
          clicked: true,
          status: 'confirmed',
          actionDisplay: 'confirmed',
        });
      }
    }
  }, [isClicked, clicked, status, toolName, toolConfirmationId]);

  const handleButtonClick = async (newStatus: string) => {
    let newActionDisplay;

    if (newStatus === ALWAYS_ALLOW) {
      newActionDisplay = 'always allowed';
    } else if (newStatus === ALLOW_ONCE) {
      newActionDisplay = 'allowed once';
    } else if (newStatus === DENY) {
      newActionDisplay = 'denied';
    } else {
      newActionDisplay = 'denied';
    }

    // Update local state
    setClicked(true);
    setStatus(newStatus);
    setActionDisplay(newActionDisplay);

    // Store in global state for persistence across navigation
    toolConfirmationState.set(toolConfirmationId, {
      clicked: true,
      status: newStatus,
      actionDisplay: newActionDisplay,
    });

    try {
      const response = await confirmToolAction({
        body: {
          sessionId: sessionId,
          id: toolConfirmationId,
          action: newStatus,
          principalType: 'Tool',
        },
      });
      if (response.error) {
        console.error('Failed to confirm tool action:', response.error);
      }
    } catch (err) {
      console.error('Error confirming tool action:', err);
    }
  };

  const handleModalClose = () => {
    setIsModalOpen(false);
  };

  function getExtensionName(toolName: string): string {
    const parts = toolName.split('__');
    return parts.length > 1 ? parts[0] : '';
  }

  return isCancelledMessage ? (
    <div className="goose-message-content bg-background-muted rounded-2xl px-4 py-2 text-textStandard">
      Tool call confirmation is cancelled.
    </div>
  ) : (
    <>
      {/* Display security message if present */}
      {prompt && (
        <div className="goose-message-content bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-2xl px-4 py-2 mb-2 text-yellow-800 dark:text-gray-200">
          {prompt}
        </div>
      )}

      <div className="goose-message-content bg-background-muted rounded-2xl px-4 py-2 rounded-b-none text-textStandard">
        {prompt
          ? 'Do you allow this tool call?'
          : 'Goose would like to call the above tool. Allow?'}
      </div>
      {clicked ? (
        <div className="goose-message-tool bg-background-default border border-borderSubtle dark:border-gray-700 rounded-b-2xl px-4 pt-2 pb-2 flex items-center justify-between">
          <div className="flex items-center">
            {(status === 'allow_once' || status === 'always_allow') && (
              <svg
                className="w-5 h-5 text-gray-500"
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                strokeWidth={2}
              >
                <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
              </svg>
            )}
            {status === 'deny' && (
              <svg
                className="w-5 h-5 text-gray-500"
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                strokeWidth={2}
              >
                <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
              </svg>
            )}
            {status === 'confirmed' && (
              <svg
                className="w-5 h-5 text-gray-500"
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                strokeWidth={2}
              >
                <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
              </svg>
            )}
            <span className="ml-2 text-textStandard">
              {isClicked
                ? 'Tool confirmation is not available'
                : `${snakeToTitleCase(toolName.substring(toolName.lastIndexOf('__') + 2))} is ${actionDisplay}`}
            </span>
          </div>

          <div className="flex items-center cursor-pointer" onClick={() => setIsModalOpen(true)}>
            <span className="mr-1 text-textStandard">Change</span>
            <ChevronRight className="w-4 h-4 ml-1 text-iconStandard" />
          </div>
        </div>
      ) : (
        <div className="goose-message-tool bg-background-default border border-borderSubtle dark:border-gray-700 rounded-b-2xl px-4 pt-2 pb-2 flex gap-2 items-center">
          <Button
            className="rounded-full"
            variant="secondary"
            onClick={() => handleButtonClick(ALLOW_ONCE)}
          >
            Allow Once
          </Button>
          {/* Only show "Always Allow" if there's no security message (no security finding) */}
          {!prompt && (
            <Button
              className="rounded-full"
              variant="secondary"
              onClick={() => handleButtonClick(ALWAYS_ALLOW)}
            >
              Always Allow
            </Button>
          )}
          <Button
            className="rounded-full"
            variant="outline"
            onClick={() => handleButtonClick(DENY)}
          >
            Deny
          </Button>
        </div>
      )}

      {/* Modal for updating tool permission */}
      {isModalOpen && (
        <PermissionModal onClose={handleModalClose} extensionName={getExtensionName(toolName)} />
      )}
    </>
  );
}
