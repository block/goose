const JSON_CONTENT_TYPE = 'application/json';

export const safeJsonFetch: typeof fetch = async (input, init) => {
  const response = await fetch(input, init);

  if (
    response.status === 204 ||
    response.status === 205 ||
    response.headers.get('Content-Length') === '0'
  ) {
    return response;
  }

  const contentType = response.headers.get('Content-Type')?.toLowerCase() ?? '';
  if (!contentType.includes(JSON_CONTENT_TYPE)) {
    return response;
  }

  const bodyText = await response.clone().text();
  if (bodyText) {
    return response;
  }

  const headers = new globalThis.Headers(response.headers);
  headers.set('Content-Length', '2');

  return new globalThis.Response('{}', {
    status: response.status,
    statusText: response.statusText,
    headers,
  });
};
