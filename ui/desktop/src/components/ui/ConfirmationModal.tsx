import { BaseModal } from './BaseModal';
import React from 'react';

export function ConfirmationModal({
  isOpen,
  title,
  message,
  onConfirm,
  onCancel,
  confirmLabel = 'Yes',
  cancelLabel = 'No',
  isSubmitting = false,
}: {
  isOpen: boolean;
  title: string;
  message: string;
  onConfirm: () => void;
  onCancel: () => void;
  confirmLabel?: string;
  cancelLabel?: string;
  isSubmitting?: boolean; // To handle debounce state
}) {
  return (
    <BaseModal
      isOpen={isOpen}
      title={title}
      onClose={onCancel}
      actions={
        <>
          <button
            onClick={onConfirm}
            disabled={isSubmitting}
            className="w-full h-[60px] rounded-none border-t dark:border-gray-600 text-indigo-500 hover:bg-indigo-50 dark:hover:bg-indigo-900/20 dark:border-gray-600 text-lg font-regular"
          >
            {isSubmitting ? 'Processing...' : confirmLabel}
          </button>
          <button
            onClick={onCancel}
            disabled={isSubmitting}
            className="w-full h-[60px] rounded-none border-t dark:border-gray-600 text-gray-400 text-lg font-regular hover:bg-gray-50"
          >
            {cancelLabel}
          </button>
        </>
      }
    >
      <p className="text-sm text-gray-600 dark:text-gray-400 whitespace-pre-wrap">{message}</p>
    </BaseModal>
  );
}
