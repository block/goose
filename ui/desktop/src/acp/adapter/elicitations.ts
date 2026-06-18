import type { Message } from '../../api';
import type { AcpElicitationRequest } from '../elicitationRequests';
import {
  type AcpChatStateChange,
  type AdapterState,
  DEFAULT_VISIBLE_MESSAGE_METADATA,
  messagesChange,
} from './shared';

export function applyElicitationRequest(
  state: AdapterState,
  request: AcpElicitationRequest
): AcpChatStateChange[] {
  if (hasExistingElicitation(state, request.id)) {
    return messagesChange(state);
  }

  state.messages.push({
    id: request.id,
    role: 'assistant',
    created: Math.floor(Date.now() / 1000),
    content: [
      {
        type: 'actionRequired',
        data: {
          actionType: 'elicitation',
          id: request.id,
          message: request.request.message,
          requested_schema: request.request.requestedSchema,
        },
      },
    ],
    metadata: { ...DEFAULT_VISIBLE_MESSAGE_METADATA },
  });

  return messagesChange(state);
}

function hasExistingElicitation(state: AdapterState, elicitationId: string): boolean {
  return state.messages.some((message: Message) =>
    message.content.some(
      (content) =>
        content.type === 'actionRequired' &&
        content.data.actionType === 'elicitation' &&
        content.data.id === elicitationId
    )
  );
}
