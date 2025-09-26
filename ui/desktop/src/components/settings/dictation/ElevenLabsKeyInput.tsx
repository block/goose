import { useState, useEffect, useRef } from 'react';
import { Input } from '../../ui/input';
import { useConfig } from '../../ConfigContext';
import { ELEVENLABS_API_KEY } from '../../../hooks/dictationConstants';

export const ElevenLabsKeyInput = () => {
  const [elevenLabsApiKey, setElevenLabsApiKey] = useState('');
  const [isLoadingKey, setIsLoadingKey] = useState(false);
  const [hasElevenLabsKey, setHasElevenLabsKey] = useState(false);
  const elevenLabsApiKeyRef = useRef('');
  const { upsert, read } = useConfig();

  useEffect(() => {
    const loadKey = async () => {
      setIsLoadingKey(true);
      try {
        const keyExists = await read(ELEVENLABS_API_KEY, true);
        if (keyExists === true) {
          setHasElevenLabsKey(true);
        }
      } catch (error) {
        console.error('Error checking ElevenLabs API key:', error);
      } finally {
        setIsLoadingKey(false);
      }
    };

    loadKey();
  }, [read]);

  // Save key on unmount to avoid losing unsaved changes
  useEffect(() => {
    return () => {
      if (elevenLabsApiKeyRef.current) {
        const keyToSave = elevenLabsApiKeyRef.current;
        if (keyToSave.trim()) {
          upsert(ELEVENLABS_API_KEY, keyToSave, true).catch((error) => {
            console.error('Error saving ElevenLabs API key on unmount:', error);
          });
        }
      }
    };
  }, [upsert]);

  const handleElevenLabsKeyChange = (key: string) => {
    setElevenLabsApiKey(key);
    elevenLabsApiKeyRef.current = key;
    if (key.length > 0) {
      setHasElevenLabsKey(false);
    }
  };

  const saveElevenLabsKey = async () => {
    try {
      if (elevenLabsApiKey.trim()) {
        console.log('Saving ElevenLabs API key to secure storage...');
        await upsert(ELEVENLABS_API_KEY, elevenLabsApiKey, true);
        setHasElevenLabsKey(true);
        console.log('ElevenLabs API key saved successfully');
      } else {
        console.log('Removing ElevenLabs API key from secure storage...');
        await upsert(ELEVENLABS_API_KEY, null, true);
        setHasElevenLabsKey(false);
        console.log('ElevenLabs API key removed successfully');
      }
    } catch (error) {
      console.error('Error saving ElevenLabs API key:', error);
    }
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
      <Input
        type="password"
        value={elevenLabsApiKey}
        onChange={(e) => handleElevenLabsKeyChange(e.target.value)}
        onBlur={saveElevenLabsKey}
        placeholder={
          hasElevenLabsKey ? 'Enter new API key to update' : 'Enter your ElevenLabs API key'
        }
        className="max-w-md"
        disabled={isLoadingKey}
      />
    </div>
  );
};
