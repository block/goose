import React, { useState, useRef, useEffect } from 'react';
import { updateSessionMetadata } from '../../sessions';
import { toastSuccess, toastError } from '../../toasts';
import { Edit2, MessageCircleMore } from 'lucide-react';

interface SessionHeaderProps {
  sessionId: string;
  sessionName: string;
  onNameUpdated?: (newName: string) => void;
}

const MAX_DESCRIPTION_LENGTH = 200;

export function SessionHeader({ sessionId, sessionName, onNameUpdated }: SessionHeaderProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [name, setName] = useState(sessionName);
  const [isLoading, setIsLoading] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Update local name when prop changes
  useEffect(() => {
    setName(sessionName);
  }, [sessionName]);

  // Focus input when editing starts
  useEffect(() => {
    if (isEditing && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [isEditing]);

  // Handle click outside to save
  useEffect(() => {
    if (!isEditing) return;

    const handleClickOutside = (event: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        // Call handleSave directly here instead of referencing it
        if (name.trim() === '') {
          toastError({ title: 'Session name cannot be empty' });
          setName(sessionName); // Reset to original
          return;
        }

        if (name.trim().length > MAX_DESCRIPTION_LENGTH) {
          toastError({ title: `Session name too long (max ${MAX_DESCRIPTION_LENGTH} characters)` });
          setName(sessionName); // Reset to original
          return;
        }

        if (name.trim() === sessionName) {
          setIsEditing(false);
          return;
        }

        setIsLoading(true);
        updateSessionMetadata(sessionId, name.trim())
          .then(() => {
            setIsEditing(false);
            onNameUpdated?.(name.trim());
            toastSuccess({ title: 'Session name updated' });
          })
          .catch((error) => {
            console.error('Failed to update session name:', error);
            if (error instanceof Error && error.message.includes('400')) {
              toastError({
                title: `Session name too long (max ${MAX_DESCRIPTION_LENGTH} characters)`,
              });
            } else {
              toastError({ title: 'Failed to update session name' });
            }
            setName(sessionName); // Reset to original
          })
          .finally(() => {
            setIsLoading(false);
          });
      }
    };

    // Add listener with slight delay to prevent immediate triggering
    const timer = setTimeout(() => {
      document.addEventListener('mousedown', handleClickOutside);
    }, 100);

    return () => {
      window.clearTimeout(timer);
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [isEditing, sessionId, name, sessionName, onNameUpdated]);

  const handleSave = async () => {
    if (name.trim() === '') {
      toastError({ title: 'Session name cannot be empty' });
      setName(sessionName); // Reset to original
      return;
    }

    if (name.trim().length > MAX_DESCRIPTION_LENGTH) {
      toastError({ title: `Session name too long (max ${MAX_DESCRIPTION_LENGTH} characters)` });
      setName(sessionName); // Reset to original
      return;
    }

    if (name.trim() === sessionName) {
      setIsEditing(false);
      return;
    }

    setIsLoading(true);
    try {
      await updateSessionMetadata(sessionId, name.trim());
      setIsEditing(false);
      onNameUpdated?.(name.trim());
      toastSuccess({ title: 'Session name updated' });
    } catch (error) {
      console.error('Failed to update session name:', error);
      if (error instanceof Error && error.message.includes('400')) {
        toastError({ title: `Session name too long (max ${MAX_DESCRIPTION_LENGTH} characters)` });
      } else {
        toastError({ title: 'Failed to update session name' });
      }
      setName(sessionName); // Reset to original
    } finally {
      setIsLoading(false);
    }
  };

  const handleCancel = () => {
    setName(sessionName);
    setIsEditing(false);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    e.stopPropagation();
    if (e.key === 'Enter') {
      handleSave();
    } else if (e.key === 'Escape') {
      handleCancel();
    }
  };

  const handleStartEditing = (e: React.MouseEvent) => {
    e.stopPropagation();
    setIsEditing(true);
  };

  return (
    <div
      ref={containerRef}
      className="no-drag"
      style={{
        WebkitAppRegion: 'no-drag',
        pointerEvents: 'auto',
        position: 'relative',
        zIndex: 100, // Higher z-index to ensure it's above drag region
      }}
    >
      {isEditing ? (
        <div
          className="border border-borderSubtle rounded-lg p-2 pr-3 text-textSubtle text-sm flex items-center bg-bgApp"
          style={{ position: 'relative', zIndex: 101 }}
        >
          <MessageCircleMore size={14} className="mr-2 flex-shrink-0" />
          <input
            ref={inputRef}
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={handleKeyDown}
            disabled={isLoading}
            className="bg-transparent outline-none w-full min-w-[150px] max-w-[250px] text-textStandard disabled:opacity-50"
            placeholder={`Enter session name (max ${MAX_DESCRIPTION_LENGTH} chars)`}
            maxLength={MAX_DESCRIPTION_LENGTH}
            style={{
              WebkitAppRegion: 'no-drag',
              pointerEvents: 'auto',
              position: 'relative',
              zIndex: 102,
            }}
          />
        </div>
      ) : (
        <button
          className="hover:cursor-pointer border border-borderSubtle hover:border-borderStandard rounded-lg p-2 pr-3 text-textSubtle hover:text-textStandard text-sm flex items-center transition-colors group"
          onClick={handleStartEditing}
          disabled={isLoading}
          style={{
            WebkitAppRegion: 'no-drag',
            pointerEvents: 'auto',
            position: 'relative',
            zIndex: 101,
          }}
        >
          <MessageCircleMore size={14} className="mr-2 flex-shrink-0" />
          <span className="truncate text-textStandard max-w-[200px]">{sessionName}</span>
          <Edit2
            size={12}
            className="ml-2 text-textSubtle opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0"
          />
        </button>
      )}
    </div>
  );
}
