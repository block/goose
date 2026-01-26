import { useState, useEffect, useCallback } from 'react';
import { Input } from '../../ui/input';
import { Button } from '../../ui/button';
import { useConfig } from '../../ConfigContext';
import { ELEVENLABS_API_KEY, isSecretKeyConfigured } from '../../../hooks/dictationConstants';
import { setElevenLabsKeyCache } from '../../../hooks/useDictationSettings';

export const ElevenLabsKeyInput = () => {
  const [elevenLabsApiKey, setElevenLabsApiKey] = useState('');
  const [isLoadingKey, setIsLoadingKey] = useState(false);
  const [hasElevenLabsKey, setHasElevenLabsKey] = useState(false);
  const [validationError, setValidationError] = useState('');
  const [isEditing, setIsEditing] = useState(false);
  const { upsert, read, remove } = useConfig();

  const loadKey = useCallback(async () => {
    setIsLoadingKey(true);
    try {
      const response = await read(ELEVENLABS_API_KEY, true);
      const hasKey = isSecretKeyConfigured(response);
      setHasElevenLabsKey(hasKey);
      setElevenLabsKeyCache(hasKey);
    } catch (error) {
      console.error(error);
      setElevenLabsKeyCache(false);
    } finally {
      setIsLoadingKey(false);
    }
  }, [read]);

  useEffect(() => {
    loadKey();
  }, [loadKey]);

  const handleElevenLabsKeyChange = (key: string) => {
    setElevenLabsApiKey(key);
    if (validationError) {
      setValidationError('');
    }
  };

  const handleSave = async () => {
    try {
      const trimmedKey = elevenLabsApiKey.trim();

      if (!trimmedKey) {
        setValidationError('API key is required');
        return;
      }

      await upsert(ELEVENLABS_API_KEY, trimmedKey, true);
      setElevenLabsApiKey('');
      setValidationError('');
      setIsEditing(false);
      await loadKey();
    } catch (error) {
      console.error(error);
      setValidationError('Failed to save API key');
    }
  };

  const handleRemove = async () => {
    try {
      await remove(ELEVENLABS_API_KEY, true);
      await loadKey();
      setElevenLabsApiKey('');
      setValidationError('');
      setIsEditing(false);
    } catch (error) {
      console.error(error);
      setValidationError('Failed to remove API key');
    }
  };

  const handleCancel = () => {
    setElevenLabsApiKey('');
    setValidationError('');
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
        <Button
          variant="outline"
          size="sm"
          onClick={() => setIsEditing(true)}
          disabled={isLoadingKey}
        >
          {hasElevenLabsKey ? 'Update API Key' : 'Add API Key'}
        </Button>
      ) : (
        <div className="space-y-2">
          <Input
            type="password"
            value={elevenLabsApiKey}
            onChange={(e) => handleElevenLabsKeyChange(e.target.value)}
            placeholder="Enter your ElevenLabs API key"
            className="max-w-md"
            autoFocus
          />
          {validationError && <p className="text-xs text-red-600 mt-1">{validationError}</p>}
          <div className="flex gap-2">
            <Button size="sm" onClick={handleSave}>
              Save
            </Button>
            <Button variant="outline" size="sm" onClick={handleCancel}>
              Cancel
            </Button>
            {hasElevenLabsKey && (
              <Button variant="destructive" size="sm" onClick={handleRemove}>
                Remove
              </Button>
            )}
          </div>
        </div>
      )}
    </div>
  );
};
