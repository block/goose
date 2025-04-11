import Confirmation from './Confirmation';
import { ConfirmExtensionRequest } from '../utils/extensionConfirm';

export default function ExtensionConfirmation({
  isCancelledMessage,
  isClicked,
  extensionConfirmationId,
  extensionName,
}) {
  return (
    <Confirmation
      isCancelledMessage={isCancelledMessage}
      isClicked={isClicked}
      confirmationId={extensionConfirmationId}
      name={extensionName}
      confirmRequest={ConfirmExtensionRequest}
      message="enable the following extension"
      enableButtonText="Enable extension"
      denyButtonText="Deny"
    />
  );
}
