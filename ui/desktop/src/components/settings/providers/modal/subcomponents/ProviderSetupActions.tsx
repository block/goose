import { SyntheticEvent } from 'react';
import { Button } from '../../../../ui/button';
import { Trash2, AlertTriangle } from 'lucide-react';
import { ConfigKey } from '../../../../../api';

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
            className="w-full h-[60px] rounded-none hover:bg-bgSubtle text-textSubtle hover:text-textStandard text-md font-regular"
          >
            Ok
          </Button>
        </div>
      );
    }

    // Normal delete confirmation: show banner + horizontal footer with Cancel and Confirm
    return (
      <div className="w-full">
        <div className="w-full px-6 py-4 bg-red-900/20 border-t border-red-500/30">
          <p className="text-red-400 text-sm mb-2">
            Are you sure you want to delete the configuration parameters for {providerName}? This
            action cannot be undone.
          </p>
        </div>
        <div className="w-full flex items-center justify-between px-6 py-3 gap-4">
          <div className="flex-1">
            <Button
              onClick={onCancelDelete}
              variant="outline"
              className="w-full h-[44px] rounded-md"
            >
              Cancel
            </Button>
          </div>
          <div className="flex-1 flex justify-end">
            <Button
              onClick={onConfirmDelete}
              className="w-full h-[44px] rounded-md border-b border-borderSubtle bg-transparent hover:bg-red-900/20 text-red-500 font-medium text-md"
            >
              <Trash2 className="h-4 w-4 mr-2" /> Confirm Delete
            </Button>
          </div>
        </div>
      </div>
    );
  }

  // Regular buttons (with delete if applicable)
  // Layout: [Delete (left, red)] [Cancel (center)] [Submit (right)]
  return (
    <div className="w-full px-6 py-2">
      <div className="w-full flex items-center justify-between gap-4">
        <div className="flex-1">
          {canDelete && onDelete && (
            <Button
              type="button"
              onClick={onDelete}
              variant="outline"
              className="text-red-500 hover:text-red-600 w-full md:w-auto"
            >
              <Trash2 className="h-4 w-4 mr-2" /> Delete Provider
            </Button>
          )}
        </div>

        <div className="flex-1 flex justify-center">
          <Button type="button" variant="outline" onClick={onCancel} className="w-full md:w-auto">
            Cancel
          </Button>
        </div>
        <div className="flex-1 flex justify-end">
          {requiredParameters && requiredParameters.length > 0 ? (
            <Button type="submit" variant="default" onClick={onSubmit} className="w-full md:w-auto">
              Submit
            </Button>
          ) : (
            <Button type="submit" variant="default" onClick={onSubmit} className="w-full md:w-auto">
              Enable Provider
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
