import { ExtensionConfig } from '../../../api/types.gen';
import { getApiUrl, getSecretKey } from '../../../config';
import { toastService, ToastServiceOptions } from '../../../toasts';

/**
 * Makes an API call to the extension endpoints
 */
export async function extensionApiCall(
  endpoint: string,
  payload: any,
  options: ToastServiceOptions = {}
): Promise<Response> {
  // Configure toast service for this call
  toastService.configure(options);

  let toastId;

  const actionType = endpoint === 'extensions/add' ? 'activating' : 'removing';
  const actionVerb = actionType === 'activating' ? 'Activating' : 'Removing';
  const pastVerb = actionType === 'activating' ? 'activated' : 'removed';

  const extensionName = payload.name;

  try {
    if (actionType === 'activating') {
      toastId = toastService.loading({
        title: extensionName,
        msg: `${actionVerb} ${extensionName} extension...`,
      });
    }

    const response = await fetch(getApiUrl(endpoint), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': getSecretKey(),
      },
      body: JSON.stringify(payload),
    });

    // Handle non-OK responses
    if (!response.ok) {
      const errorMsg = `Server returned ${response.status}: ${response.statusText}`;
      console.error(errorMsg);

      // Special handling for 428 Precondition Required (agent not initialized)
      if (response.status === 428 && actionType === 'activating') {
        toastService.dismiss(toastId);
        toastService.error({
          title: extensionName,
          msg: 'Agent is not initialized. Please initialize the agent first.',
          traceback: errorMsg,
        });
        throw new Error('Agent is not initialized. Please initialize the agent first.');
      }

      const msg = `Failed to ${actionType === 'activating' ? 'add' : 'remove'} ${extensionName} extension: ${errorMsg}`;
      toastService.dismiss(toastId);
      toastService.error({
        title: extensionName,
        msg: msg,
        traceback: errorMsg,
      });
      throw new Error(msg);
    }

    // Parse response JSON safely
    let data;
    try {
      const text = await response.text();
      data = text ? JSON.parse(text) : { error: false };
    } catch (parseError) {
      console.warn('Could not parse response as JSON, assuming success', parseError);
      data = { error: false };
    }

    if (!data.error) {
      toastService.dismiss(toastId);
      toastService.success({ title: extensionName, msg: `Successfully ${pastVerb} extension` });
      return response;
    } else {
      const errorMessage = `Error ${actionType} extension -- parsing data: ${data.message || 'Unknown error'}`;
      toastService.dismiss(toastId);
      toastService.error({
        title: extensionName,
        msg: errorMessage,
        traceback: data.message || 'Unknown error',
      });
      throw new Error(errorMessage);
    }
  } catch (error) {
    toastService.dismiss(toastId);
    console.error(`Error in extensionApiCall for ${extensionName}:`, error);
    throw error;
  }
}

/**
 * Add an extension to the agent
 */
export async function addToAgent(
  extension: ExtensionConfig,
  options: ToastServiceOptions = {}
): Promise<Response> {
  try {
    if (extension.type === 'stdio') {
      extension.cmd = await replaceWithShims(extension.cmd);
    }

    return await extensionApiCall('/extensions/add', extension, options);
  } catch (error) {
    // Check if this is a 428 error and make the message more descriptive
    if (error.message && error.message.includes('428')) {
      const enhancedError = new Error(
        'Agent is not initialized. Please initialize the agent first. (428 Precondition Required)'
      );
      console.error(`Failed to add extension ${extension.name} to agent: ${enhancedError.message}`);
      throw enhancedError;
    }

    console.error(`Failed to add extension ${extension.name} to agent:`, error);
    throw error;
  }
}

/**
 * Remove an extension from the agent
 */
export async function removeFromAgent(
  name: string,
  options: ToastServiceOptions = {}
): Promise<Response> {
  try {
    return await extensionApiCall('/extensions/remove', name, options);
  } catch (error) {
    console.error(`Failed to remove extension ${name} from agent:`, error);
    throw error;
  }
}

// Update the path to the binary based on the command
async function replaceWithShims(cmd: string): Promise<string> {
  const binaryPathMap: Record<string, string> = {
    goosed: await window.electron.getBinaryPath('goosed'),
    npx: await window.electron.getBinaryPath('npx'),
    uvx: await window.electron.getBinaryPath('uvx'),
  };

  if (binaryPathMap[cmd]) {
    console.log('--------> Replacing command with shim ------>', cmd, binaryPathMap[cmd]);
    return binaryPathMap[cmd];
  }

  return cmd;
}
