import { useState } from 'react';
import { ConfirmToolRequest } from '../utils/toolConfirm';
import { snakeToTitleCase } from '../utils';
import PermissionModal from './settings_v2/permission/PermissionModal';
import { ChevronRight } from 'lucide-react';

const ALWAYS_ALLOW = 'always_allow';
const ALLOW_ONCE = 'allow_once';
const DENY = 'deny';

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
  const [clicked, setClicked] = useState(isClicked);
  const [status, setStatus] = useState('unknown');
  const [actionDisplay, setActionDisplay] = useState('');
  const [isModalOpen, setIsModalOpen] = useState(false);

  const handleButtonClick = (action: string) => {
    setClicked(true);
    setStatus(action);
    if (action === ALWAYS_ALLOW) {
      setActionDisplay('always allowed');
    } else if (action === ALLOW_ONCE) {
      setActionDisplay('allowed once');
    } else {
      setActionDisplay('denied');
    }
    ConfirmToolRequest(toolConfirmationId, action);
  };

  const handleModalClose = () => {
    setIsModalOpen(false);
  };

  function getExtensionName(toolName: string): string {
    const parts = toolName.split('__');
    return parts.length > 1 ? parts[0] : '';
  }

  return isCancelledMessage ? (
    <div className="goose-message-content bg-bgSubtle rounded-2xl px-4 py-2 text-textStandard">
      Tool call confirmation is cancelled.
    </div>
  ) : (
    <>
      <div className="goose-message-content bg-bgSubtle rounded-2xl px-4 py-2 rounded-b-none text-textStandard">
        Goose would like to call the above tool. Allow?
      </div>
      {clicked ? (
        <div className="goose-message-tool bg-bgApp border border-borderSubtle dark:border-gray-700 rounded-b-2xl px-4 pt-2 pb-2 flex items-center justify-between">
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
        <div className="goose-message-tool bg-bgApp border border-borderSubtle dark:border-gray-700 rounded-b-2xl px-4 pt-2 pb-2 flex gap-2 items-center">
          <button
            className={
              'bg-black text-white dark:bg-white dark:text-black rounded-full px-6 py-2 transition'
            }
            onClick={() => handleButtonClick(ALWAYS_ALLOW)}
          >
            Always Allow
          </button>
          <button
            className={
              'bg-bgProminent text-white dark:text-white rounded-full px-6 py-2 transition'
            }
            onClick={() => handleButtonClick(ALLOW_ONCE)}
          >
            Allow Once
          </button>
          <button
            className={
              'bg-white text-black dark:bg-black dark:text-white border border-gray-300 dark:border-gray-700 rounded-full px-6 py-2 transition'
            }
            onClick={() => handleButtonClick(DENY)}
          >
            Deny
          </button>
        </div>
      )}

      {/* Modal for updating tool permission */}
      {isModalOpen && (
        <PermissionModal onClose={handleModalClose} extensionName={getExtensionName(toolName)} />
      )}
    </>
  );
}
