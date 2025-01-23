import { generateId as generateIdFunction } from '@ai-sdk/provider-utils';
import type { JSONValue, Message } from '@ai-sdk/ui-utils';
import { parsePartialJson, processDataStream } from '@ai-sdk/ui-utils';
import { LanguageModelV1FinishReason } from '@ai-sdk/provider';
import { LanguageModelUsage } from './core/types/usage';

// Simple usage calculation since we don't have access to the original
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
  generateId = generateIdFunction,
  getCurrentDate = () => new Date(),
}: {
  stream: ReadableStream<Uint8Array>;
  update: (newMessages: Message[], data: JSONValue[] | undefined) => void;
  onToolCall?: (options: { toolCall: any }) => Promise<any>;
  onFinish?: (options: {
    message: Message | undefined;
    finishReason: LanguageModelV1FinishReason;
    usage: LanguageModelUsage;
  }) => void;
  generateId?: () => string;
  getCurrentDate?: () => Date;
}) {
  const createdAt = getCurrentDate();
  let currentMessage: Message | undefined = undefined;
  const previousMessages: Message[] = [];
  const data: JSONValue[] = [];
  let lastEventType: 'text' | 'tool' | undefined = undefined;

  // Keep track of partial tool calls
  const partialToolCalls: Record<string, { text: string; index: number; toolName: string }> = {};

  let usage: LanguageModelUsage = {
    completionTokens: NaN,
    promptTokens: NaN,
    totalTokens: NaN,
  };
  let finishReason: LanguageModelV1FinishReason = 'unknown';

  function execUpdate() {
    const copiedData = [...data];
    if (currentMessage == null) {
      update(previousMessages, copiedData);
      return;
    }

    const copiedMessage = {
      ...JSON.parse(JSON.stringify(currentMessage)),
      revisionId: generateId(),
    } as Message;

    update([...previousMessages, copiedMessage], copiedData);
  }

  // Create a new message only if needed
  function createNewMessage(): Message {
    if (currentMessage == null) {
      currentMessage = {
        id: generateId(),
        role: 'assistant',
        content: '',
        createdAt,
      };
    }
    return currentMessage;
  }

  // Move the current message to previous messages if it exists
  function archiveCurrentMessage() {
    if (currentMessage != null) {
      previousMessages.push(currentMessage);
      currentMessage = undefined;
    }
  }

  await processDataStream({
    stream,
    onTextPart(value) {
      // If the last event wasn't text, or we don't have a current message, create a new one
      if (lastEventType !== 'text' || currentMessage == null) {
        archiveCurrentMessage();
        currentMessage = createNewMessage();
        currentMessage.content = value;
      } else {
        // Concatenate with the existing message
        currentMessage.content += value;
      }
      lastEventType = 'text';
      execUpdate();
    },
    onToolCallStreamingStartPart(value) {
      // Always create a new message for tool calls
      archiveCurrentMessage();
      currentMessage = createNewMessage();
      lastEventType = 'tool';

      if (currentMessage.toolInvocations == null) {
        currentMessage.toolInvocations = [];
      }

      partialToolCalls[value.toolCallId] = {
        text: '',
        toolName: value.toolName,
        index: currentMessage.toolInvocations.length,
      };

      currentMessage.toolInvocations.push({
        state: 'partial-call',
        toolCallId: value.toolCallId,
        toolName: value.toolName,
        args: undefined,
      });

      execUpdate();
    },
    onToolCallDeltaPart(value) {
      if (!currentMessage) {
        currentMessage = createNewMessage();
      }
      lastEventType = 'tool';

      const partialToolCall = partialToolCalls[value.toolCallId];
      partialToolCall.text += value.argsTextDelta;

      const { value: partialArgs } = parsePartialJson(partialToolCall.text);

      currentMessage.toolInvocations![partialToolCall.index] = {
        state: 'partial-call',
        toolCallId: value.toolCallId,
        toolName: partialToolCall.toolName,
        args: partialArgs,
      };

      execUpdate();
    },
    async onToolCallPart(value) {
      if (!currentMessage) {
        currentMessage = createNewMessage();
      }
      lastEventType = 'tool';

      if (partialToolCalls[value.toolCallId] != null) {
        currentMessage.toolInvocations![partialToolCalls[value.toolCallId].index] = {
          state: 'call',
          ...value,
        };
      } else {
        if (currentMessage.toolInvocations == null) {
          currentMessage.toolInvocations = [];
        }

        currentMessage.toolInvocations.push({
          state: 'call',
          ...value,
        });
      }

      if (onToolCall) {
        const result = await onToolCall({ toolCall: value });
        if (result != null) {
          currentMessage.toolInvocations![currentMessage.toolInvocations!.length - 1] = {
            state: 'result',
            ...value,
            result,
          };
        }
      }

      execUpdate();
    },
    onToolResultPart(value) {
      if (!currentMessage) {
        currentMessage = createNewMessage();
      }
      lastEventType = 'tool';

      const toolInvocations = currentMessage.toolInvocations;
      if (toolInvocations == null) {
        throw new Error('tool_result must be preceded by a tool_call');
      }

      const toolInvocationIndex = toolInvocations.findIndex(
        (invocation) => invocation.toolCallId === value.toolCallId
      );

      if (toolInvocationIndex === -1) {
        throw new Error('tool_result must be preceded by a tool_call with the same toolCallId');
      }

      toolInvocations[toolInvocationIndex] = {
        ...toolInvocations[toolInvocationIndex],
        state: 'result' as const,
        ...value,
      };

      execUpdate();
    },
    onDataPart(value) {
      data.push(...value);
      execUpdate();
    },
    onFinishStepPart() {
      // Archive the current message when a step finishes
      archiveCurrentMessage();
    },
    onFinishMessagePart(value) {
      finishReason = value.finishReason;
      if (value.usage != null) {
        usage = calculateLanguageModelUsage(value.usage);
      }
    },
    onErrorPart(error) {
      throw new Error(error);
    },
  });

  onFinish?.({ message: currentMessage, finishReason, usage });
}
