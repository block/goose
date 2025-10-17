import { ContextManageRequest, ContextManageResponse, manageContext, Message } from '../../api';

export async function manageContextFromBackend({
  messages,
  manageAction,
  sessionId,
}: {
  messages: Message[];
  manageAction: 'truncation' | 'summarize';
  sessionId: string;
}): Promise<ContextManageResponse> {
  const contextManagementRequest = { manageAction, messages, sessionId };

  // Cast to the API-expected type
  const result = await manageContext({
    body: contextManagementRequest as unknown as ContextManageRequest,
  });

  // Check for errors in the result
  if (result.error) {
    throw new Error(`Context management failed: ${result.error}`);
  }

  if (!result.data) {
    throw new Error('Context management returned no data');
  }

  return result.data;
}
