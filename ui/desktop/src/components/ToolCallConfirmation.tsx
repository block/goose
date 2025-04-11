import { ConfirmToolRequest } from '../utils/toolConfirm';
import Confirmation from './Confirmation';

export default function ToolCallConfirmation({
  isCancelledMessage,
  isClicked,
  toolConfirmationId,
  toolName,
}) {
  return (
    <Confirmation
      isCancelledMessage={isCancelledMessage}
      isClicked={isClicked}
      confirmationId={toolConfirmationId}
      name={toolName}
      confirmRequest={ConfirmToolRequest}
      message="call the above tool"
      enableButtonText="Allow tool"
      denyButtonText="Deny"
    />
  );
}
