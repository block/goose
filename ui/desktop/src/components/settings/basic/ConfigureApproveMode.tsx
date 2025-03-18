import React, { useEffect, useState } from 'react';
import { Card } from '../../ui/card';
import { Button } from '../../ui/button';
import { GooseMode, ModeSelectionItem } from './ModeSelectionItem';

interface ConfigureApproveModeProps {
  onClose: () => void;
  handleModeChange: (newMode: string) => void;
  currentMode: string | null;
}

export function ConfigureApproveMode({
  onClose,
  handleModeChange,
  currentMode,
}: ConfigureApproveModeProps) {
  const approveModes: GooseMode[] = [
    {
      key: 'approve',
      label: 'Manual Approval',
      description: 'All tools, extensions and file modificatio will require human approval',
    },
    {
      key: 'smart_approve',
      label: 'Smart Approval',
      description: 'Intelligently determine which actions need approval based on risk level ',
    },
  ];

  const [isSubmitting, setIsSubmitting] = useState(false);
  const [approveMode, setApproveMode] = useState(currentMode);

  useEffect(() => {
    setApproveMode(currentMode);
  }, [currentMode]);

  const handleModeSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    setIsSubmitting(true);
    try {
      handleModeChange(approveMode);
      onClose();
    } catch (error) {
      console.error('Error configuring goose mode:', error);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/20 backdrop-blur-sm">
      <Card className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[440px] bg-white dark:bg-gray-800 rounded-xl shadow-xl overflow-hidden p-[16px] pt-[24px] pb-0">
        <div className="px-4 pb-0 space-y-6">
          {/* Header */}
          <div className="flex">
            <h2 className="text-2xl font-regular dark:text-white text-gray-900">
              Configure Approve Mode
            </h2>
          </div>

          <div className="mt-[24px]">
            <p className="text-sm text-gray-500 dark:text-gray-400 mb-6">
              Approve requests can either be given to all tool requests or determine which actions
              may need integration
            </p>
            <div className="space-y-4">
              {approveModes.map((mode) => (
                <ModeSelectionItem
                  key={mode.key}
                  mode={mode}
                  showDescription={true}
                  currentMode={approveMode}
                  isApproveModeConfigure={true}
                  handleModeChange={(newMode) => {
                    setApproveMode(newMode);
                  }}
                />
              ))}
            </div>
          </div>

          {/* Actions */}
          <div className="mt-[8px] ml-[-24px] mr-[-24px] pt-[16px]">
            <Button
              type="submit"
              variant="ghost"
              disabled={isSubmitting}
              onClick={handleModeSubmit}
              className="w-full h-[60px] rounded-none border-t dark:border-gray-600 text-lg hover:bg-gray-50 hover:dark:text-black dark:text-white dark:border-gray-600 font-regular"
            >
              {isSubmitting ? 'Saving...' : 'Save Mode'}
            </Button>
            <Button
              type="button"
              variant="ghost"
              disabled={isSubmitting}
              onClick={onClose}
              className="w-full h-[60px] rounded-none border-t dark:border-gray-600 text-gray-400 hover:bg-gray-50 dark:border-gray-600 text-lg font-regular"
            >
              Cancel
            </Button>
          </div>
        </div>
      </Card>
    </div>
  );
}
