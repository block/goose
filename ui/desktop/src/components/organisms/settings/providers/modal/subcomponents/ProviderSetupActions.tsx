import { AlertTriangle, Trash2 } from 'lucide-react';
import type { SyntheticEvent } from 'react';
import type { ConfigKey } from '@/api';
import { Button } from '@/components/atoms/button';

interface ProviderSetupActionsProps {
  onCancel: () => void;
  onSubmit: (e: SyntheticEvent) => void;
  onDelete?: () => void;
  showDeleteConfirmation?: boolean;
  onConfirmDelete?: () => void;
  onCancelDelete?: () => void;
  canDelete?: boolean;
  providerName?: string;
  requiredParameters?: ConfigKey[];
  isActiveProvider?: boolean; // Made optional with default false
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
  requiredParameters,
  isActiveProvider = false, // Default value provided
}: ProviderSetupActionsProps) {
  // If we're showing delete confirmation, render the delete confirmation buttons
  if (showDeleteConfirmation) {
    // Check if this is the active provider
    if (isActiveProvider) {
      return (
        <div className="w-full">
          <div className="w-full px-6 py-4 bg-yellow-600/20 border-t border-yellow-500/30">
            <p className="text-yellow-500 text-sm mb-2 flex items-start">
              <AlertTriangle className="h-4 w-4 mr-2 mt-0.5 flex-shrink-0" />
              <span>
                You cannot delete {providerName} while it's currently in use. Please switch to a
                different model before deleting this provider.
              </span>
            </p>
          </div>
          <Button
            variant="ghost"
            onClick={onCancelDelete}
            className="w-full h-[60px] rounded-none hover:bg-background-muted text-text-muted hover:text-text-default text-md font-regular"
          >
            Ok
          </Button>
        </div>
      );
    }

    // Normal delete confirmation
    return (
      <div className="w-full">
        <div className="w-full px-6 py-4 bg-red-900/20 border-t border-red-500/30">
          <p className="text-red-400 text-sm mb-2">
            Are you sure you want to delete the configuration parameters for {providerName}? This
            action cannot be undone.
          </p>
        </div>
        <Button
          onClick={onConfirmDelete}
          className="w-full h-[60px] rounded-none border-b border-border-default bg-transparent hover:bg-red-900/20 text-red-500 font-medium text-md"
        >
          <Trash2 className="h-4 w-4 mr-2" /> Confirm Delete
        </Button>
        <Button
          variant="ghost"
          onClick={onCancelDelete}
          className="w-full h-[60px] rounded-none hover:bg-background-muted text-text-muted hover:text-text-default text-md font-regular"
        >
          Cancel
        </Button>
      </div>
    );
  }

  // Regular buttons (with delete if applicable)
  return (
    <div className="w-full">
      {canDelete && onDelete && (
        <Button
          type="button"
          onClick={onDelete}
          className="w-full h-[60px] rounded-none border-t border-border-default bg-transparent hover:bg-background-muted text-red-500 font-medium text-md"
        >
          <Trash2 className="h-4 w-4 mr-2" /> Delete Provider
        </Button>
      )}
      {requiredParameters && requiredParameters.length > 0 ? (
        <>
          <Button
            type="submit"
            variant="ghost"
            onClick={onSubmit}
            className="w-full h-[60px] rounded-none border-t border-border-default text-md hover:bg-background-muted text-text-default font-medium"
          >
            Submit
          </Button>
          <Button
            type="button"
            variant="ghost"
            onClick={onCancel}
            className="w-full h-[60px] rounded-none border-t border-border-default hover:text-text-default text-text-muted hover:bg-background-muted text-md font-regular"
          >
            Cancel
          </Button>
        </>
      ) : (
        <>
          <Button
            type="submit"
            variant="ghost"
            onClick={onSubmit}
            className="w-full h-[60px] rounded-none border-t border-border-default text-md hover:bg-background-muted text-text-default font-medium"
          >
            Enable Provider
          </Button>
          <Button
            type="button"
            variant="ghost"
            onClick={onCancel}
            className="w-full h-[60px] rounded-none border-t border-border-default hover:text-text-default text-text-muted hover:bg-background-muted text-md font-regular"
          >
            Cancel
          </Button>
        </>
      )}
    </div>
  );
}
