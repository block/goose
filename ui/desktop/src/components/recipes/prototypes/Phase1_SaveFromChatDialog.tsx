/**
 * Phase 1: Discovery - Save from Chat Dialog
 *
 * This dialog appears when:
 * 1. AI detects user gave detailed instructions in chat
 * 2. User explicitly says "save this as recipe"
 * 3. User clicks a "Save as Recipe" button
 *
 * Features:
 * - AI extracts recipe from conversation
 * - Auto-preview shows first message
 * - Optional one-turn test
 * - Edit button leads to full builder
 */

import React, { useState } from 'react';
import { X, Sparkles, Play, Edit, Save } from 'lucide-react';

// Mock extracted recipe data (in real implementation, AI would extract this)
const mockExtractedRecipe = {
  name: 'Email Writer',
  description: 'Professional email writing assistant',
  behaviors: [
    'Writes in professional but friendly tone',
    'Asks what the email is about first',
    'No emojis',
    'Keeps emails concise',
  ],
  preview: "Hi! What email do you need help with today? I'll ask a few questions before drafting.",
};

interface Phase1_SaveFromChatDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: (recipe: typeof mockExtractedRecipe) => void;
  onEdit: (recipe: typeof mockExtractedRecipe) => void;
}

export default function Phase1_SaveFromChatDialog({
  isOpen,
  onClose,
  onSave,
  onEdit,
}: Phase1_SaveFromChatDialogProps) {
  const [name, setName] = useState(mockExtractedRecipe.name);
  const [testInput, setTestInput] = useState('');
  const [testResponse, setTestResponse] = useState<string | null>(null);
  const [isTesting, setIsTesting] = useState(false);

  if (!isOpen) return null;

  const handleTest = () => {
    if (!testInput.trim()) return;
    setIsTesting(true);
    // Simulate AI response
    setTimeout(() => {
      setTestResponse(
        `Great! I'll help you write an email about "${testInput}". A few quick questions:\n\n• Who is this email to?\n• What tone would you prefer?\n• Any specific points to include?`
      );
      setIsTesting(false);
    }, 1000);
  };

  const handleSave = () => {
    onSave({ ...mockExtractedRecipe, name });
  };

  const handleEdit = () => {
    onEdit({ ...mockExtractedRecipe, name });
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white dark:bg-gray-900 rounded-lg shadow-xl w-full max-w-lg mx-4">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-gray-200 dark:border-gray-700">
          <div className="flex items-center gap-2">
            <Sparkles className="w-5 h-5 text-purple-500" />
            <h2 className="text-lg font-semibold">Create Recipe from This Chat</h2>
          </div>
          <button
            onClick={onClose}
            className="p-1 hover:bg-gray-100 dark:hover:bg-gray-800 rounded"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="p-4 space-y-4">
          {/* Name Input */}
          <div>
            <label className="block text-sm font-medium mb-1">Name</label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800"
              placeholder="Recipe name"
            />
          </div>

          {/* What it does */}
          <div className="bg-gray-50 dark:bg-gray-800 rounded-lg p-3">
            <h3 className="text-sm font-medium mb-2">What it does:</h3>
            <ul className="space-y-1">
              {mockExtractedRecipe.behaviors.map((behavior, index) => (
                <li key={index} className="text-sm text-gray-600 dark:text-gray-400 flex items-start gap-2">
                  <span className="text-green-500 mt-0.5">•</span>
                  {behavior}
                </li>
              ))}
            </ul>
          </div>

          {/* Preview */}
          <div className="border border-gray-200 dark:border-gray-700 rounded-lg">
            <div className="px-3 py-2 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
              <h3 className="text-sm font-medium">Preview - how it will start:</h3>
            </div>
            <div className="p-3">
              <div className="bg-blue-50 dark:bg-blue-900/20 rounded-lg p-3 mb-3">
                <p className="text-sm">
                  <span className="font-medium">AI:</span> "{mockExtractedRecipe.preview}"
                </p>
              </div>

              {/* Test Response */}
              {testResponse && (
                <div className="bg-blue-50 dark:bg-blue-900/20 rounded-lg p-3 mb-3">
                  <p className="text-sm whitespace-pre-wrap">
                    <span className="font-medium">AI:</span> {testResponse}
                  </p>
                </div>
              )}

              {/* Test Input */}
              <div>
                <label className="block text-xs text-gray-500 mb-1">
                  Try it (optional):
                </label>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={testInput}
                    onChange={(e) => setTestInput(e.target.value)}
                    onKeyDown={(e) => e.key === 'Enter' && handleTest()}
                    className="flex-1 px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800"
                    placeholder="Type a message to test..."
                  />
                  <button
                    onClick={handleTest}
                    disabled={!testInput.trim() || isTesting}
                    className="px-3 py-2 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 rounded-md disabled:opacity-50"
                  >
                    <Play className="w-4 h-4" />
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="flex justify-end gap-2 p-4 border-t border-gray-200 dark:border-gray-700">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-md"
          >
            Cancel
          </button>
          <button
            onClick={handleEdit}
            className="px-4 py-2 text-sm border border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-md flex items-center gap-2"
          >
            <Edit className="w-4 h-4" />
            Edit
          </button>
          <button
            onClick={handleSave}
            className="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-700 text-white rounded-md flex items-center gap-2"
          >
            <Save className="w-4 h-4" />
            Save
          </button>
        </div>
      </div>
    </div>
  );
}

/**
 * Demo wrapper to show the dialog
 */
export function Phase1_Demo() {
  const [isOpen, setIsOpen] = useState(true);

  return (
    <div className="p-8">
      <h1 className="text-2xl font-bold mb-4">Phase 1: Discovery - Save from Chat</h1>
      <p className="text-gray-600 mb-4">
        This dialog appears when the AI detects detailed instructions in a chat,
        or when the user explicitly wants to save as a recipe.
      </p>

      <button
        onClick={() => setIsOpen(true)}
        className="px-4 py-2 bg-purple-600 text-white rounded-md flex items-center gap-2"
      >
        <Sparkles className="w-4 h-4" />
        Show "Save as Recipe" Dialog
      </button>

      <Phase1_SaveFromChatDialog
        isOpen={isOpen}
        onClose={() => setIsOpen(false)}
        onSave={(recipe) => {
          console.log('Saved recipe:', recipe);
          setIsOpen(false);
          alert(`Recipe "${recipe.name}" saved!`);
        }}
        onEdit={(recipe) => {
          console.log('Edit recipe:', recipe);
          setIsOpen(false);
          alert(`Opening editor for "${recipe.name}"...`);
        }}
      />
    </div>
  );
}
