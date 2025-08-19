const DEFAULT_OLLAMA_HOST = 'http://127.0.0.1:11434';
const OLLAMA_DOWNLOAD_URL = 'https://ollama.com/download';
const PREFERRED_MODEL = 'gpt-oss:20b';

export interface OllamaStatus {
  isRunning: boolean;
  host: string;
  error?: string;
}

export interface OllamaModel {
  name: string;
  size: number;
  digest: string;
  modified_at: string;
}

export interface PullProgress {
  status: string;
  digest?: string;
  total?: number;
  completed?: number;
}

/**
 * Check if Ollama is running on the default port
 */
export async function checkOllamaStatus(): Promise<OllamaStatus> {
  try {
    // Create an AbortController for timeout
    const controller = new AbortController();
    const timeoutId = window.setTimeout(() => controller.abort(), 2000);

    try {
      // Ollama exposes a health endpoint at /api/tags
      const response = await fetch(`${DEFAULT_OLLAMA_HOST}/api/tags`, {
        method: 'GET',
        signal: controller.signal,
      });

      window.clearTimeout(timeoutId);

      return {
        isRunning: response.ok,
        host: DEFAULT_OLLAMA_HOST,
      };
    } catch (err) {
      window.clearTimeout(timeoutId);
      throw err;
    }
  } catch (error) {
    return {
      isRunning: false,
      host: DEFAULT_OLLAMA_HOST,
      error: error instanceof Error ? error.message : 'Unknown error',
    };
  }
}

/**
 * Get the Ollama download URL
 */
export function getOllamaDownloadUrl(): string {
  return OLLAMA_DOWNLOAD_URL;
}

/**
 * Get the preferred model name
 */
export function getPreferredModel(): string {
  return PREFERRED_MODEL;
}

/**
 * Check which models are available in Ollama
 */
export async function getOllamaModels(): Promise<OllamaModel[]> {
  try {
    const controller = new AbortController();
    const timeoutId = window.setTimeout(() => controller.abort(), 5000);

    try {
      const response = await fetch(`${DEFAULT_OLLAMA_HOST}/api/tags`, {
        method: 'GET',
        signal: controller.signal,
      });

      window.clearTimeout(timeoutId);

      if (!response.ok) {
        throw new Error(`Failed to get models: ${response.statusText}`);
      }

      const data = await response.json();
      return data.models || [];
    } catch (err) {
      window.clearTimeout(timeoutId);
      throw err;
    }
  } catch (error) {
    console.error('Failed to get Ollama models:', error);
    return [];
  }
}

/**
 * Check if a specific model is available
 */
export async function hasModel(modelName: string): Promise<boolean> {
  const models = await getOllamaModels();
  return models.some((model) => model.name === modelName);
}

/**
 * Pull a model from Ollama
 */
export async function pullOllamaModel(
  modelName: string,
  onProgress?: (progress: PullProgress) => void
): Promise<boolean> {
  try {
    const response = await fetch(`${DEFAULT_OLLAMA_HOST}/api/pull`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        name: modelName,
        stream: true,
      }),
    });

    if (!response.ok) {
      throw new Error(`Failed to pull model: ${response.statusText}`);
    }

    const reader = response.body?.getReader();
    if (!reader) {
      throw new Error('No response body');
    }

    const decoder = new window.TextDecoder();
    let done = false;

    while (!done) {
      const { value, done: readerDone } = await reader.read();
      done = readerDone;

      if (value) {
        const text = decoder.decode(value);
        const lines = text.split('\n').filter((line) => line.trim());

        for (const line of lines) {
          try {
            const progress = JSON.parse(line) as PullProgress;
            if (onProgress) {
              onProgress(progress);
            }
          } catch {
            // Ignore parse errors
          }
        }
      }
    }

    return true;
  } catch (error) {
    console.error('Failed to pull model:', error);
    return false;
  }
}

/**
 * Poll for Ollama availability
 * @param onDetected Callback when Ollama is detected
 * @param intervalMs Polling interval in milliseconds
 * @returns A function to stop polling
 */
export function pollForOllama(
  onDetected: (status: OllamaStatus) => void,
  intervalMs: number = 5000
): () => void {
  let intervalId: number | null = null;
  let isPolling = true;

  const poll = async () => {
    if (!isPolling) return;

    const status = await checkOllamaStatus();
    if (status.isRunning) {
      onDetected(status);
      stopPolling();
    }
  };

  const stopPolling = () => {
    isPolling = false;
    if (intervalId) {
      window.clearInterval(intervalId);
      intervalId = null;
    }
  };

  // Start polling immediately
  poll();

  // Then poll at intervals
  intervalId = window.setInterval(poll, intervalMs);

  return stopPolling;
}
