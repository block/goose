import { useState, useEffect } from 'react';
import { Input } from '../../ui/input';
import { Button } from '../../ui/button';
import { useConfig } from '../../ConfigContext';
import { ELEVENLABS_API_KEY, isSecretKeyConfigured } from '../../../hooks/dictationConstants';
import { setElevenLabsKeyCache } from '../../../hooks/useDictationSettings';

export const ElevenLabsKeyInput = () => {
  const [elevenLabsApiKey, setElevenLabsApiKey] = useState('');
  const [hasElevenLabsKey, setHasElevenLabsKey] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const { upsert, read, remove } = useConfig();

  useEffect(() => {
    const checkKey = async () => {
      const response = await read(ELEVENLABS_API_KEY, true);
      const hasKey = isSecretKeyConfigured(response);
      setHasElevenLabsKey(hasKey);
      setElevenLabsKeyCache(hasKey);
    };
    checkKey();
  }, [read]);

  const handleSave = async () => {
    const trimmedKey = elevenLabsApiKey.trim();
    if (!trimmedKey) return;

    await upsert(ELEVENLABS_API_KEY, trimmedKey, true);
    setElevenLabsApiKey('');
    setIsEditing(false);
    setHasElevenLabsKey(true);
    setElevenLabsKeyCache(true);
  };

  const handleRemove = async () => {
    await remove(ELEVENLABS_API_KEY, true);
    setElevenLabsApiKey('');
    setIsEditing(false);
    setHasElevenLabsKey(false);
    setElevenLabsKeyCache(false);
  };

  const handleCancel = () => {
    setElevenLabsApiKey('');
    setIsEditing(false);
  };

  return (
    <div className="py-2 px-2 bg-background-subtle rounded-lg">
      <div className="mb-2">
        <h4 className="text-text-default text-sm">ElevenLabs API Key</h4>
        <p className="text-xs text-text-muted mt-[2px]">
          Required for ElevenLabs voice recognition
          {hasElevenLabsKey && <span className="text-green-600 ml-2">(Configured)</span>}
        </p>
      </div>

      {!isEditing ? (
        <div className="flex gap-2 flex-wrap">
          <Button variant="outline" size="sm" onClick={() => setIsEditing(true)}>
            {hasElevenLabsKey ? 'Update API Key' : 'Add API Key'}
          </Button>
          {hasElevenLabsKey && (
            <Button variant="destructive" size="sm" onClick={handleRemove}>
              Remove API Key
            </Button>
          )}
        </div>
      ) : (
        <div className="space-y-2">
          <Input
            type="password"
            value={elevenLabsApiKey}
            onChange={(e) => setElevenLabsApiKey(e.target.value)}
            placeholder="Enter your ElevenLabs API key"
            className="max-w-md"
            autoFocus
          />
          <div className="flex gap-2">
            <Button size="sm" onClick={handleSave}>
              Save
            </Button>
            <Button variant="outline" size="sm" onClick={handleCancel}>
              Cancel
            </Button>
          </div>
        </div>
      )}
    </div>
  );
};
