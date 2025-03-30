import React from 'react';
import { Button } from '../../../../ui/button';
import { Trash2 } from 'lucide-react';

interface ProviderSetupActionsProps {
  onCancel: () => void;
  onSubmit: (e: any) => void;
  onDelete?: () => void;
  showDeleteConfirmation?: boolean;
  onConfirmDelete?: () => void;
  onCancelDelete?: () => void;
  canDelete?: boolean;
  providerName?: string;
}

/**
 * Renders the action buttons at the bottom of the provider modal.
 * Includes submit, cancel, and delete functionality with confirmation.
 */
export default function ProviderSetupActions({
  onCancel,
  onSubmit,
  onDelete,
  showDeleteConfirmation,
  onConfirmDelete,
  onCancelDelete,
  canDelete,
  providerName,
}: ProviderSetupActionsProps) {
  // If we're showing delete confirmation, render the delete confirmation buttons
  if (showDeleteConfirmation) {
    return (
      <>
        <div className="w-full px-6 py-4 bg-red-900/20 border-t border-red-500/30">
          <p className="text-red-400 text-sm mb-2">
            Are you sure you want to delete the configuration parameters for {providerName}? This
            action cannot be undone.
          </p>
        </div>
        <Button
          onClick={onConfirmDelete}
          className="w-full h-[60px] rounded-none border-b border-borderSubtle bg-transparent hover:bg-red-900/20 text-red-500 font-medium text-md"
        >
          <Trash2 className="h-4 w-4 mr-2" /> Confirm Delete
        </Button>
        <Button
          variant="ghost"
          onClick={onCancelDelete}
          className="w-full h-[60px] rounded-none hover:bg-bgSubtle text-textSubtle hover:text-textStandard text-md font-regular"
        >
          Cancel
        </Button>
      </>
    );
  }

  // Regular buttons (with delete if applicable)
  return (
    <div className="-ml-8 -mr-8">
      {canDelete && onDelete && (
        <Button
          type="button"
          onClick={onDelete}
          className="w-full h-[60px] rounded-none border-t border-borderSubtle bg-transparent hover:bg-bgSubtle text-red-500 font-medium text-md"
        >
          <Trash2 className="h-4 w-4 mr-2" /> Delete Provider
        </Button>
      )}
      <Button
        type="submit"
        variant="ghost"
        onClick={onSubmit}
        className="w-full h-[60px] rounded-none border-t border-borderSubtle text-md hover:bg-bgSubtle text-textProminent font-medium"
      >
        Submit
      </Button>
      <Button
        type="button"
        variant="ghost"
        onClick={onCancel}
        className="w-full h-[60px] rounded-none border-t border-borderSubtle hover:text-textStandard text-textSubtle hover:bg-bgSubtle text-md font-regular"
      >
        Cancel
      </Button>
    </div>
  );
}
