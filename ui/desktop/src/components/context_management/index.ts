import { Message } from '../../types/message';
import { getApiUrl, getSecretKey } from '../../config';

export async function manageContext({
  messages,
  manageAction,
}: {
  messages: Message[];
  manageAction: 'trunction' | 'summarize';
}) {
  const response = await fetch(getApiUrl('/context/manage'), {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Secret-Key': getSecretKey(),
    },
    body: JSON.stringify({
      messages,
      manageAction,
    }),
  });
  if (!response.ok) {
    if (!response.ok) {
      // Get the status text or a default message
      const errorText = await response.text().catch(() => 'Unknown error');
      // log error with status and details
      console.error(
        `Context management failed: ${response.status} ${response.statusText} - ${errorText}`
      );
      throw new Error(
        `Context management failed: ${response.status} ${response.statusText} - ${errorText}\n\nStart a new session.`
      );
    }
  }
  const data = await response.json();
  return data;
}
