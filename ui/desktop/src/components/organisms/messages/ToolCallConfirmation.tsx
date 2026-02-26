import type { ActionRequired } from '@/api';
import ToolApprovalButtons from './ToolApprovalButtons';

type ToolConfirmationData = Extract<ActionRequired['data'], { actionType: 'toolConfirmation' }>;

interface ToolConfirmationProps {
  sessionId: string;
  isClicked: boolean;
  actionRequiredContent: ActionRequired & { type: 'actionRequired' };
}

export default function ToolConfirmation({
  sessionId,
  isClicked,
  actionRequiredContent,
}: ToolConfirmationProps) {
  const data = actionRequiredContent.data as ToolConfirmationData;
  const { id, toolName, prompt } = data;

  return (
    <div className="goose-message-content bg-background-default border border-border-default rounded-2xl overflow-hidden">
      <div className="bg-background-muted px-4 py-2 text-text-default">
        {prompt
          ? 'Do you allow this tool call?'
          : 'Goose would like to call the above tool. Allow?'}
      </div>
      <ToolApprovalButtons
        data={{ id, toolName, prompt: prompt ?? undefined, sessionId, isClicked }}
      />
    </div>
  );
}
