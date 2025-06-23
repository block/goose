import {
  Message as FrontendMessage,
  Content as FrontendContent,
  MessageContent as FrontendMessageContent,
  ToolCallResult,
  ToolCall,
  Role,
} from '../../types/message';
import {
  ContextManageRequest,
  ContextManageResponse,
  manageContext,
  Message as ApiMessage,
  MessageContent as ApiMessageContent,
  ContextPathItem,
} from '../../api';
import { generateId } from 'ai';

export async function manageContextFromBackend({
  messages,
  manageAction,
}: {
  messages: FrontendMessage[];
  manageAction: 'truncation' | 'summarize';
}): Promise<ContextManageResponse> {
  try {
    const contextManagementRequest = { manageAction, messages };

    // Cast to the API-expected type
    const result = await manageContext({
      body: contextManagementRequest as unknown as ContextManageRequest,
    });

    // Check for errors in the result
    if (result.error) {
      throw new Error(`Context management failed: ${result.error}`);
    }

    // Extract the actual data from the result
    if (!result.data) {
      throw new Error('Context management returned no data');
    }

    return result.data;
  } catch (error) {
    console.error(`Context management failed: ${error || 'Unknown error'}`);
    throw new Error(
      `Context management failed: ${error || 'Unknown error'}\n\nStart a new session.`
    );
  }
}

// Function to convert API Message to frontend Message
export function convertApiMessageToFrontendMessage(
  apiMessage: ApiMessage,
  display?: boolean,
  sendToLLM?: boolean
): FrontendMessage {
  return {
    display: display ?? true,
    sendToLLM: sendToLLM ?? true,
    id: generateId(),
    role: apiMessage.role,
    created: apiMessage.created,
    content: apiMessage.content
      .map((apiContent) => mapApiContentToFrontendMessageContent(apiContent))
      .filter((content): content is FrontendMessageContent => content !== null),
  };
}

// Function to convert API MessageContent to frontend MessageContent
function mapApiContentToFrontendMessageContent(
  apiContent: ApiMessageContent
): FrontendMessageContent | null {
  // Handle each content type specifically based on its "type" property
  if (apiContent.type === 'text') {
    return {
      type: 'text',
      text: apiContent.text,
      annotations: apiContent.annotations as Record<string, unknown> | undefined,
    };
  } else if (apiContent.type === 'image') {
    return {
      type: 'image',
      data: apiContent.data,
      mimeType: apiContent.mimeType,
      annotations: apiContent.annotations as Record<string, unknown> | undefined,
    };
  } else if (apiContent.type === 'toolRequest') {
    // Ensure the toolCall has the correct type structure
    const toolCall = apiContent.toolCall as unknown as ToolCallResult<ToolCall>;

    return {
      type: 'toolRequest',
      id: apiContent.id,
      toolCall: toolCall,
    };
  } else if (apiContent.type === 'toolResponse') {
    // Ensure the toolResult has the correct type structure
    const toolResult = apiContent.toolResult as unknown as ToolCallResult<FrontendContent[]>;

    return {
      type: 'toolResponse',
      id: apiContent.id,
      toolResult: toolResult,
    };
  } else if (apiContent.type === 'toolConfirmationRequest') {
    return {
      type: 'toolConfirmationRequest',
      id: apiContent.id,
      toolName: apiContent.toolName,
      arguments: apiContent.arguments as Record<string, unknown>,
      prompt: apiContent.prompt === null ? undefined : apiContent.prompt,
    };
  } else if (apiContent.type === 'contextLengthExceeded') {
    return {
      type: 'contextLengthExceeded',
      msg: apiContent.msg,
    };
  } else if (apiContent.type === 'summarizationRequested') {
    return {
      type: 'summarizationRequested',
      msg: apiContent.msg,
    };
  } else if (apiContent.type === 'contextPaths') {
    return {
      type: 'contextPaths',
      paths: apiContent.paths.map((pathItem: ContextPathItem) => ({
        path: pathItem.path,
        type: pathItem.pathType as 'file' | 'directory' | 'unknown',
      })),
    };
  }

  // For types that exist in API but not in frontend, either skip or convert
  console.warn(`Skipping unsupported content type: ${apiContent.type}`);
  return null;
}

export function createSummarizationRequestMessage(
  messages: FrontendMessage[],
  requestMessage: string
): FrontendMessage {
  // Get the last message
  const lastMessage = messages[messages.length - 1];

  // Determine the next role (opposite of the last message)
  const nextRole: Role = lastMessage.role === 'user' ? 'assistant' : 'user';

  // Create the new message with SummarizationRequestedContent
  return {
    id: generateId(),
    role: nextRole,
    created: Math.floor(Date.now() / 1000),
    content: [
      {
        type: 'summarizationRequested',
        msg: requestMessage,
      },
    ],
    sendToLLM: false,
    display: true,
  };
}

// Function to convert frontend Message to API Message format
export function convertFrontendMessageToApiMessage(frontendMessage: FrontendMessage): ApiMessage {
  return {
    role: frontendMessage.role,
    created: frontendMessage.created,
    content: frontendMessage.content
      .map((frontendContent) => mapFrontendContentToApiMessageContent(frontendContent))
      .filter((content): content is ApiMessageContent => content !== null),
  };
}

// Function to convert frontend MessageContent to API MessageContent
function mapFrontendContentToApiMessageContent(
  frontendContent: FrontendMessageContent
): ApiMessageContent | null {
  if (frontendContent.type === 'text') {
    return {
      type: 'text',
      text: frontendContent.text,
      annotations: frontendContent.annotations as Record<string, unknown> | undefined,
    };
  } else if (frontendContent.type === 'image') {
    return {
      type: 'image',
      data: frontendContent.data,
      mimeType: frontendContent.mimeType,
      annotations: frontendContent.annotations as Record<string, unknown> | undefined,
    };
  } else if (frontendContent.type === 'toolRequest') {
    return {
      type: 'toolRequest',
      id: frontendContent.id,
      toolCall: frontendContent.toolCall
        ? (JSON.parse(JSON.stringify(frontendContent.toolCall)) as Record<string, unknown>)
        : {},
    };
  } else if (frontendContent.type === 'toolResponse') {
    return {
      type: 'toolResponse',
      id: frontendContent.id,
      toolResult: frontendContent.toolResult
        ? (JSON.parse(JSON.stringify(frontendContent.toolResult)) as Record<string, unknown>)
        : {},
    };
  } else if (frontendContent.type === 'toolConfirmationRequest') {
    return {
      type: 'toolConfirmationRequest',
      id: frontendContent.id,
      toolName: frontendContent.toolName,
      arguments: frontendContent.arguments as Record<string, unknown>,
      prompt: frontendContent.prompt,
    };
  } else if (frontendContent.type === 'contextLengthExceeded') {
    return {
      type: 'contextLengthExceeded',
      msg: frontendContent.msg,
    };
  } else if (frontendContent.type === 'summarizationRequested') {
    return {
      type: 'summarizationRequested',
      msg: frontendContent.msg,
    };
  } else if (frontendContent.type === 'contextPaths') {
    // Preserve contextPaths content type for backend storage
    return {
      type: 'contextPaths',
      paths: frontendContent.paths.map((pathItem) => ({
        path: pathItem.path,
        pathType: pathItem.type,
      })),
    };
  }

  // Type-safe fallback for unknown types
  const type = (frontendContent as { type?: string }).type;
  console.warn(`Skipping unsupported frontend content type: ${type}`);
  return null;
}
