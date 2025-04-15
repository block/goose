import { FrontendToolDefinition } from './types';
import { toolRegistry } from './registry';
import { getApiUrl, getSecretKey } from '../config';
import { getExtensions as apiGetExtensions } from '../api';
import type { ExtensionConfig } from '../api/types.gen';

// Calculator tool definition
export const calculatorTool: FrontendToolDefinition = {
  name: 'calculator',
  description: 'Perform basic arithmetic calculations',
  inputSchema: {
    type: 'object',
    required: ['operation', 'numbers'],
    properties: {
      operation: {
        type: 'string',
        description: 'The arithmetic operation to perform',
        enum: ['add', 'subtract', 'multiply', 'divide'],
      },
      numbers: {
        type: 'array',
        description: 'List of numbers to operate on in order',
        items: {
          type: 'number',
        },
      },
    },
  },
};

// Enable Extension tool definition
export const enableExtensionTool: FrontendToolDefinition = {
  name: 'enable_extension',
  description: 'Enable extensions to help complete tasks. Enable an extension by providing the extension name.',
  inputSchema: {
    type: 'object',
    required: ['extension_name'],
    properties: {
      extension_name: {
        type: 'string',
        description: 'The name of the extension to enable',
      },
    },
  },
};

// Calculator implementation
async function executeCalculator(args: Record<string, unknown>) {
  const operation = args.operation as string;
  const numbers = (args.numbers as number[]) || [];

  if (numbers.length === 0) {
    return [{
      type: 'text',
      text: 'Error: No numbers provided',
      annotations: null,
    }];
  }

  try {
    let result: number;
    switch (operation) {
      case 'add':
        result = numbers.reduce((a, b) => a + b, 0);
        break;
      case 'subtract':
        result = numbers[0] - numbers.slice(1).reduce((a, b) => a + b, 0);
        break;
      case 'multiply':
        result = numbers.reduce((a, b) => a * b, 1);
        break;
      case 'divide':
        result = numbers.reduce((a, b) => {
          if (b === 0) throw new Error('Division by zero');
          return a / b;
        });
        break;
      default:
        throw new Error(`Unknown operation: ${operation}`);
    }

    return [{
      type: 'text',
      text: result.toString(),
      annotations: null,
    }];
  } catch (error) {
    return [{
      type: 'text',
      text: `Error: ${error.message}`,
      annotations: null,
    }];
  }
}

// Enable Extension implementation
async function executeEnableExtension(args: Record<string, unknown>) {
  const extensionName = args.extension_name as string;

  if (!extensionName) {
    return [{
      type: 'text',
      text: 'Error: No extension name provided',
      annotations: null,
    }];
  }

  try {
    console.log('Getting extensions list...');
    // Get all available extensions from config
    const response = await apiGetExtensions();
    if (response.error) {
      console.error('Failed to get extensions:', response.error);
      return [{
        type: 'text',
        text: `Error: Failed to get extensions: ${response.error}`,
        annotations: null,
      }];
    }

    console.log('Available extensions:', response.data.extensions);

    // Find the extension config
    const extension = response.data.extensions.find(ext => ext.name === extensionName);
    if (!extension) {
      return [{
        type: 'text',
        text: `Error: Extension '${extensionName}' not found in config`,
        annotations: null,
      }];
    }

    console.log('Found extension config:', extension);

    // Call the /extensions/add_extension endpoint with the extension config
    const addResponse = await fetch(getApiUrl('/extensions/add'), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': getSecretKey(),
      },
      body: JSON.stringify({
        type: extension.type,
        name: extension.name,
        cmd: extension.cmd,
        args: extension.args,
        envs: extension.envs || {},
        timeout: extension.timeout,
        bundled: extension.bundled,
      }),
    });

    if (!addResponse.ok) {
      const errorText = await addResponse.text();
      return [{
        type: 'text',
        text: `Error: Failed to enable extension: ${errorText}`,
        annotations: null,
      }];
    }

    console.log('Add extension response:', await addResponse.json());

    return [{
      type: 'text',
      text: `Successfully enabled extension: ${extensionName}`,
      annotations: null,
    }];
  } catch (error) {
    console.error('Error in executeEnableExtension:', error);
    return [{
      type: 'text',
      text: `Error: ${error.message}`,
      annotations: null,
    }];
  }
}

// Register calculator tool
toolRegistry.registerTool(calculatorTool, executeCalculator);

// Register enable extension tool
toolRegistry.registerTool(enableExtensionTool, executeEnableExtension);

// Export available tools
export const availableTools = [calculatorTool, enableExtensionTool];