import { DictationProvider } from '../../../hooks/useDictationSettings';
import { VOICE_DICTATION_ELEVENLABS_ENABLED } from '../../../updates';

interface ProviderInfoProps {
  provider: DictationProvider;
}

export const ProviderInfo = ({ provider }: ProviderInfoProps) => {
  if (!provider) return null;

  return (
    <div className="p-3 bg-background-subtle rounded-md">
      {provider === 'openai' && (
        <p className="text-xs text-text-muted">
          Uses OpenAI's Whisper API for high-quality transcription. Requires an OpenAI API key
          configured in the Models section.
        </p>
      )}
      {VOICE_DICTATION_ELEVENLABS_ENABLED && provider === 'elevenlabs' && (
        <div>
          <p className="text-xs text-text-muted">
            Uses ElevenLabs speech-to-text API for high-quality transcription.
          </p>
          <p className="text-xs text-text-muted mt-2">
            <strong>Features:</strong>
          </p>
          <ul className="text-xs text-text-muted ml-4 mt-1 list-disc">
            <li>Advanced voice processing</li>
            <li>High accuracy transcription</li>
            <li>Multiple language support</li>
            <li>Fast processing</li>
          </ul>
          <p className="text-xs text-text-muted mt-2">
            <strong>Note:</strong> Requires an ElevenLabs API key with speech-to-text access.
          </p>
        </div>
      )}
    </div>
  );
};
