import { useState, useEffect, useCallback } from 'react';
import { RefreshCw, ExternalLink, Zap, Play } from 'lucide-react';
import { Button } from '../../ui/button';
import { Input } from '../../ui/input';
import { useModelAndProvider } from '../../ModelAndProviderContext';
import { useConfig } from '../../ConfigContext';
import { setConfigProvider } from '../../../api';

interface MeshModel {
  id: string;
  live: boolean;
}

const DEFAULT_PORT = 9337;
const DEFAULT_MODEL = 'GLM-4.7-Flash-Q4_K_M';

type MeshStatus = 'unknown' | 'checking' | 'running' | 'stopped' | 'starting';

export const MeshSettings = () => {
  const { currentModel, currentProvider, setProviderAndModel } = useModelAndProvider();
  const { read, upsert } = useConfig();
  const [meshStatus, setMeshStatus] = useState<MeshStatus>('unknown');
  const [models, setModels] = useState<MeshModel[]>([]);
  const [port, setPort] = useState<number>(DEFAULT_PORT);
  const [invite, setInvite] = useState<string>('');
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [saving, setSaving] = useState(false);
  const [startError, setStartError] = useState<string | null>(null);

  const isMeshSelected = currentProvider === 'mesh';
  const selectedModelId = isMeshSelected ? currentModel : null;

  const checkMeshStatus = useCallback(async () => {
    // Don't overwrite 'starting' state with 'checking'
    setMeshStatus((prev) => (prev === 'starting' ? prev : 'checking'));
    try {
      const result = await window.electron.checkMesh(port);
      if (result.running) {
        setMeshStatus('running');
        setModels(result.models.map((id) => ({ id, live: true })));
      } else {
        setMeshStatus((prev) => (prev === 'starting' ? prev : 'stopped'));
        setModels([]);
      }
    } catch {
      setMeshStatus((prev) => (prev === 'starting' ? prev : 'stopped'));
      setModels([]);
    }
  }, [port]);

  useEffect(() => {
    checkMeshStatus();
    const interval = setInterval(checkMeshStatus, meshStatus === 'starting' ? 3000 : 15000);
    return () => clearInterval(interval);
  }, [checkMeshStatus, meshStatus]);

  // Load saved config values
  useEffect(() => {
    (async () => {
      try {
        const savedPort = (await read('MESH_PORT', false)) as string;
        if (savedPort) setPort(parseInt(savedPort, 10) || DEFAULT_PORT);
      } catch {
        // use default
      }
      try {
        const savedInvite = (await read('MESH_INVITE', false)) as string;
        if (savedInvite) setInvite(savedInvite);
      } catch {
        // no invite configured
      }
    })();
  }, [read]);

  const selectModel = async (modelId: string) => {
    setSaving(true);
    try {
      setProviderAndModel('mesh', modelId);
      await setConfigProvider({
        body: { provider: 'mesh', model: modelId },
        throwOnError: true,
      });
    } catch (error) {
      console.error('Failed to select mesh model:', error);
    } finally {
      setSaving(false);
    }
  };

  const startMesh = async () => {
    setStartError(null);
    setMeshStatus('starting');
    try {
      const result = await window.electron.startMesh(port);
      if (!result.started) {
        setStartError(result.error || 'Failed to start mesh-llm');
        setMeshStatus('stopped');
      }
      // polling will pick up when it's ready
    } catch (error) {
      setStartError('Failed to start mesh-llm');
      setMeshStatus('stopped');
    }
  };

  const enableMesh = async () => {
    await selectModel(DEFAULT_MODEL);
    // Also start it right away if not running
    if (meshStatus !== 'running') {
      await startMesh();
    }
  };

  const saveAdvancedSettings = async () => {
    setSaving(true);
    try {
      await upsert('MESH_PORT', String(port), false);
      if (invite.trim()) {
        await upsert('MESH_INVITE', invite.trim(), false);
      }
    } catch (error) {
      console.error('Failed to save mesh settings:', error);
    } finally {
      setSaving(false);
    }
  };

  const statusIndicator = () => {
    switch (meshStatus) {
      case 'running':
        return (
          <span className="flex items-center gap-1.5 text-xs text-green-500">
            <span className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
            Running on port {port} — {models.length} model{models.length !== 1 ? 's' : ''}{' '}
            available
          </span>
        );
      case 'starting':
        return (
          <span className="flex items-center gap-1.5 text-xs text-yellow-500">
            <RefreshCw className="w-3 h-3 animate-spin" />
            Starting mesh-llm — this may take a minute if downloading a model...
          </span>
        );
      case 'stopped':
        return (
          <span className="flex items-center gap-1.5 text-xs text-text-muted">
            <span className="w-2 h-2 rounded-full bg-gray-400" />
            Not running
          </span>
        );
      case 'checking':
        return (
          <span className="flex items-center gap-1.5 text-xs text-text-muted">
            <RefreshCw className="w-3 h-3 animate-spin" />
            Checking...
          </span>
        );
      default:
        return null;
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <div className="flex items-center justify-between">
          <h3 className="text-text-default font-medium">Mesh Inference</h3>
          <a
            href="https://github.com/michaelneale/decentralized-inference"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center text-xs text-text-muted hover:text-text-default transition-colors"
          >
            <ExternalLink className="w-3 h-3 mr-1" />
            Learn more
          </a>
        </div>
        <p className="text-xs text-text-muted max-w-2xl mt-1">
          Decentralized local LLM inference via mesh-llm. Automatically downloads and runs models on
          your GPU — no API keys needed. Pool resources with others using an invite token.
        </p>
        <div className="mt-2">{statusIndicator()}</div>
        {startError && (
          <p className="text-xs text-red-400 mt-1">{startError}</p>
        )}
      </div>

      {/* Not selected: show enable button */}
      {!isMeshSelected && (
        <div className="border border-border-subtle rounded-xl p-4 bg-background-default">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-text-default">Switch to Mesh</p>
              <p className="text-xs text-text-muted mt-1">
                Use mesh-llm for inference. No API keys needed — runs on your GPU.
              </p>
            </div>
            <Button onClick={enableMesh} disabled={saving || meshStatus === 'starting'} size="sm">
              <Zap className="w-3 h-3 mr-1" />
              Use Mesh
            </Button>
          </div>
        </div>
      )}

      {/* Selected but not running: show start button */}
      {isMeshSelected && meshStatus === 'stopped' && (
        <div className="border border-accent-primary/30 rounded-xl p-4 bg-accent-primary/5">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-text-default">Mesh is your active provider</p>
              <p className="text-xs text-text-muted mt-1">
                Start mesh-llm to see available models and begin chatting.
              </p>
            </div>
            <Button onClick={startMesh} size="sm">
              <Play className="w-3 h-3 mr-1" />
              Start
            </Button>
          </div>
        </div>
      )}

      {/* Selected and starting */}
      {isMeshSelected && meshStatus === 'starting' && (
        <div className="border border-yellow-500/30 rounded-xl p-4 bg-yellow-500/5">
          <p className="text-sm font-medium text-text-default">Starting mesh-llm...</p>
          <p className="text-xs text-text-muted mt-1">
            Finding the mesh network and loading a model. This may take a minute on first run.
          </p>
        </div>
      )}

      {/* Live model list when running */}
      {meshStatus === 'running' && models.length > 0 && (
        <div>
          <h4 className="text-sm font-medium text-text-default mb-2">Available Models</h4>
          <div className="space-y-2">
            {models.map((model) => {
              const isSelected = selectedModelId === model.id;
              return (
                <div
                  key={model.id}
                  className={`border rounded-lg p-3 transition-colors cursor-pointer ${
                    isSelected
                      ? 'border-accent-primary bg-accent-primary/5'
                      : 'border-border-subtle bg-background-default hover:border-border-default'
                  }`}
                  onClick={() => !saving && selectModel(model.id)}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <input
                        type="radio"
                        checked={isSelected}
                        onChange={() => selectModel(model.id)}
                        className="cursor-pointer"
                        disabled={saving}
                      />
                      <span className="text-sm font-medium text-text-default">{model.id}</span>
                      <span className="text-xs text-green-500">live</span>
                    </div>
                    {isSelected && <span className="text-xs text-green-500">Active</span>}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {meshStatus === 'running' && models.length === 0 && (
        <p className="text-xs text-text-muted">
          Mesh is running but no models are available yet. A model may still be loading.
        </p>
      )}

      {/* Advanced Settings */}
      <div className="border-t border-border-subtle pt-4">
        <button
          onClick={() => setShowAdvanced(!showAdvanced)}
          className="text-sm text-text-muted hover:text-text-default transition-colors"
        >
          {showAdvanced ? '▾' : '▸'} Advanced Settings
        </button>

        {showAdvanced && (
          <div className="mt-3 space-y-4">
            <div>
              <label className="text-sm text-text-default block mb-1">Port</label>
              <Input
                type="number"
                value={port}
                onChange={(e) => setPort(parseInt(e.target.value, 10) || DEFAULT_PORT)}
                className="w-32"
                min={1024}
                max={65535}
              />
              <p className="text-xs text-text-muted mt-1">
                Port for mesh-llm API (default: {DEFAULT_PORT})
              </p>
            </div>

            <div>
              <label className="text-sm text-text-default block mb-1">Invite Token</label>
              <Input
                type="text"
                value={invite}
                onChange={(e) => setInvite(e.target.value)}
                placeholder="Optional — paste invite to join a mesh group"
                className="max-w-md"
              />
              <p className="text-xs text-text-muted mt-1">
                Join a decentralized group to pool GPU resources with others.
              </p>
            </div>

            <Button variant="outline" size="sm" onClick={saveAdvancedSettings} disabled={saving}>
              Save Settings
            </Button>
          </div>
        )}
      </div>

      {/* Refresh */}
      <div className="border-t border-border-subtle pt-4">
        <Button variant="outline" size="sm" onClick={checkMeshStatus}>
          <RefreshCw className="w-3 h-3 mr-1" />
          Refresh Status
        </Button>
      </div>
    </div>
  );
};
