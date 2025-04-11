import React, { useState } from 'react';
import { snakeToTitleCase } from '../utils';
import PermissionModal from './settings_v2/permission/PermissionModal';
import { ChevronRight } from 'lucide-react';

export default function Confirmation({
  isCancelledMessage,
  isClicked,
  confirmationId,
  name,
  confirmRequest,
  message,
  enableButtonText,
  actions,
  actionDisplayMap,
}) {
  const [clicked, setClicked] = useState(isClicked);
  const [status, setStatus] = useState('unknown');
  const [actionDisplay, setActionDisplay] = useState('');
  const [isModalOpen, setIsModalOpen] = useState(false);

  const handleButtonClick = (action) => {
    setClicked(true);
    setStatus(action);
    setActionDisplay(actionDisplayMap[action] || 'unknown');
    confirmRequest(confirmationId, action);
  };

  const handleModalClose = () => {
    setIsModalOpen(false);
  };

  return isCancelledMessage ? (
    <div className="goose-message-content bg-bgSubtle rounded-2xl px-4 py-2 text-textStandard">
      {enableButtonText} is cancelled.
    </div>
  ) : (
    <>
      <div className="goose-message-content bg-bgSubtle rounded-2xl px-4 py-2 rounded-b-none text-textStandard">
        Goose would like to {message}. Allow?
      </div>
      {clicked ? (
        <div className="goose-message-tool bg-bgApp border border-borderSubtle dark:border-gray-700 rounded-b-2xl px-4 pt-4 pb-2 flex gap-4 mt-1">
          <div className="flex items-center">
            {status === 'approved' && (
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
            {status === 'denied' && (
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
                ? `${enableButtonText} is not available`
                : `${snakeToTitleCase(name.includes('__') ? name.split('__').pop() : name)} is ${actionDisplay}`}
            </span>
          </div>
          <div className="flex items-center cursor-pointer" onClick={() => setIsModalOpen(true)}>
            <span className="mr-1 text-textStandard">Change</span>
            <ChevronRight className="w-4 h-4 ml-1 text-iconStandard" />
          </div>
        </div>
      ) : (
        <div className="goose-message-tool bg-bgApp border border-borderSubtle dark:border-gray-700 rounded-b-2xl px-4 pt-4 pb-2 flex gap-4 mt-1">
          {actions.map((action) => (
            <button
              key={action}
              className={
                'bg-black text-white dark:bg-white dark:text-black rounded-full px-6 py-2 transition'
              }
              onClick={() => handleButtonClick(action)}
            >
              {actionDisplayMap[action]}
            </button>
          ))}
        </div>
      )}

      {isModalOpen && <PermissionModal onClose={handleModalClose} extensionName={name} />}
    </>
  );
}
