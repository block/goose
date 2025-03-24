import React from 'react';

interface ErrorMsgProps {
  extensionName: string;
  errorMessage: string;
  closeToast?: () => void;
}

export const ErrorMsg: React.FC<ErrorMsgProps> = ({ extensionName, errorMessage, closeToast }) => (
  <div className="flex flex-col gap-1">
    <div>Error adding {extensionName} extension</div>
    <div>
      <button
        className="text-sm rounded px-2 py-1 bg-gray-400 hover:bg-gray-300 text-white cursor-pointer"
        onClick={() => {
          navigator.clipboard.writeText(errorMessage);
          closeToast?.();
        }}
      >
        Copy error message
      </button>
    </div>
  </div>
);
