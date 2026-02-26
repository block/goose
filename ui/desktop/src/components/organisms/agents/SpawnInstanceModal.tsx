import { AlertCircle, ChevronDown, Rocket } from 'lucide-react';
import type React from 'react';
import { useCallback, useEffect, useState } from 'react';
import type { ProviderDetails } from '../../../api';
import { providers as fetchProviders, getProviderModels } from '../../../api';
import type { SpawnInstanceRequest } from '../../../lib/instances';
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../../molecules/ui/dialog';

interface SpawnInstanceModalProps {
  open: boolean;
  onClose: () => void;
  onSpawn: (req: SpawnInstanceRequest) => Promise<void>;
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
  const personaId = 'spawn-instance-persona';
  const instructionsId = 'spawn-instance-instructions';
  const providerId = 'spawn-instance-provider';
  const modelId = 'spawn-instance-model';
  const maxTurnsId = 'spawn-instance-max-turns';

  const [persona, setPersona] = useState(defaultPersona || '');
  const [instructions, setInstructions] = useState('');
  const [provider, setProvider] = useState('');
  const [model, setModel] = useState('');
  const [maxTurns, setMaxTurns] = useState<number | ''>('');
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showAdvanced, setShowAdvanced] = useState(false);

  // Provider/model dropdown state
  const [providerList, setProviderList] = useState<ProviderDetails[]>([]);
  const [modelList, setModelList] = useState<string[]>([]);
  const [modelsLoading, setModelsLoading] = useState(false);

  // Reset form when modal opens with a new persona
  useEffect(() => {
    if (open) {
      setPersona(defaultPersona || '');
      setInstructions('');
      setProvider('');
      setModel('');
      setMaxTurns('');
      setError(null);
      setShowAdvanced(false);
      setModelList([]);
    }
  }, [open, defaultPersona]);

  // Fetch providers on mount
  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    fetchProviders()
      .then((resp) => {
        if (!cancelled && resp.data) {
          setProviderList(resp.data);
        }
      })
      .catch(() => {
        // Providers fetch failed — allow manual text entry as fallback
      });
    return () => {
      cancelled = true;
    };
  }, [open]);

  // Fetch models when provider changes
  useEffect(() => {
    if (!provider) {
      setModelList([]);
      setModel('');
      return;
    }
    let cancelled = false;
    setModelsLoading(true);
    setModel('');
    getProviderModels({ path: { name: provider } })
      .then((resp) => {
        if (!cancelled && resp.data) {
          setModelList(resp.data);
        }
      })
      .catch(() => {
        if (!cancelled) setModelList([]);
      })
      .finally(() => {
        if (!cancelled) setModelsLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [provider]);

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

  const configuredProviders = providerList.filter((p) => p.is_configured);

  return (
    <Dialog open={open} onOpenChange={(isOpen) => !isOpen && onClose()}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Rocket className="w-5 h-5 text-accent-default" />
            Deploy Agent Instance
          </DialogTitle>
          <DialogDescription>
            Spawn a new agent instance that runs independently with its own context.
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4 mt-2">
          {/* Persona */}
          <div>
            <label htmlFor={personaId} className="block text-sm font-medium mb-1.5">
              Persona
            </label>
            {personas.length > 0 ? (
              <div className="relative">
                <select
                  id={personaId}
                  value={persona}
                  onChange={(e) => setPersona(e.target.value)}
                  className="w-full appearance-none px-3 py-2 pr-8 text-sm rounded-lg border border-border-default bg-background-default focus:ring-2 focus:ring-accent-default focus:border-transparent outline-none"
                >
                  <option value="">Select a persona...</option>
                  {personas.map((p) => (
                    <option key={p} value={p}>
                      {p}
                    </option>
                  ))}
                </select>
                <ChevronDown className="absolute right-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-text-muted pointer-events-none" />
              </div>
            ) : (
              <input
                id={personaId}
                type="text"
                value={persona}
                onChange={(e) => setPersona(e.target.value)}
                placeholder="e.g. developer, planner"
                className="w-full px-3 py-2 text-sm rounded-lg border border-border-default bg-background-default focus:ring-2 focus:ring-accent-default focus:border-transparent outline-none"
                autoFocus
              />
            )}
          </div>

          {/* Instructions */}
          <div>
            <label htmlFor={instructionsId} className="block text-sm font-medium mb-1.5">
              Instructions
            </label>
            <textarea
              id={instructionsId}
              value={instructions}
              onChange={(e) => setInstructions(e.target.value)}
              placeholder="What should this agent do?"
              rows={3}
              className="w-full px-3 py-2 text-sm rounded-lg border border-border-default bg-background-default focus:ring-2 focus:ring-accent-default focus:border-transparent outline-none resize-y"
            />
          </div>

          {/* Advanced Options */}
          <div>
            <button
              type="button"
              onClick={() => setShowAdvanced(!showAdvanced)}
              className="text-xs text-text-muted hover:text-text-default transition-colors"
            >
              {showAdvanced ? '▾ Hide advanced' : '▸ Advanced options'}
            </button>

            {showAdvanced && (
              <div className="mt-3 space-y-3 pl-2 border-l-2 border-border-muted">
                <div className="grid grid-cols-2 gap-3">
                  {/* Provider dropdown */}
                  <div>
                    <label htmlFor={providerId} className="block text-xs font-medium mb-1 text-text-muted">
                      Provider
                    </label>
                    {configuredProviders.length > 0 ? (
                      <div className="relative">
                        <select
                          id={providerId}
                          value={provider}
                          onChange={(e) => setProvider(e.target.value)}
                          className="w-full appearance-none px-2.5 py-1.5 pr-7 text-xs rounded-md border border-border-default bg-background-default outline-none focus:ring-1 focus:ring-accent-default"
                        >
                          <option value="">(inherit)</option>
                          {configuredProviders.map((p) => (
                            <option key={p.name} value={p.name}>
                              {p.name}
                            </option>
                          ))}
                        </select>
                        <ChevronDown className="absolute right-2 top-1/2 -translate-y-1/2 w-3 h-3 text-text-muted pointer-events-none" />
                      </div>
                    ) : (
                      <input
                        id={providerId}
                        type="text"
                        value={provider}
                        onChange={(e) => setProvider(e.target.value)}
                        placeholder="(inherit)"
                        className="w-full px-2.5 py-1.5 text-xs rounded-md border border-border-default bg-background-default outline-none"
                      />
                    )}
                  </div>

                  {/* Model dropdown */}
                  <div>
                    <label htmlFor={modelId} className="block text-xs font-medium mb-1 text-text-muted">
                      Model
                    </label>
                    {modelList.length > 0 ? (
                      <div className="relative">
                        <select
                          id={modelId}
                          value={model}
                          onChange={(e) => setModel(e.target.value)}
                          className="w-full appearance-none px-2.5 py-1.5 pr-7 text-xs rounded-md border border-border-default bg-background-default outline-none focus:ring-1 focus:ring-accent-default"
                        >
                          <option value="">(inherit)</option>
                          {modelList.map((m) => (
                            <option key={m} value={m}>
                              {m}
                            </option>
                          ))}
                        </select>
                        <ChevronDown className="absolute right-2 top-1/2 -translate-y-1/2 w-3 h-3 text-text-muted pointer-events-none" />
                      </div>
                    ) : modelsLoading ? (
                      <div className="flex items-center gap-1.5 px-2.5 py-1.5 text-xs text-text-muted">
                        <span className="w-3 h-3 border-2 border-text-muted/30 border-t-text-muted rounded-full animate-spin" />
                        Loading…
                      </div>
                    ) : (
                      <input
                        id={modelId}
                        type="text"
                        value={model}
                        onChange={(e) => setModel(e.target.value)}
                        placeholder={provider ? '(no models found)' : '(inherit)'}
                        className="w-full px-2.5 py-1.5 text-xs rounded-md border border-border-default bg-background-default outline-none"
                      />
                    )}
                  </div>
                </div>

                {/* Hint when provider selected but no models */}
                {provider && !modelsLoading && modelList.length === 0 && (
                  <p className="text-xs text-text-subtle italic">
                    No models found for {provider}. You can type a model name manually.
                  </p>
                )}

                <div>
                  <label htmlFor={maxTurnsId} className="block text-xs font-medium mb-1 text-text-muted">
                    Max turns
                  </label>
                  <input
                    id={maxTurnsId}
                    type="number"
                    value={maxTurns}
                    onChange={(e) =>
                      setMaxTurns(e.target.value === '' ? '' : parseInt(e.target.value, 10))
                    }
                    placeholder="Unlimited"
                    min={1}
                    max={1000}
                    className="w-32 px-2.5 py-1.5 text-xs rounded-md border border-border-default bg-background-default outline-none"
                  />
                </div>
              </div>
            )}
          </div>

          {/* Error */}
          {error && (
            <div className="flex items-center gap-2 p-2.5 text-xs rounded-lg bg-error-muted border border-error-default text-error-default">
              <AlertCircle className="w-3.5 h-3.5 shrink-0" />
              {error}
            </div>
          )}

          {/* Footer */}
          <DialogFooter className="gap-2">
            <DialogClose asChild>
              <button
                type="button"
                className="px-4 py-2 text-sm text-text-muted hover:bg-background-muted rounded-lg transition-colors"
              >
                Cancel
              </button>
            </DialogClose>
            <button
              type="submit"
              disabled={submitting || !persona.trim()}
              className="flex items-center gap-2 px-4 py-2 text-sm bg-accent-default text-white rounded-lg hover:bg-accent-default/90 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
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
