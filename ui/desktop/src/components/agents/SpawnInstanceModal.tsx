import React, { useState, useCallback } from 'react';
import { Rocket, AlertCircle } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
  DialogClose,
} from '../ui/dialog';
import type { SpawnInstanceRequest } from '../../api/instances';

interface SpawnInstanceModalProps {
  open: boolean;
  onClose: () => void;
  onSpawn: (req: SpawnInstanceRequest) => Promise<void>;
  /** Pre-fill persona from the catalog card */
  defaultPersona?: string;
  personas?: string[];
}

export function SpawnInstanceModal({
  open,
  onClose,
  onSpawn,
  defaultPersona = '',
  personas = [],
}: SpawnInstanceModalProps) {
  const [persona, setPersona] = useState(defaultPersona || '');
  const [instructions, setInstructions] = useState('');
  const [provider, setProvider] = useState('');
  const [model, setModel] = useState('');
  const [maxTurns, setMaxTurns] = useState<number | ''>('');
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showAdvanced, setShowAdvanced] = useState(false);

  // Reset form when modal opens with a new persona
  React.useEffect(() => {
    if (open) {
      setPersona(defaultPersona || '');
      setInstructions('');
      setProvider('');
      setModel('');
      setMaxTurns('');
      setError(null);
      setShowAdvanced(false);
    }
  }, [open, defaultPersona]);

  const handleSubmit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault();
      if (!persona.trim()) {
        setError('Persona is required');
        return;
      }

      setSubmitting(true);
      setError(null);

      try {
        const req: SpawnInstanceRequest = {
          persona: persona.trim(),
          instructions: instructions.trim() || undefined,
          provider: provider.trim() || undefined,
          model: model.trim() || undefined,
          max_turns: typeof maxTurns === 'number' && maxTurns > 0 ? maxTurns : undefined,
        };
        await onSpawn(req);
        onClose();
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to spawn instance');
      } finally {
        setSubmitting(false);
      }
    },
    [persona, instructions, provider, model, maxTurns, onSpawn, onClose]
  );

  return (
    <Dialog open={open} onOpenChange={(isOpen) => !isOpen && onClose()}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Rocket className="w-5 h-5 text-blue-500" />
            Deploy Agent Instance
          </DialogTitle>
          <DialogDescription>
            Spawn a new agent instance that runs independently with its own context.
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4 mt-2">
          {/* Persona */}
          <div>
            <label className="block text-sm font-medium mb-1.5">Persona</label>
            {personas.length > 0 ? (
              <select
                value={persona}
                onChange={(e) => setPersona(e.target.value)}
                className="w-full px-3 py-2 text-sm rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 focus:ring-2 focus:ring-blue-500 focus:border-transparent outline-none"
              >
                <option value="">Select a persona...</option>
                {personas.map((p) => (
                  <option key={p} value={p}>
                    {p}
                  </option>
                ))}
              </select>
            ) : (
              <input
                type="text"
                value={persona}
                onChange={(e) => setPersona(e.target.value)}
                placeholder="e.g. developer, planner"
                className="w-full px-3 py-2 text-sm rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 focus:ring-2 focus:ring-blue-500 focus:border-transparent outline-none"
                autoFocus
              />
            )}
          </div>

          {/* Instructions */}
          <div>
            <label className="block text-sm font-medium mb-1.5">Instructions</label>
            <textarea
              value={instructions}
              onChange={(e) => setInstructions(e.target.value)}
              placeholder="What should this agent do?"
              rows={3}
              className="w-full px-3 py-2 text-sm rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 focus:ring-2 focus:ring-blue-500 focus:border-transparent outline-none resize-y"
            />
          </div>

          {/* Advanced Options */}
          <div>
            <button
              type="button"
              onClick={() => setShowAdvanced(!showAdvanced)}
              className="text-xs text-gray-500 hover:text-gray-700 dark:hover:text-gray-300 transition-colors"
            >
              {showAdvanced ? '▾ Hide advanced' : '▸ Advanced options'}
            </button>

            {showAdvanced && (
              <div className="mt-3 space-y-3 pl-2 border-l-2 border-gray-200 dark:border-gray-700">
                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <label className="block text-xs font-medium mb-1 text-gray-500">Provider</label>
                    <input
                      type="text"
                      value={provider}
                      onChange={(e) => setProvider(e.target.value)}
                      placeholder="(inherit)"
                      className="w-full px-2.5 py-1.5 text-xs rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 outline-none"
                    />
                  </div>
                  <div>
                    <label className="block text-xs font-medium mb-1 text-gray-500">Model</label>
                    <input
                      type="text"
                      value={model}
                      onChange={(e) => setModel(e.target.value)}
                      placeholder="(inherit)"
                      className="w-full px-2.5 py-1.5 text-xs rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 outline-none"
                    />
                  </div>
                </div>
                <div>
                  <label className="block text-xs font-medium mb-1 text-gray-500">Max turns</label>
                  <input
                    type="number"
                    value={maxTurns}
                    onChange={(e) =>
                      setMaxTurns(e.target.value === '' ? '' : parseInt(e.target.value, 10))
                    }
                    placeholder="Unlimited"
                    min={1}
                    max={1000}
                    className="w-32 px-2.5 py-1.5 text-xs rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 outline-none"
                  />
                </div>
              </div>
            )}
          </div>

          {/* Error */}
          {error && (
            <div className="flex items-center gap-2 p-2.5 text-xs rounded-lg bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300">
              <AlertCircle className="w-3.5 h-3.5 shrink-0" />
              {error}
            </div>
          )}

          {/* Footer */}
          <DialogFooter className="gap-2">
            <DialogClose asChild>
              <button
                type="button"
                className="px-4 py-2 text-sm text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors"
              >
                Cancel
              </button>
            </DialogClose>
            <button
              type="submit"
              disabled={submitting || !persona.trim()}
              className="flex items-center gap-2 px-4 py-2 text-sm bg-blue-500 text-white rounded-lg hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              {submitting ? (
                <>
                  <span className="w-3.5 h-3.5 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                  Spawning...
                </>
              ) : (
                <>
                  <Rocket className="w-3.5 h-3.5" />
                  Deploy
                </>
              )}
            </button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
