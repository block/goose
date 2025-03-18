import React from 'react';
import { Button } from '../../../../ui/button';

interface ProviderSetupActionsProps {
  onCancel: () => void;
  onSubmit: (e: any) => void;
}

/**
 * Renders the "Submit" and "Cancel" buttons at the bottom.
 * Updated to match the design from screenshots.
 */
export default function ProviderSetupActions({ onCancel, onSubmit }: ProviderSetupActionsProps) {
  return (
    <div className="-ml-8 -mr-8">
      {/* We rely on the <form> "onSubmit" for the actual Submit logic */}
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
