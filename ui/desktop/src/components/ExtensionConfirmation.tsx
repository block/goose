import React, { useState } from 'react';
import { snakeToTitleCase } from '../utils';
import { confirmPermission } from '../api';

interface ExtensionConfirmationProps {
  isCancelledMessage: boolean;
  isClicked: boolean;
  extensionConfirmationId: string;
  extensionName: string;
}
export default function ExtensionConfirmation({
  isCancelledMessage,
  isClicked,
  extensionConfirmationId,
  extensionName,
}: ExtensionConfirmationProps) {
  const [clicked, setClicked] = useState(isClicked);
  const [status, setStatus] = useState('unknown');

  const handleButtonClick = async (confirmed: boolean) => {
    setClicked(true);
    setStatus(confirmed ? 'approved' : 'denied');
    try {
      const response = await confirmPermission({
        body: {
          id: extensionConfirmationId,
          action: confirmed ? 'allow_once' : 'deny',
          principal_type: 'Extension',
        },
      });
      if (response.error) {
        console.error('Failed to confirm permission: ', response.error);
      }
    } catch (err) {
      console.error('Error fetching tools:', err);
    }
  };

  return isCancelledMessage ? (
    <div className="goose-message-content bg-bgSubtle rounded-2xl px-4 py-2 text-textStandard">
      Extension enablement is cancelled.
    </div>
  ) : (
    <>
      <div className="goose-message-content bg-bgSubtle rounded-2xl px-4 py-2 rounded-b-none text-textStandard">
        Goose would like to enable the above extension. Allow?
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
                ? 'Extension enablement is not available'
                : `${snakeToTitleCase(extensionName.includes('__') ? extensionName.split('__').pop() : extensionName)} is ${status}`}{' '}
            </span>
          </div>
        </div>
      ) : (
        <div className="goose-message-tool bg-bgApp border border-borderSubtle dark:border-gray-700 rounded-b-2xl px-4 pt-4 pb-2 flex gap-4 mt-1">
          <button
            className={
              'bg-black text-white dark:bg-white dark:text-black rounded-full px-6 py-2 transition'
            }
            onClick={() => handleButtonClick(true)}
          >
            Enable extension
          </button>
          <button
            className={
              'bg-white text-black dark:bg-black dark:text-white border border-gray-300 dark:border-gray-700 rounded-full px-6 py-2 transition'
            }
            onClick={() => handleButtonClick(false)}
          >
            Deny
          </button>
        </div>
      )}
    </>
  );
}
