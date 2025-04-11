import { ConfirmToolRequest } from '../utils/toolConfirm';
import Confirmation from './Confirmation';

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
  const actions = [ALWAYS_ALLOW, ALLOW_ONCE, DENY];
  const actionDisplayMap = {
    [ALWAYS_ALLOW]: 'Always Allow',
    [ALLOW_ONCE]: 'Allow Once',
    [DENY]: 'Deny',
  };

  return (
    <Confirmation
      isCancelledMessage={isCancelledMessage}
      isClicked={isClicked}
      confirmationId={toolConfirmationId}
      name={toolName}
      confirmRequest={ConfirmToolRequest}
      message="call the above tool"
      enableButtonText="Allow"
      actions={actions}
      actionDisplayMap={actionDisplayMap}
    />
  );
}
