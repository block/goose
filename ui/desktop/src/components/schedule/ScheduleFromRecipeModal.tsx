import React, { useState, useEffect } from 'react';
import { Card } from '../ui/card';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Recipe, generateDeepLink } from '../../recipe';
import Copy from '../icons/Copy';
import { Check } from 'lucide-react';

interface ScheduleFromRecipeModalProps {
  isOpen: boolean;
  onClose: () => void;
  recipe: Recipe;
  onCreateSchedule: (deepLink: string) => void;
}

export const ScheduleFromRecipeModal: React.FC<ScheduleFromRecipeModalProps> = ({
  isOpen,
  onClose,
  recipe,
  onCreateSchedule,
}) => {
  const [copied, setCopied] = useState(false);
  const [deepLink, setDeepLink] = useState('');

  useEffect(() => {
    let isCancelled = false;

    const generateLink = async () => {
      if (isOpen && recipe) {
        try {
          const link = await generateDeepLink(recipe);
          if (!isCancelled) {
            setDeepLink(link);
          }
        } catch (error) {
          console.error('Failed to generate deeplink:', error);
          if (!isCancelled) {
            setDeepLink('Error generating deeplink');
          }
        }
      }
    };

    generateLink();

    return () => {
      isCancelled = true;
    };
  }, [isOpen, recipe]);

  const handleCopy = () => {
    navigator.clipboard
      .writeText(deepLink)
      .then(() => {
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      })
      .catch((err) => {
        console.error('Failed to copy the text:', err);
      });
  };

  const handleCreateSchedule = () => {
    onCreateSchedule(deepLink);
    onClose();
  };

  const handleClose = () => {
    setCopied(false);
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 z-40 flex items-center justify-center p-4">
      <Card className="w-full max-w-md bg-background-default shadow-xl rounded-lg z-50 flex flex-col">
        <div className="px-6 pt-6 pb-4">
          <h2 className="text-xl font-semibold text-gray-900 dark:text-white">
            Create Schedule from Recipe
          </h2>
          <p className="text-sm text-gray-600 dark:text-gray-400 mt-2">
            Create a scheduled task using this recipe configuration.
          </p>
        </div>

        <div className="px-6 py-4 space-y-4">
          <div>
            <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Recipe Details:
            </h3>
            <div className="bg-gray-50 dark:bg-gray-800 p-3 rounded-md">
              <p className="text-sm font-medium text-gray-900 dark:text-white">{recipe.title}</p>
              <p className="text-xs text-gray-600 dark:text-gray-400 mt-1">{recipe.description}</p>
            </div>
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Recipe Deep Link:
            </label>
            <div className="flex items-center">
              <Input type="text" value={deepLink} readOnly className="flex-1 text-xs font-mono" />
              <Button
                type="button"
                onClick={handleCopy}
                className="ml-2 px-3 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600 flex items-center"
              >
                {copied ? <Check className="w-4 h-4" /> : <Copy className="w-4 h-4" />}
              </Button>
            </div>
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
              This link contains your recipe configuration and can be used to create a schedule.
            </p>
          </div>
        </div>

        <div className="px-6 pb-6 flex gap-2">
          <Button
            type="button"
            variant="outline"
            onClick={handleClose}
            className="flex-1 rounded-xl hover:bg-bgSubtle text-textSubtle"
          >
            Cancel
          </Button>
          <Button
            type="button"
            onClick={handleCreateSchedule}
            className="flex-1 bg-background-defaultInverse text-sm text-textProminentInverse rounded-xl hover:bg-bgStandardInverse transition-colors"
          >
            Create Schedule
          </Button>
        </div>
      </Card>
    </div>
  );
};
