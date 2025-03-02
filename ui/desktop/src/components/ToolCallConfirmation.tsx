import React, { useState } from 'react';
import { ConfirmToolRequest } from '../utils/toolConfirm';

export default function ToolConfirmation({ toolConfirmationId }) {
  const [disabled, setDisabled] = useState(false);

  const handleButtonClick = (confirmed) => {
    setDisabled(true);
    ConfirmToolRequest(toolConfirmationId, confirmed);
  };

  return (
    <>
      <div className="goose-message-content bg-bgSubtle rounded-2xl px-4 py-2 rounded-b-none">
        Goose would like to call the above tool. Allow?
      </div>
      <div className="goose-message-tool bg-bgApp border border-borderSubtle dark:border-gray-700 rounded-b-2xl px-4 pt-4 pb-2 flex gap-4 mt-1">
        <button
          className={
            'bg-black text-white dark:bg-white dark:text-black rounded-full px-6 py-2 transition'
          }
          onClick={() => handleButtonClick(true)}
          disabled={disabled}
        >
          Allow tool
        </button>
        <button
          className={
            'bg-white text-black dark:bg-black dark:text-white border border-gray-300 dark:border-gray-700 rounded-full px-6 py-2 transition'
          }
          onClick={() => handleButtonClick(false)}
          disabled={disabled}
        >
          Deny
        </button>
      </div>
    </>
  );
}
