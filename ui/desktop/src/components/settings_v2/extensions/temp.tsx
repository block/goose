import { ExtensionConfig } from '../../../api';
import { AddToAgent, RemoveFromAgent } from './agent_stuff';

interface UpdateExtensionProps {
  enabled: boolean;
  addToConfig: (name: string, extensionConfig: ExtensionConfig, enabled: boolean) => Promise<void>;
  extensionConfig: ExtensionConfig;
}

// updating -- no change to enabled state
export async function UpdateExtension({
  enabled,
  addToConfig,
  extensionConfig,
}: UpdateExtensionProps) {
  if (enabled) {
    try {
      // AddToAgent
      await AddToAgent(extensionConfig);
    } catch (error) {
      // i think only error that gets thrown here is when it's not from the response... rest are handled by agent
      console.log('error', error);
      // failed to add to agent -- show that error to user and do not update the config file
      return;
    }

    // Then add to config
    try {
      await addToConfig(extensionConfig.name, extensionConfig, enabled);
    } catch (error) {
      // config error workflow
      console.log('error', error);
    }
  } else {
    try {
      await addToConfig(extensionConfig.name, extensionConfig, enabled);
    } catch (error) {
      // TODO: Add to agent with previous configuration and raise error
      // for now just log error
      console.log('error', error);
    }
  }
}

// Adding a net-new extension (not in config)
interface AddNewExtensionProps {
  addToConfig: (name: string, extensionConfig: ExtensionConfig, enabled: boolean) => Promise<void>;
  extensionConfig: ExtensionConfig;
}

export async function AddNewExtension({ addToConfig, extensionConfig }: AddNewExtensionProps) {
  try {
    // AddToAgent
    await AddToAgent(extensionConfig);
  } catch (error) {
    // add to config with enabled = false
    await addToConfig(extensionConfig.name, extensionConfig, false);
    // show user the error, return
    console.log('error', error);
    return;
  }

  // Then add to config
  try {
    await addToConfig(extensionConfig.name, extensionConfig, true);
  } catch (error) {
    // remove from Agent
    await RemoveFromAgent(extensionConfig.name);
    // config error workflow
    console.log('error', error);
  }
}

// TODO: handle errors in their respective functions
interface ToggleExtensionProps {
  toggle: 'toggleOn' | 'toggleOff';
  extensionConfig: ExtensionConfig;
  addToConfig: (name: string, extensionConfig: ExtensionConfig, enabled: boolean) => Promise<void>;
  removeFromConfig: (name: string) => Promise<void>;
}

export async function ToggleExtension({
  toggle,
  extensionConfig,
  addToConfig,
}: ToggleExtensionProps) {
  // disabled to enabled
  if (toggle == 'toggleOn') {
    try {
      // add to agent
      await AddToAgent(extensionConfig);
    } catch (error) {
      // do nothing raise error
      // show user error
      console.log('Error adding extension to agent. Error:', error);
      return;
    }

    // update the config
    try {
      await addToConfig(extensionConfig.name, extensionConfig, true);
    } catch (error) {
      // remove from agent?
      await RemoveFromAgent(extensionConfig.name);
    }
  } else if (toggle == 'toggleOff') {
    // enabled to disabled
    try {
      await RemoveFromAgent(extensionConfig.name);
    } catch (error) {
      // note there was an error, but remove from config anyway
      console.error('Error removing extension from agent', extensionConfig.name, error);
    }
    // update the config
    try {
      await addToConfig(extensionConfig.name, extensionConfig, false);
    } catch (error) {
      // TODO: Add to agent with previous configuration
      console.log('Error removing extension from config', extensionConfig.name, 'Error:', error);
    }
  }
}

interface DeleteExtensionProps {
  name: string;
  removeFromConfig: (name: string) => Promise<void>;
}

export async function DeleteExtension({ name, removeFromConfig }: DeleteExtensionProps) {
  // remove from agent
  await RemoveFromAgent(name);

  try {
    await removeFromConfig(name);
  } catch (error) {
    console.log('Failed to remove extension from config after removing from agent. Error:', error);
    // TODO: tell user to restart goose and try again to remove (will still be present in settings but not on agent until restart)
    throw error;
  }
}
