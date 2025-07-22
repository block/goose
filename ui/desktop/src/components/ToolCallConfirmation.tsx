import { useState, useEffect } from 'react';
import { snakeToTitleCase } from '../utils';
import PermissionModal from './settings/permission/PermissionModal';
import { ChevronRight } from 'lucide-react';
import { confirmPermission } from '../api';
import { Button } from './ui/button';

const ALWAYS_ALLOW = 'always_allow';
const ALLOW_ONCE = 'allow_once';
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

interface ToolConfirmationProps {
  isCancelledMessage: boolean;
  isClicked: boolean;
  toolConfirmationId: string;
  toolName: string;
}

export default function ToolConfirmation({
  isCancelledMessage,
  isClicked,
  toolConfirmationId,
  toolName,
}: ToolConfirmationProps) {
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

  const handleButtonClick = async (action: string) => {
    const newClicked = true;
    const newStatus = action;
    let newActionDisplay = '';

    if (action === ALWAYS_ALLOW) {
      newActionDisplay = 'always allowed';
    } else if (action === ALLOW_ONCE) {
      newActionDisplay = 'allowed once';
    } else {
      newActionDisplay = 'denied';
    }

    // Update local state
    setClicked(newClicked);
    setStatus(newStatus);
    setActionDisplay(newActionDisplay);

    // Store in global state for persistence across navigation
    toolConfirmationState.set(toolConfirmationId, {
      clicked: newClicked,
      status: newStatus,
      actionDisplay: newActionDisplay,
    });

    try {
      const response = await confirmPermission({
        body: { id: toolConfirmationId, action, principal_type: 'Tool' },
      });
      if (response.error) {
        console.error('Failed to confirm permission:', response.error);
      }
    } catch (err) {
      console.error('Error confirming permission:', err);
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
      <div className="goose-message-content bg-background-muted rounded-2xl px-4 py-2 rounded-b-none text-textStandard">
        Goose would like to call the above tool. Allow?
      </div>
      {clicked ? (
        <div className="goose-message-tool bg-background-default border border-borderSubtle dark:border-gray-700 rounded-b-2xl px-4 pt-2 pb-2 flex items-center justify-between">
          <div className="flex items-center">
            {status === 'always_allow' && (
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
            {status === 'allow_once' && (
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
          <Button className="rounded-full" onClick={() => handleButtonClick(ALWAYS_ALLOW)}>
            Always Allow
          </Button>
          <Button
            className="rounded-full"
            variant="secondary"
            onClick={() => handleButtonClick(ALLOW_ONCE)}
          >
            Allow Once
          </Button>
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
