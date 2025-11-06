import { toastService } from '../../../toasts';
import { agentAddExtension, ExtensionConfig, agentRemoveExtension } from '../../../api';
import { errorMessage } from '../../../utils/conversionUtils';
import {
  createExtensionRecoverHints,
  formatExtensionErrorMessage,
} from '../../../utils/extensionErrorUtils';

export async function addToAgent(
  extensionConfig: ExtensionConfig,
  sessionId: string,
  showToast: boolean
) {
  const extensionName = extensionConfig.name;
  let toastId = showToast
    ? toastService.loading({
        title: extensionName,
        msg: `adding ${extensionName} extension...`,
      })
    : 0;

  try {
    await agentAddExtension({
      body: { session_id: sessionId, config: extensionConfig },
      throwOnError: true,
    });
    if (showToast) {
      toastService.dismiss(toastId);
      toastService.success({
        title: extensionName,
        msg: `Successfully added extension`,
      });
    }
  } catch (error) {
    if (showToast) {
      toastService.dismiss(toastId);
      const errMsg = errorMessage(error);
      const recoverHints = createExtensionRecoverHints(errMsg);
      const msg = formatExtensionErrorMessage(errMsg, 'Failed to add extension');
      toastService.error({
        title: extensionName,
        msg: msg,
        traceback: errMsg,
        recoverHints,
      });
    }
    throw error;
  }
}

export async function removeFromAgent(
  extensionName: string,
  sessionId: string,
  showToast: boolean
) {
  let toastId = showToast
    ? toastService.loading({
        title: extensionName,
        msg: `Removing ${extensionName} extension...`,
      })
    : 0;

  try {
    await agentRemoveExtension({
      body: { session_id: sessionId, name: extensionName },
      throwOnError: true,
    });
    if (showToast) {
      toastService.dismiss(toastId);
      toastService.success({
        title: extensionName,
        msg: `Successfully removed extension`,
      });
    }
  } catch (error) {
    if (showToast) {
      toastService.dismiss(toastId);
      const errMsg = errorMessage(error);
      const msg = formatExtensionErrorMessage(errMsg, 'Failed to remove extension');
      toastService.error({
        title: extensionName,
        msg: msg,
        traceback: errMsg,
      });
    }
    throw error;
  }
}

export function sanitizeName(name: string) {
  return name.toLowerCase().replace(/-/g, '').replace(/_/g, '').replace(/\s/g, '');
}
