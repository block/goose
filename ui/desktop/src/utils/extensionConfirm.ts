import { getApiUrl, getSecretKey } from '../config';

export async function ConfirmExtensionRequest(requestId: string, confirmed: boolean) {
  try {
    const response = await fetch(getApiUrl('/confirm'), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': getSecretKey(),
      },
      body: JSON.stringify({
        id: requestId,
        confirmed,
        principal_type: 'Extension',
      }),
    });

    if (!response.ok) {
      const errorText = await response.text();
      console.error('Delete response error: ', errorText);
      throw new Error('Failed to confirm extension enablement');
    }
  } catch (error) {
    console.error('Error confirming extension enablement: ', error);
  }
}
