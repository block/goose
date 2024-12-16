import { nanoid } from 'nanoid';

export type Message = {
  id: string;
  role: 'assistant' | 'user';
  content: string;
  createdAt: Date;
  toolInvocations?: any[];
};

export type JSONValue = 
  | string
  | number
  | boolean
  | null
  | JSONValue[]
  | { [key: string]: JSONValue };

export type LanguageModelUsage = {
  completionTokens: number;
  promptTokens: number;
  totalTokens: number;
};

function calculateLanguageModelUsage(usage: LanguageModelUsage): LanguageModelUsage {
  return {
    completionTokens: usage.completionTokens,
    promptTokens: usage.promptTokens,
    totalTokens: usage.totalTokens,
  };
}

export async function processCustomChatResponse({
  stream,
  update,
  onToolCall,
  onFinish,
  generateId = () => nanoid(),
  getCurrentDate = () => new Date(),
}: {
  stream: ReadableStream<Uint8Array>;
  update: (newMessages: Message[], data: JSONValue[] | undefined) => void;
  onToolCall?: (options: { toolCall: any }) => Promise<any>;
  onFinish?: (options: {
    message: Message | undefined;
    finishReason: string;
    usage: LanguageModelUsage;
  }) => void;
  generateId?: () => string;
  getCurrentDate?: () => Date;
}) {
  const createdAt = getCurrentDate();
  let currentMessage: Message | undefined = undefined;
  const previousMessages: Message[] = [];
  const data: JSONValue[] = [];

  const reader = stream.getReader();
  const decoder = new TextDecoder();

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      const text = decoder.decode(value);
      
      // Create new message if needed
      if (!currentMessage) {
        currentMessage = {
          id: generateId(),
          role: 'assistant',
          content: text,
          createdAt,
        };
      } else {
        // Append to existing message
        currentMessage.content += text;
      }

      // Update UI
      const messages = [...previousMessages];
      if (currentMessage) {
        messages.push({
          ...currentMessage,
          id: generateId(),
        });
      }
      update(messages, data);
    }
  } catch (error) {
    console.error('Error processing stream:', error);
    throw error;
  } finally {
    reader.releaseLock();
  }

  // Final update
  if (currentMessage) {
    previousMessages.push(currentMessage);
  }
  
  onFinish?.({
    message: currentMessage,
    finishReason: 'stop',
    usage: {
      completionTokens: 0,
      promptTokens: 0,
      totalTokens: 0,
    },
  });
}