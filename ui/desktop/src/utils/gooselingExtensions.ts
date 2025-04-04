import { ExtensionConfig } from '../api/types.gen';
import { addExtension } from '../extensions';
import { BUILT_IN_EXTENSIONS } from '../extensions';

export async function configureGooselingExtensions(extensions: ExtensionConfig[]) {
  console.log('configureGooselingExtensions called with:', extensions);

  if (!extensions || extensions.length === 0) {
    console.log('No extensions to configure');
    return;
  }

  for (const extension of extensions) {
    try {
      console.log('Configuring extension:', extension.name);

      // Create a full extension config
      const fullConfig = {
        id: extension.name, // Use name as id since we don't have a separate id
        name: extension.name,
        description: 'Added from gooseling', // Default description
        enabled: true,
        ...extension,
      };

      console.log('Created full config:', fullConfig);

      // First check if the extension exists in localStorage
      const userSettingsStr = localStorage.getItem('user_settings');
      console.log('Current user_settings raw:', userSettingsStr);

      let userSettings;
      try {
        userSettings = userSettingsStr
          ? JSON.parse(userSettingsStr)
          : { models: [], extensions: [] };
      } catch (parseError) {
        console.error('Error parsing user_settings:', parseError);
        userSettings = { models: [], extensions: [] };
      }
      console.log('Current parsed user settings:', userSettings);

      // Ensure extensions array exists
      if (!Array.isArray(userSettings.extensions)) {
        userSettings.extensions = [];
      }

      // Check if extension already exists
      const existingExtIndex = userSettings.extensions.findIndex(
        (ext: { id: string }) => ext.id === fullConfig.id
      );

      if (existingExtIndex !== -1) {
        console.log('Extension exists, updating enabled state');
        // Update existing extension's enabled state
        userSettings.extensions[existingExtIndex] = {
          ...userSettings.extensions[existingExtIndex],
          enabled: true,
        };
      } else {
        console.log('Extension does not exist, adding to storage');
        // Add new extension
        userSettings.extensions.push(fullConfig);
      }

      // Store updated settings
      const newSettings = JSON.stringify(userSettings);
      console.log('Storing updated settings:', newSettings);
      localStorage.setItem('user_settings', newSettings);

      // Add to agent (with silent=true and skipReInit=true to avoid notification spam and re-initialization)
      console.log('Adding extension to agent...');
      await addExtension(fullConfig, false, true);
      console.log('Successfully added extension to agent');

      // Notify settings update
      window.electron.emit('settings-updated');
      console.log('Emitted settings-updated event');
    } catch (error) {
      console.error(`Failed to configure extension ${extension.name}:`, error);
      // Continue with other extensions even if one fails
    }
  }
}
