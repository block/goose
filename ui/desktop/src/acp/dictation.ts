import type { DictationProviderStatusEntry } from '@aaif/goose-sdk';
import { getAcpClient } from './acpConnection';

export type { DictationProviderStatusEntry };

export type DictationProviders = Record<string, DictationProviderStatusEntry>;

export async function getDictationConfig(): Promise<DictationProviders> {
  const client = await getAcpClient();
  const response = await client.goose.dictationConfig_unstable({});
  return response.providers ?? {};
}

export async function transcribeDictation(
  audio: string,
  mimeType: string,
  provider: string
): Promise<string> {
  const client = await getAcpClient();
  const response = await client.goose.dictationTranscribe_unstable({ audio, mimeType, provider });
  return response.text;
}
