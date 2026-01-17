import { useState, useEffect, useRef } from 'react';
import { Input } from '../../ui/input';
import { useConfig } from '../../ConfigContext';
import { ELEVENLABS_API_KEY } from '../../../hooks/dictationConstants';

export const ElevenLabsKeyInput = () => {
  const [elevenLabsApiKey, setElevenLabsApiKey] = useState('');
  const [isLoadingKey, setIsLoadingKey] = useState(false);
  const [hasElevenLabsKey, setHasElevenLabsKey] = useState(false);
  const elevenLabsApiKeyRef = useRef('');
  const { upsert, read, remove } = useConfig();

  useEffect(() => {
    const loadKey = async () => {
      setIsLoadingKey(true);
      try {
        const keyExists = await read(ELEVENLABS_API_KEY, true);
        const hasKey =
          keyExists !== null &&
          keyExists !== '' &&
          (typeof keyExists === 'object' && 'masked_value' in keyExists
            ? keyExists.masked_value
            : keyExists);

        if (hasKey) {
          setHasElevenLabsKey(true);
          setElevenLabsApiKey('••••••••••••••••');
        } else {
          setHasElevenLabsKey(false);
          setElevenLabsApiKey('');
        }
      } catch (error) {
        console.error('Error checking ElevenLabs API key:', error);
      } finally {
        setIsLoadingKey(false);
      }
    };

    loadKey();
  }, [read]);

  useEffect(() => {
    return () => {
      if (elevenLabsApiKeyRef.current && elevenLabsApiKeyRef.current !== '••••••••••••••••') {
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
    if (key.length > 0 && key !== '••••••••••••••••') {
      setHasElevenLabsKey(false);
    }
  };

  const saveElevenLabsKey = async () => {
    try {
      if (elevenLabsApiKey === '••••••••••••••••') {
        return;
      }

      const trimmedKey = elevenLabsApiKey.trim();

      if (trimmedKey) {
        if (trimmedKey.length < 32) {
          setHasElevenLabsKey(false);
          return;
        }

        await upsert(ELEVENLABS_API_KEY, trimmedKey, true);
        setHasElevenLabsKey(true);
        setElevenLabsApiKey('••••••••••••••••');
      } else {
        await remove(ELEVENLABS_API_KEY, true);
        setHasElevenLabsKey(false);
        setElevenLabsApiKey('');
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
        type="text"
        value={elevenLabsApiKey}
        onChange={(e) => handleElevenLabsKeyChange(e.target.value)}
        onBlur={saveElevenLabsKey}
        onFocus={(e) => {
          if (e.target.value === '••••••••••••••••') {
            setElevenLabsApiKey('');
          }
        }}
        placeholder={
          hasElevenLabsKey ? 'Enter new API key to update' : 'Enter your ElevenLabs API key'
        }
        className="max-w-md"
        disabled={isLoadingKey}
      />
    </div>
  );
};
