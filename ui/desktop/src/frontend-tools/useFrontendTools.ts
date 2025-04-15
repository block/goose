import { useCallback } from 'react';
import { toolRegistry } from './registry';
import { FrontendToolRequestMessageContent } from '../types/message';
import { getApiUrl, getSecretKey } from '../config';

export function useFrontendTools() {
  const handleToolRequest = useCallback(async (request: FrontendToolRequestMessageContent) => {
    if (!request.toolCall.value) {
      throw new Error('Invalid tool call');
    }

    console.log('Handling tool request:', request);
    const { name, arguments: args } = request.toolCall.value;
    const tool = toolRegistry.getTool(name);

    if (!tool) {
      throw new Error(`Tool not found: ${name}`);
    }

    try {
      console.log('Executing tool:', name, 'with args:', args);
      const result = await tool.execute(args);
      console.log('Tool execution result:', result);

      // Send result back to server
      console.log('Sending successful result back to server:', {
        id: request.id,
        result: { Ok: result },
      });

      const response = await fetch(getApiUrl('/tool_result'), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': getSecretKey(),
        },
        body: JSON.stringify({
          id: request.id,
          result: { Ok: result },
        }),
      });

      if (!response.ok) {
        throw new Error(`Failed to send tool result: ${response.statusText}`);
      }

      console.log('Successfully sent result to server');
      return result;
    } catch (error) {
      console.error('Error executing or sending tool result:', error);

      // Send error back to server
      console.log('Sending error result back to server:', {
        id: request.id,
        result: { Err: error.message },
      });

      const response = await fetch(getApiUrl('/tool_result'), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': getSecretKey(),
        },
        body: JSON.stringify({
          id: request.id,
          result: { Err: error.message },
        }),
      });

      if (!response.ok) {
        console.error('Failed to send error result to server:', response.statusText);
      }

      throw error;
    }
  }, []);

  return { handleToolRequest };
}
