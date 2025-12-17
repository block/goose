import { useState } from 'react';
import { ActionRequired } from '../api';
import JsonSchemaForm from './ui/JsonSchemaForm';
import type { JsonSchema } from './ui/JsonSchemaForm';

interface ElicitationRequestProps {
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
    <div className="flex flex-col">
      <div className="goose-message-content bg-background-muted rounded-2xl rounded-b-none px-4 py-2 text-textStandard">
        {message || 'Goose needs some information from you.'}
      </div>
      <div className="goose-message-content bg-background-default border border-borderSubtle dark:border-gray-700 rounded-b-2xl px-4 py-3">
        <JsonSchemaForm
          schema={requested_schema as JsonSchema}
          onSubmit={handleSubmit}
          submitLabel="Submit"
        />
      </div>
    </div>
  );
}
