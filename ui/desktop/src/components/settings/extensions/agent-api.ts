import { toastService } from '../../../toasts';
import { agentAddExtension, ExtensionConfig, agentRemoveExtension } from '../../../api';
import { errorMessage } from '../../../utils/conversionUtils';

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
    }
    const errMsg = errorMessage(error);
    const msg = errMsg.length < 70 ? errMsg : `Failed to add extension`;
    toastService.error({
      title: extensionName,
      msg: msg,
      traceback: errMsg,
    });
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
    }
    const errorMessage = error instanceof Error ? error.message : String(error);
    const msg = errorMessage.length < 70 ? errorMessage : `Failed to remove extension`;
    toastService.error({
      title: extensionName,
      msg: msg,
      traceback: errorMessage,
    });
    throw error;
  }
}

export function sanitizeName(name: string) {
  return name.toLowerCase().replace(/-/g, '').replace(/_/g, '').replace(/\s/g, '');
}
