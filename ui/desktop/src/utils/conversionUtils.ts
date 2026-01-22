export async function safeJsonParse<T>(
  response: Response,
  errorMessage: string = 'Failed to parse server response'
): Promise<T> {
  try {
    return (await response.json()) as T;
  } catch (error) {
    if (error instanceof SyntaxError) {
      throw new Error(errorMessage);
    }
    throw error;
  }
}

export function errorMessage(err: Error | unknown, default_value?: string) {
  if (err instanceof Error) {
    return err.message;
  } else if (typeof err === 'object' && err !== null && 'message' in err) {
    return String(err.message);
  } else {
    return default_value || String(err);
  }
}

export async function compressImageDataUrl(dataUrl: string): Promise<string> {
  const res = await fetch(dataUrl);
  const blob = await res.blob();
  const bitmap = await globalThis.createImageBitmap(blob);

  const maxDim = 1024;
  const scale = Math.min(1, maxDim / Math.max(bitmap.width, bitmap.height));
  const width = Math.floor(bitmap.width * scale);
  const height = Math.floor(bitmap.height * scale);

  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  canvas.getContext('2d')!.drawImage(bitmap, 0, 0, width, height);

  return canvas.toDataURL('image/jpeg', 0.85);
}
