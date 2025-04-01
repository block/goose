import { ExtensionConfig } from '../../../api/types.gen';
import { getApiUrl, getSecretKey } from '../../../config';
import { toastService, ToastServiceOptions } from '../../../toasts';
import { replaceWithShims } from './utils';

/**
 * Makes an API call to the extension endpoints
 */
export async function extensionApiCall(
  endpoint: string,
  payload: any,
  options: ToastServiceOptions = {}
): Promise<Response> {
  // Configure toast notifications
  toastService.configure(options);

  // Determine if we're activating or removing an extension
  const isActivating = endpoint == '/extensions/add';
  const action = {
    type: isActivating ? 'activating' : 'removing',
    verb: isActivating ? 'Activating' : 'Removing',
    pastTense: isActivating ? 'activated' : 'removed',
    presentTense: isActivating ? 'activate' : 'remove',
  };

  // for adding the payload is an extensionConfig, for removing payload is just the name
  const extensionName = isActivating ? payload.name : payload;
  let toastId;

  // Step 1: Show loading toast (only for activation of stdio)
  if (isActivating && (payload as ExtensionConfig) && payload.type == 'stdio') {
    toastId = toastService.loading({
      title: extensionName,
      msg: `${action.verb} ${extensionName} extension...`,
    });
  }

  try {
    // Step 2: Make the API call
    const response = await fetch(getApiUrl(endpoint), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': getSecretKey(),
      },
      body: JSON.stringify(payload),
    });

    // Step 3: Handle non-successful responses
    if (!response.ok) {
      return handleErrorResponse(response, extensionName, action, toastId);
    }

    // Step 4: Parse response data
    const data = await parseResponseData(response);

    // Step 5: Check for errors in the response data
    if (data.error) {
      const errorMessage = `Error ${action.type} extension: ${data.message || 'Unknown error'}`;
      toastService.dismiss(toastId);
      // Rely on the global error catch to show the copyable error toast here
      throw new Error(errorMessage);
    }

    // Step 6: Success - dismiss loading toast and return
    toastService.dismiss(toastId);
    toastService.success({
      title: extensionName,
      msg: `Successfully ${action.pastTense} extension!`,
    });
    return response;
  } catch (error) {
    // Final catch-all error handler
    toastService.dismiss(toastId);
    const msg = error.length < 70 ? error : `Failed to ${action.presentTense} extension`;
    toastService.error({
      title: extensionName,
      msg: msg,
      traceback: error,
    });
    console.error(`Error in extensionApiCall for ${extensionName}:`, error);
    throw error;
  }
}

// Helper functions to separate concerns

// Handles HTTP error responses
function handleErrorResponse(
  response: Response,
  extensionName: string,
  action: { type: string; verb: string },
  toastId: string
): never {
  const errorMsg = `Server returned ${response.status}: ${response.statusText}`;
  console.error(errorMsg);

  // Special case: Agent not initialized (status 428)
  if (response.status === 428 && action.type === 'activating') {
    toastService.dismiss(toastId);
    toastService.error({
      title: extensionName,
      msg: 'Failed to add extension. Goose Agent was still starting up. Please try again.',
      traceback: errorMsg,
    });
    throw new Error('Agent is not initialized. Please initialize the agent first.');
  }

  // General error case
  const msg = `Failed to ${action.type === 'activating' ? 'add' : 'remove'} ${extensionName} extension: ${errorMsg}`;
  toastService.dismiss(toastId);
  toastService.error({
    title: extensionName,
    msg: msg,
    traceback: errorMsg,
  });
  throw new Error(msg);
}

// Safely parses JSON response
async function parseResponseData(response: Response): Promise<any> {
  try {
    const text = await response.text();
    return text ? JSON.parse(text) : { error: false };
  } catch (parseError) {
    console.warn('Could not parse response as JSON, assuming success', parseError);
    return { error: false };
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
        'Failed to add extension. Goose Agent was still starting up. Please try again.'
      );
      console.error(`Failed to add extension ${extension.name} to agent: ${enhancedError.message}`);
      throw enhancedError;
    }
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
