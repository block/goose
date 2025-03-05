import { getApiUrl, getSecretKey } from '../config';

export async function ConfirmToolRequest(requesyId: string, confirmed: boolean) {
  try {
    const response = await fetch(getApiUrl('/confirm'), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': getSecretKey(),
      },
      body: JSON.stringify({
        id: requesyId,
        confirmed,
      }),
    });

    if (!response.ok) {
      const errorText = await response.text();
      console.error('Delete response error: ', errorText);
      throw new Error('Failed to confirm tool');
    }
  } catch (error) {
    console.error('Error confirm tool: ', error);
  }
}
