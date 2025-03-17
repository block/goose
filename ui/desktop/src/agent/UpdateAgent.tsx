import { useConfig, FixedExtensionEntry } from '../components/ConfigContext';
import { getApiUrl, getSecretKey } from '../config';
import { ExtensionConfig } from '../api';
import { toast } from 'react-toastify';
import React, { useState } from 'react';
import { initializeAgent as startAgent, replaceWithShims } from './utils';

// extensionUpdate = an extension was newly added or updated so we should attempt to add it

export const useAgent = () => {
  const { getExtensions, read } = useConfig();
  const [isUpdating, setIsUpdating] = useState(false);

  // whenever we change the model, we must call this
  const initializeAgent = async (provider: string, model: string) => {
    try {
      console.log('Initializing agent with provider', provider, 'model', model);

      const response = await startAgent(model, provider);

      if (!response.ok) {
        throw new Error(`Failed to initialize agent: ${response.statusText}`);
      }

      return true;
    } catch (error) {
      console.error('Failed to initialize agent:', error);
      toast.error(
        `Failed to initialize agent: ${error instanceof Error ? error.message : 'Unknown error'}`
      );
      return false;
    }
  };

  const updateAgent = async (extensionUpdate?: ExtensionConfig) => {
    setIsUpdating(true);

    try {
      // need to initialize agent first (i dont get why but if we dont do this, we get a 428)
      // note: we must write the value for GOOSE_MODEL and GOOSE_PROVIDER in the config before updating agent
      const goose_model = (await read('GOOSE_MODEL', false)) as string;
      const goose_provider = (await read('GOOSE_PROVIDER', false)) as string;

      console.log(
        `Starting agent with GOOSE_MODEL=${goose_model} and GOOSE_PROVIDER=${goose_provider}`
      );

      // Initialize the agent if it's a model change
      if (goose_model && goose_provider) {
        const success = await initializeAgent(goose_provider, goose_model);
        if (!success) {
          console.error('Failed to initialize agent during model change');
          return false;
        }
      }

      if (extensionUpdate) {
        await addExtensionToAgent(extensionUpdate);
      }

      return true;
    } catch (error) {
      console.error('Error updating agent:', error);
      return false;
    } finally {
      setIsUpdating(false);
    }
  };

  // TODO: set 'enabled' to false if we fail to start / add the extension
  // only for non-builtins

  // TODO: try to add some descriptive error messages for common failure modes
  const addExtensionToAgent = async (
    extension: ExtensionConfig,
    silent: boolean = false
  ): Promise<Response> => {
    if (extension.type == 'stdio') {
      console.log('extension command', extension.cmd);
      extension.cmd = await replaceWithShims(extension.cmd);
      console.log('next ext command', extension.cmd);
    }

    try {
      let toastId;
      if (!silent) {
        toastId = toast.loading(`Adding ${extension.name} extension...`, {
          position: 'top-center',
        });
        toast.info('Press the escape key to continue using goose while extension loads');
      }

      const response = await fetch(getApiUrl('/extensions/add'), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': getSecretKey(),
        },
        body: JSON.stringify(extension),
      });

      // Handle non-OK responses
      if (!response.ok) {
        const errorMsg = `Server returned ${response.status}: ${response.statusText}`;
        console.error(errorMsg);

        // Special handling for 428 Precondition Required (agent not initialized)
        if (response.status === 428) {
          if (!silent) {
            if (toastId) toast.dismiss(toastId);
            toast.error('Agent is not initialized. Please initialize the agent first.');
          }
          return response;
        }

        if (!silent) {
          if (toastId) toast.dismiss(toastId);
          toast.error(`Failed to add ${extension.name} extension: ${errorMsg}`);
        }
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

      console.log('Response data:', data);

      if (!data.error) {
        if (!silent) {
          if (toastId) toast.dismiss(toastId);
          toast.success(`Successfully enabled ${extension.name} extension`);
        }
        return response;
      }

      console.log('Error trying to send a request to the extensions endpoint');
      const errorMessage = `Error adding ${extension.name} extension${data.message ? `. ${data.message}` : ''}`;
      const ErrorMsg = ({ closeToast }: { closeToast?: () => void }) => (
        <div className="flex flex-col gap-1">
          <div>Error adding {extension.name} extension</div>
          <div>
            <button
              className="text-sm rounded px-2 py-1 bg-gray-400 hover:bg-gray-300 text-white cursor-pointer"
              onClick={() => {
                navigator.clipboard.writeText(data.message || 'Unknown error');
                closeToast?.();
              }}
            >
              Copy error message
            </button>
          </div>
        </div>
      );

      console.error(errorMessage);
      if (toastId) toast.dismiss(toastId);
      toast(ErrorMsg, { type: 'error', autoClose: false });

      return response;
    } catch (error) {
      console.log('Got some other error');
      const errorMessage = `Failed to add ${extension.name} extension: ${error instanceof Error ? error.message : 'Unknown error'}`;
      console.error(errorMessage);
      toast.error(errorMessage, { autoClose: false });
      throw error;
    }
  };

  return {
    updateAgent,
    addExtensionToAgent,
    initializeAgent,
    isUpdating,
  };
};
