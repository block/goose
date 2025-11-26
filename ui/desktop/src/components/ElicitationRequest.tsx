import { useState } from 'react';
import { ActionRequired } from '../api';
import JsonSchemaForm from './ui/JsonSchemaForm';

interface ElicitationRequestProps {
  sessionId: string;
  isCancelledMessage: boolean;
  isClicked: boolean;
  actionRequiredContent: ActionRequired & { type: 'actionRequired' };
  onSubmit: (elicitationId: string, userData: Record<string, unknown>) => void;
}

export default function ElicitationRequest({
  isCancelledMessage,
  isClicked,
  actionRequiredContent,
  onSubmit,
}: ElicitationRequestProps) {
  const [submitted, setSubmitted] = useState(isClicked);

  if (actionRequiredContent.data.actionType !== 'elicitation') {
    return null;
  }

  const { id: elicitationId, message, requested_schema } = actionRequiredContent.data;

  const handleSubmit = (formData: Record<string, unknown>) => {
    setSubmitted(true);
    onSubmit(elicitationId, formData);
  };

  if (isCancelledMessage) {
    return (
      <div className="goose-message-content bg-background-muted rounded-2xl px-4 py-2 text-textStandard">
        Information request was cancelled.
      </div>
    );
  }

  if (submitted) {
    return (
      <div className="goose-message-content bg-background-muted rounded-2xl px-4 py-2 text-textStandard">
        <div className="flex items-center gap-2">
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
          <span>Information submitted</span>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      {message && (
        <div className="goose-message-content bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-2xl px-4 py-2 text-blue-800 dark:text-gray-200">
          {message}
        </div>
      )}

      <div className="goose-message-content bg-background-muted rounded-2xl px-4 py-3">
        <JsonSchemaForm
          schema={requested_schema as Record<string, unknown>}
          onSubmit={handleSubmit}
          submitLabel="Submit"
        />
      </div>
    </div>
  );
}
