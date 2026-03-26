import React, { useState, useEffect } from "react";
import { useInput } from "ink";
import type { GooseClient } from "@aaif/goose-acp";
import { SelectModal, filterOptions } from "./modal.js";
import type { SelectOption } from "./modal.js";

interface ModelInfo {
  modelId: string;
  name: string;
  description?: string | null;
}

interface ProviderModelSwitcherProps {
  client: GooseClient;
  sessionId: string;
  width: number;
  onComplete: () => void;
  onCancel: () => void;
}

type SwitcherMode = "selecting-provider" | "selecting-model";

export function ProviderModelSwitcher({
  client,
  sessionId,
  width,
  onComplete,
  onCancel,
}: ProviderModelSwitcherProps) {
  const [mode, setMode] = useState<SwitcherMode>("selecting-provider");
  const [providers, setProviders] = useState<SelectOption[]>([]);
  const [models, setModels] = useState<SelectOption[]>([]);
  const [selectedIdx, setSelectedIdx] = useState(0);
  const [filter, setFilter] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedProvider, setSelectedProvider] = useState<string | null>(null);

  useEffect(() => {
    loadAvailableModels();
  }, [client, sessionId]);

  const loadAvailableModels = async () => {
    try {
      setLoading(true);
      setError(null);

      const sessionInfo = await client.goose.sessionGet({ sessionId });
      const session = sessionInfo.session as any;

      if (session?.models?.availableModels) {
        const modelInfos: ModelInfo[] = session.models.availableModels;
        
        // Extract unique providers from model IDs
        const providerSet = new Set<string>();
        const modelsByProvider = new Map<string, ModelInfo[]>();

        for (const model of modelInfos) {
          // Model IDs are typically in format "provider/model-name"
          const parts = model.modelId.split("/");
          const provider = parts.length > 1 ? parts[0]! : "default";
          
          providerSet.add(provider);
          
          if (!modelsByProvider.has(provider)) {
            modelsByProvider.set(provider, []);
          }
          modelsByProvider.get(provider)!.push(model);
        }

        const providerOptions: SelectOption[] = Array.from(providerSet).map(
          (p) => ({
            id: p,
            name: p,
            description: `${modelsByProvider.get(p)?.length || 0} models available`,
          }),
        );

        setProviders(providerOptions);

        // If only one provider, skip to model selection
        if (providerOptions.length === 1) {
          const singleProvider = providerOptions[0]!.id;
          setSelectedProvider(singleProvider);
          setModels(
            modelsByProvider.get(singleProvider)!.map((m) => ({
              id: m.modelId,
              name: m.name,
              description: m.description || undefined,
            })),
          );
          setMode("selecting-model");
        }
      } else {
        setError("No models available in session");
      }
    } catch (e: unknown) {
      const errMsg = e instanceof Error ? e.message : String(e);
      setError(`Failed to load models: ${errMsg}`);
    } finally {
      setLoading(false);
    }
  };

  const handleProviderSelect = (providerId: string) => {
    setSelectedProvider(providerId);
    
    // Load models for this provider
    const sessionInfo = client.goose.sessionGet({ sessionId });
    sessionInfo.then((info) => {
      const session = info.session as any;
      if (session?.models?.availableModels) {
        const modelInfos: ModelInfo[] = session.models.availableModels;
        const providerModels = modelInfos.filter((m) =>
          m.modelId.startsWith(`${providerId}/`),
        );
        
        setModels(
          providerModels.map((m) => ({
            id: m.modelId,
            name: m.name,
            description: m.description || undefined,
          })),
        );
        setMode("selecting-model");
        setSelectedIdx(0);
        setFilter("");
      }
    });
  };

  const handleModelSelect = async (modelId: string) => {
    try {
      await client.unstable_setSessionModel({ sessionId, modelId });
      onComplete();
    } catch (e: unknown) {
      const errMsg = e instanceof Error ? e.message : String(e);
      setError(`Failed to set model: ${errMsg}`);
    }
  };

  useInput((ch, key) => {
    if (key.escape) {
      if (mode === "selecting-model" && providers.length > 1) {
        // Go back to provider selection
        setMode("selecting-provider");
        setSelectedIdx(0);
        setFilter("");
        return;
      }
      onCancel();
      return;
    }

    const currentOptions =
      mode === "selecting-provider" ? providers : models;
    const filtered = filterOptions(currentOptions, filter);

    if (key.upArrow) {
      setSelectedIdx((i) => (i - 1 + filtered.length) % filtered.length);
      return;
    }

    if (key.downArrow) {
      setSelectedIdx((i) => (i + 1) % filtered.length);
      return;
    }

    if (key.return) {
      const selected = filtered[selectedIdx];
      if (selected) {
        if (mode === "selecting-provider") {
          handleProviderSelect(selected.id);
        } else {
          handleModelSelect(selected.id);
        }
      }
      return;
    }

    if (key.backspace || key.delete) {
      setFilter((f) => f.slice(0, -1));
      setSelectedIdx(0);
      return;
    }

    if (ch && ch.length === 1 && !key.ctrl && !key.meta) {
      setFilter((f) => f + ch);
      setSelectedIdx(0);
    }
  });

  if (loading) {
    return (
      <SelectModal
        title="Loading models..."
        options={[]}
        selectedIdx={0}
        filter=""
        width={width}
      />
    );
  }

  if (error) {
    return (
      <SelectModal
        title={`Error: ${error}`}
        options={[]}
        selectedIdx={0}
        filter=""
        width={width}
      />
    );
  }

  const title =
    mode === "selecting-provider"
      ? "Select Provider"
      : `Select Model (${selectedProvider})`;
  const options = mode === "selecting-provider" ? providers : models;

  return (
    <SelectModal
      title={title}
      options={options}
      selectedIdx={selectedIdx}
      filter={filter}
      width={width}
    />
  );
}

export { type SwitcherMode };
