import type React from 'react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { toast } from 'react-toastify';
import { errorMessage } from '@/utils/conversionUtils';

interface InlineEditTextProps {
  value: string;
  onSave: (newValue: string) => Promise<void>;
  maxLength?: number;
  placeholder?: string;
  disabled?: boolean;
  className?: string;
  editClassName?: string;
  onEditStart?: () => void;
  onEditEnd?: () => void;
  allowEmpty?: boolean;
  singleClickEdit?: boolean;

  /**
   * Optional activation handler for the non-editing display state.
   *
   * Intended for cases like the sidebar session list:
   * - single click should activate (open)
   * - double click should edit
   */
  onActivate?: () => void;
  activateDelayMs?: number;
}

export const InlineEditText: React.FC<InlineEditTextProps> = ({
  value,
  onSave,
  maxLength = 200,
  placeholder = 'Enter text',
  disabled = false,
  className = '',
  editClassName = '',
  onEditStart,
  onEditEnd,
  allowEmpty = false,
  singleClickEdit = true,
  onActivate,
  activateDelayMs = 200,
}) => {
  const [isEditing, setIsEditing] = useState(false);
  const [editValue, setEditValue] = useState(value);
  const [isSaving, setIsSaving] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const originalValue = useRef(value);
  const activateTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (!isEditing) {
      setEditValue(value);
      originalValue.current = value;
    }
  }, [value, isEditing]);

  useEffect(() => {
    if (isEditing && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [isEditing]);

  useEffect(() => {
    return () => {
      if (activateTimeoutRef.current) {
        clearTimeout(activateTimeoutRef.current);
        activateTimeoutRef.current = null;
      }
    };
  }, []);

  const handleStartEdit = useCallback(() => {
    if (disabled || isSaving) return;

    if (activateTimeoutRef.current) {
      clearTimeout(activateTimeoutRef.current);
      activateTimeoutRef.current = null;
    }

    setIsEditing(true);
    setEditValue(value);
    onEditStart?.();
  }, [disabled, isSaving, value, onEditStart]);

  const handleCancel = useCallback(() => {
    setIsEditing(false);
    setEditValue(originalValue.current);
    onEditEnd?.();
  }, [onEditEnd]);

  const handleSave = useCallback(async () => {
    if (isSaving) return;

    const trimmedValue = editValue.trim();

    // Check if value unchanged
    if (trimmedValue === originalValue.current) {
      handleCancel();
      return;
    }

    // Check if empty when not allowed
    if (!allowEmpty && !trimmedValue) {
      handleCancel();
      return;
    }

    setIsSaving(true);
    try {
      await onSave(trimmedValue);
      originalValue.current = trimmedValue;
      setIsEditing(false);
      onEditEnd?.();
    } catch (error) {
      const errMsg = errorMessage(error, 'Failed to save');
      console.error('InlineEditText save error:', errMsg);
      toast.error(errMsg);
      setEditValue(originalValue.current);
      handleCancel();
    } finally {
      setIsSaving(false);
    }
  }, [editValue, isSaving, allowEmpty, onSave, handleCancel, onEditEnd]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === 'Enter' && !isSaving) {
        e.preventDefault();
        handleSave();
      } else if (e.key === 'Escape' && !isSaving) {
        e.preventDefault();
        handleCancel();
      }
    },
    [handleSave, handleCancel, isSaving]
  );

  const handleBlur = useCallback(() => {
    if (!isSaving) {
      handleSave();
    }
  }, [handleSave, isSaving]);

  const handleChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setEditValue(e.target.value);
  }, []);

  const handleClick = useCallback(
    (e: React.MouseEvent) => {
      if (disabled || isSaving) return;

      if (singleClickEdit) {
        e.stopPropagation();
        handleStartEdit();
        return;
      }

      // If single-click edit is disabled, a click can be treated as an activation
      // (e.g. open sidebar session). Delay activation slightly so a double click
      // can still enter edit mode.
      if (onActivate) {
        e.stopPropagation();

        if (activateTimeoutRef.current) {
          clearTimeout(activateTimeoutRef.current);
        }

        activateTimeoutRef.current = setTimeout(() => {
          activateTimeoutRef.current = null;
          onActivate();
        }, activateDelayMs);
      }
    },
    [activateDelayMs, disabled, handleStartEdit, isSaving, onActivate, singleClickEdit]
  );

  const handleDoubleClick = useCallback(
    (e: React.MouseEvent) => {
      if (!singleClickEdit) {
        if (activateTimeoutRef.current) {
          clearTimeout(activateTimeoutRef.current);
          activateTimeoutRef.current = null;
        }
        e.stopPropagation();
        handleStartEdit();
      }
    },
    [singleClickEdit, handleStartEdit]
  );

  if (isEditing) {
    return (
      <input
        ref={inputRef}
        type="text"
        value={editValue}
        onChange={handleChange}
        onKeyDown={handleKeyDown}
        onBlur={handleBlur}
        maxLength={maxLength}
        placeholder={placeholder}
        disabled={isSaving}
        className={`
          w-full px-2 py-1 border rounded
          bg-background-default text-text-standard
          border-blue-500 ring-2 ring-blue-500/20
          focus:outline-none focus:ring-2 focus:ring-blue-500/40
          disabled:opacity-50 disabled:cursor-not-allowed
          ${editClassName}
        `}
        onClick={(e) => e.stopPropagation()}
      />
    );
  }

  return (
    <button
      type="button"
      className={`
        cursor-pointer px-2 py-1 rounded
        hover:bg-background-hover
        transition-colors
        ${disabled ? 'opacity-50 cursor-not-allowed' : ''}
        ${className}
      `}
      onClick={handleClick}
      onDoubleClick={handleDoubleClick}
      title={disabled ? '' : singleClickEdit ? 'Click to edit' : 'Double-click to edit'}
      disabled={disabled}
    >
      {value || <span className="text-text-subtle italic">{placeholder}</span>}
    </button>
  );
};
