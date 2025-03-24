import { replaceWithShims } from '../../../agent/utils';
import { ExtensionConfig } from '../../../api';
import { toast } from 'react-toastify';
import { getApiUrl, getSecretKey } from '../../../config';
import React from 'react';

// Error message component
const ErrorMsg = ({
  name,
  message,
  closeToast,
}: {
  name: string;
  message?: string;
  closeToast?: () => void;
}) => (
  <div className="flex flex-col gap-1">
    <div>
      Error {message?.includes('adding') ? 'adding' : 'removing'} {name} extension
    </div>
    <div>
      <button
        className="text-sm rounded px-2 py-1 bg-gray-400 hover:bg-gray-300 text-white cursor-pointer"
        onClick={() => {
          navigator.clipboard.writeText(message || 'Unknown error');
          closeToast?.();
        }}
      >
        Copy error message
      </button>
    </div>
  </div>
);

// Core API call function
async function extensionApiCall<T>(
  endpoint: string,
  payload: any,
  actionType: 'adding' | 'removing',
  extensionName: string
): Promise<Response> {
  let toastId;
  const actionVerb = actionType === 'adding' ? 'Adding' : 'Removing';
  const pastVerb = actionType === 'adding' ? 'added' : 'removed';

  try {
    toastId = toast.loading(`${actionVerb} ${extensionName} extension...`, {
      position: 'top-center',
    });

    if (actionType === 'adding') {
      toast.info(
        'Press the ESC key on your keyboard to continue using goose while extension loads'
      );
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
      if (response.status === 428 && actionType === 'adding') {
        if (toastId) toast.dismiss(toastId);
        toast.error('Agent is not initialized. Please initialize the agent first.');
        return response;
      }

      if (toastId) toast.dismiss(toastId);
      toast.error(
        `Failed to ${actionType === 'adding' ? 'add' : 'remove'} ${extensionName} extension: ${errorMsg}`
      );
      return response;
    }

    // Parse response JSON safely
    let data;
    try {
      const text = await response.text();
      data = text ? JSON.parse(text) : { error: false };
    } catch (error) {
      console.warn('Could not parse response as JSON, assuming success', error);
      data = { error: false };
    }

    if (!data.error) {
      if (toastId) toast.dismiss(toastId);
      toast.success(
        `Successfully ${actionType === 'adding' ? 'enabled' : 'disabled'} ${extensionName} extension`
      );
      return response;
    }

    const errorMessage = `Error ${actionType} ${extensionName} extension${data.message ? `. ${data.message}` : ''}`;
    console.error(errorMessage);

    if (toastId) toast.dismiss(toastId);
    toast(<ErrorMsg name={extensionName} message={data.message} />, {
      type: 'error',
      autoClose: false,
    });

    return response;
  } catch (error) {
    console.log('Got some other error');
    const errorMessage = `Failed to ${actionType === 'adding' ? 'add' : 'remove'} ${extensionName} extension: ${error instanceof Error ? error.message : 'Unknown error'}`;
    console.error(errorMessage);
    if (toastId) toast.dismiss(toastId);
    toast.error(errorMessage, { autoClose: false });
    throw error;
  }
}

// Public functions
export async function AddToAgent(extension: ExtensionConfig): Promise<Response> {
  if (extension.type === 'stdio') {
    console.log('extension command', extension.cmd);
    extension.cmd = await replaceWithShims(extension.cmd);
    console.log('next ext command', extension.cmd);
  }

  return extensionApiCall('/extensions/add', extension, 'adding', extension.name);
}

export async function RemoveFromAgent(name: string): Promise<Response> {
  return extensionApiCall('/extensions/remove', name, 'removing', name);
}
