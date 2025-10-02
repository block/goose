import { useState, useEffect } from 'react';
import { Button } from './ui/button';
import { SamplingConfirmationRequestMessageContent } from '../types/message';

const APPROVE = 'approve';
const DENY = 'deny';
const EDIT = 'edit';

// Global state to track sampling confirmation decisions
// This persists across navigation within the same session
const samplingConfirmationState = new Map<
  string,
  {
    clicked: boolean;
    status: string;
    actionDisplay: string;
  }
>();

interface SamplingConfirmationProps {
  sessionId: string;
  isCancelledMessage: boolean;
  isClicked: boolean;
  samplingConfirmationContent: SamplingConfirmationRequestMessageContent;
}

export default function SamplingConfirmation({
  sessionId,
  isCancelledMessage,
  isClicked,
  samplingConfirmationContent,
}: SamplingConfirmationProps) {
  const {
    id: samplingId,
    extensionName,
    messages,
    systemPrompt,
    prompt,
  } = samplingConfirmationContent;

  // Check if we have a stored state for this sampling confirmation
  const storedState = samplingConfirmationState.get(samplingId);

  // Initialize state from stored state if available, otherwise use props/defaults
  const [clicked, setClicked] = useState(storedState?.clicked ?? isClicked);
  const [status, setStatus] = useState(storedState?.status ?? 'unknown');
  const [actionDisplay, setActionDisplay] = useState(storedState?.actionDisplay ?? '');
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editedMessages, setEditedMessages] = useState(messages);
  const [isEditing, setIsEditing] = useState(false);

  // Sync internal state with stored state and props
  useEffect(() => {
    const currentStoredState = samplingConfirmationState.get(samplingId);

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
        samplingConfirmationState.set(samplingId, {
          clicked: true,
          status: 'confirmed',
          actionDisplay: 'confirmed',
        });
      }
    }
  }, [isClicked, clicked, status, samplingId]);

  const handleButtonClick = async (newStatus: string) => {
    let newActionDisplay;

    if (newStatus === APPROVE) {
      newActionDisplay = 'approved';
    } else if (newStatus === DENY) {
      newActionDisplay = 'denied';
    } else if (newStatus === EDIT) {
      // Open modal for editing
      setIsModalOpen(true);
      setIsEditing(true);
      return;
    } else {
      newActionDisplay = 'denied';
    }

    // Update local state
    setClicked(true);
    setStatus(newStatus);
    setActionDisplay(newActionDisplay);

    // Store in global state for persistence across navigation
    samplingConfirmationState.set(samplingId, {
      clicked: true,
      status: newStatus,
      actionDisplay: newActionDisplay,
    });

    try {
      // Import the API function dynamically to avoid circular dependencies
      const { handleSamplingApproval } = await import('../api');

      const response = await handleSamplingApproval({
        body: {
          session_id: sessionId,
          id: samplingId,
          action: newStatus,
          edited_messages: newStatus === EDIT ? editedMessages : undefined,
        },
      });

      if (response.error) {
        console.error('Failed to approve sampling:', response.error);
      }
    } catch (err) {
      console.error('Error approving sampling:', err);
    }
  };

  const handleModalClose = () => {
    setIsModalOpen(false);
    setIsEditing(false);
  };

  const handleSaveEdit = async () => {
    // Update local state
    setClicked(true);
    setStatus(EDIT);
    setActionDisplay('edited and approved');

    // Store in global state
    samplingConfirmationState.set(samplingId, {
      clicked: true,
      status: EDIT,
      actionDisplay: 'edited and approved',
    });

    try {
      // Import the API function dynamically
      const { handleSamplingApproval } = await import('../api');

      const response = await handleSamplingApproval({
        body: {
          session_id: sessionId,
          id: samplingId,
          action: EDIT,
          edited_messages: editedMessages,
        },
      });

      if (response.error) {
        console.error('Failed to approve edited sampling:', response.error);
      }
    } catch (err) {
      console.error('Error approving edited sampling:', err);
    }

    handleModalClose();
  };

  const handleMessageEdit = (index: number, newContent: string) => {
    const updatedMessages = [...editedMessages];
    updatedMessages[index] = { ...updatedMessages[index], content: newContent };
    setEditedMessages(updatedMessages);
  };

  return (
    <>
      {isCancelledMessage ? (
        <div className="goose-message-content bg-background-muted rounded-2xl px-4 py-2 text-textStandard">
          Sampling confirmation is cancelled.
        </div>
      ) : (
        <>
          {/* Display prompt if present */}
          {prompt && (
            <div className="goose-message-content bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-2xl px-4 py-2 mb-2 text-yellow-800 dark:text-gray-200">
              {prompt}
            </div>
          )}

          {/* Display the sampling request details */}
          <div className="goose-message-content bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-2xl px-4 py-3 mb-2">
            <div className="text-sm font-semibold text-blue-800 dark:text-blue-200 mb-2">
              Extension "{extensionName}" wants to make an API call with the following messages:
            </div>

            {systemPrompt && (
              <div className="mb-3 p-2 bg-white dark:bg-gray-800 rounded">
                <div className="text-xs font-semibold text-gray-600 dark:text-gray-400 mb-1">
                  System Prompt:
                </div>
                <div className="text-sm text-gray-700 dark:text-gray-300">{systemPrompt}</div>
              </div>
            )}

            <div className="space-y-2">
              {messages.map((msg, index) => (
                <div key={index} className="p-2 bg-white dark:bg-gray-800 rounded">
                  <div className="text-xs font-semibold text-gray-600 dark:text-gray-400 mb-1">
                    {msg.role === 'user' ? 'User' : 'Assistant'}:
                  </div>
                  <div className="text-sm text-gray-700 dark:text-gray-300">{msg.content}</div>
                </div>
              ))}
            </div>
          </div>

          <div className="goose-message-content bg-background-muted rounded-2xl px-4 py-2 rounded-b-none text-textStandard">
            Do you approve this API call?
          </div>

          {clicked ? (
            <div className="goose-message-tool bg-background-default border border-borderSubtle dark:border-gray-700 rounded-b-2xl px-4 pt-2 pb-2 flex items-center justify-between">
              <div className="flex items-center">
                {status === APPROVE && (
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
                {status === DENY && (
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
                {(status === EDIT || status === 'confirmed') && (
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
                    ? 'Sampling confirmation is not available'
                    : `Sampling request from ${extensionName} is ${actionDisplay}`}
                </span>
              </div>
            </div>
          ) : (
            <div className="goose-message-tool bg-background-default border border-borderSubtle dark:border-gray-700 rounded-b-2xl px-4 pt-2 pb-2 flex gap-2 items-center">
              <Button
                className="rounded-full"
                variant="secondary"
                onClick={() => handleButtonClick(APPROVE)}
              >
                Approve
              </Button>
              <Button
                className="rounded-full"
                variant="secondary"
                onClick={() => handleButtonClick(EDIT)}
              >
                Edit & Approve
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
        </>
      )}

      {/* Modal for editing messages */}
      {isModalOpen && isEditing && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-2xl w-full max-h-[80vh] overflow-y-auto">
            <h2 className="text-xl font-semibold mb-4">Edit Sampling Messages</h2>

            {systemPrompt && (
              <div className="mb-4">
                <label className="block text-sm font-medium mb-1">System Prompt:</label>
                <div className="p-2 bg-gray-100 dark:bg-gray-700 rounded text-sm">
                  {systemPrompt}
                </div>
              </div>
            )}

            <div className="space-y-4 mb-4">
              {editedMessages.map((msg, index) => (
                <div key={index}>
                  <label className="block text-sm font-medium mb-1">
                    {msg.role === 'user' ? 'User' : 'Assistant'} Message:
                  </label>
                  <textarea
                    className="w-full p-2 border rounded dark:bg-gray-700 dark:border-gray-600"
                    rows={3}
                    value={msg.content}
                    onChange={(e) => handleMessageEdit(index, e.target.value)}
                  />
                </div>
              ))}
            </div>

            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={handleModalClose}>
                Cancel
              </Button>
              <Button onClick={handleSaveEdit}>Save & Approve</Button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
