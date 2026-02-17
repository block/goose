/**
 * Theme Preset Gallery
 *
 * Browse and apply built-in + custom theme presets.
 * Fully client-side — reads from the TS preset registry and localStorage.
 */

import { useState, useMemo } from 'react';
import { Button } from '../../../../ui/button';
import { toast } from 'react-toastify';
import type { ThemePreset } from '../../../../../themes/presets/types';
import {
  getAllPresets,
  deleteCustomTheme,
} from '../../../../../themes/presets';
import { useTheme } from '../../../../../contexts/ThemeContext';
import { Check, Download, Trash2, Sliders } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipTrigger } from '../../../../ui/Tooltip';

interface PresetGalleryProps {
  onApply?: () => void;
  onEdit?: (preset: ThemePreset) => void;
}

export function PresetGallery({ onApply, onEdit }: PresetGalleryProps) {
  const { resolvedTheme, applyPreset, activePresetId } = useTheme();
  const [applying, setApplying] = useState<string | null>(null);
  const [selectedTag, setSelectedTag] = useState<string | null>(null);
  const [presets, setPresets] = useState(() => getAllPresets());
  const allTags = useMemo(() => {
    const tags = new Set<string>();
    presets.forEach((p) => p.tags.forEach((t) => tags.add(t)));
    return Array.from(tags).sort();
  }, [presets]);

  const filteredPresets = useMemo(() => {
    if (!selectedTag) return presets;
    return presets.filter((p) => p.tags.includes(selectedTag));
  }, [presets, selectedTag]);

  const handleApplyPreset = (presetId: string) => {
    setApplying(presetId);
    applyPreset(presetId);
    toast.success('Theme applied!');
    setApplying(null);
    onApply?.();
  };

  const handleDeleteTheme = (themeId: string, themeName: string) => {
    if (!window.confirm(`Are you sure you want to delete "${themeName}"? This cannot be undone.`)) {
      return;
    }

    deleteCustomTheme(themeId);

    // If the deleted theme was active, reset to default
    if (activePresetId === themeId) {
      applyPreset(null);
    }

    toast.success(`Theme "${themeName}" deleted`);
    setPresets(getAllPresets());
  };

  return (
    <div className="h-full flex flex-col">
      {/* Filter Tags */}
      <div className="flex-shrink-0 mb-4">
        <div className="flex flex-wrap gap-2">
          <button
            onClick={() => setSelectedTag(null)}
            className={`px-3 py-1 text-xs rounded-full transition-colors ${
              selectedTag === null
                ? 'bg-background-inverse text-text-inverse'
                : 'bg-background-secondary text-text-secondary hover:bg-background-tertiary'
            }`}
          >
            All
          </button>
          {allTags.map((tag) => (
            <button
              key={tag}
              onClick={() => setSelectedTag(tag)}
              className={`px-3 py-1 text-xs rounded-full transition-colors capitalize ${
                selectedTag === tag
                  ? 'bg-background-inverse text-text-inverse'
                  : 'bg-background-secondary text-text-secondary hover:bg-background-tertiary'
              }`}
            >
              {tag}
            </button>
          ))}
        </div>
      </div>

      {/* Theme Grid */}
      <div className="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4 flex-1 overflow-y-auto pr-2">
        {filteredPresets.map((preset) => {
          const isApplied = preset.id === activePresetId;
          const isCustom = preset.isCustom || preset.tags.includes('custom');

          return (
            <div
              key={preset.id}
              className={`border rounded-lg p-4 flex flex-col hover:border-border-secondary transition-colors ${
                isApplied ? 'border-border-info border-2' : 'border-border-primary'
              }`}
            >
              {/* Theme Preview Colors */}
              <div className="grid grid-cols-4 h-24 rounded border border-border-primary overflow-hidden">
                <div
                  style={{
                    backgroundColor:
                      preset.colors[resolvedTheme]?.['color-background-primary'] ?? '#888',
                  }}
                />
                <div
                  style={{
                    backgroundColor:
                      preset.colors[resolvedTheme]?.['color-background-secondary'] ?? '#888',
                  }}
                />
                <div
                  style={{
                    backgroundColor:
                      preset.colors[resolvedTheme]?.['color-text-primary'] ?? '#888',
                  }}
                />
                <div
                  style={{
                    backgroundColor:
                      preset.colors[resolvedTheme]?.['color-background-inverse'] ?? '#888',
                  }}
                />
              </div>

              <div className="flex-1" />

              {/* Bottom-aligned content */}
              <div className="space-y-3 mt-3">
                <div>
                  <h3 className="text-sm font-semibold text-text-primary">{preset.name}</h3>
                  <p className="text-xs text-text-secondary mt-1">{preset.description}</p>
                  <p className="text-xs text-text-secondary mt-1">by {preset.author}</p>
                </div>

                <div className="flex flex-wrap gap-1">
                  {preset.tags.map((tag) => (
                    <span
                      key={tag}
                      className="px-2 py-0.5 text-xs bg-background-secondary text-text-secondary rounded capitalize"
                    >
                      {tag}
                    </span>
                  ))}
                </div>

                <div className="flex gap-2">
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <Button
                        onClick={() => handleApplyPreset(preset.id)}
                        disabled={applying !== null}
                        variant={isApplied ? 'default' : 'secondary'}
                        size="sm"
                        shape="round"
                        className="flex-1"
                      >
                        {isApplied ? (
                          <Check className="w-4 h-4" />
                        ) : (
                          <Download className="w-4 h-4" />
                        )}
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent>
                      {applying === preset.id
                        ? 'Applying...'
                        : isApplied
                          ? 'Currently Applied'
                          : 'Apply Theme'}
                    </TooltipContent>
                  </Tooltip>

                  {isCustom && (
                    <>
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <Button
                            onClick={() => onEdit?.(preset)}
                            disabled={applying !== null}
                            variant="outline"
                            size="sm"
                            shape="round"
                          >
                            <Sliders className="w-4 h-4" />
                          </Button>
                        </TooltipTrigger>
                        <TooltipContent>Edit Theme</TooltipContent>
                      </Tooltip>

                      <Tooltip>
                        <TooltipTrigger asChild>
                          <Button
                            onClick={() => handleDeleteTheme(preset.id, preset.name)}
                            disabled={applying !== null}
                            variant="outline"
                            size="sm"
                            shape="round"
                            className="text-text-danger hover:bg-background-danger"
                          >
                            <Trash2 className="w-4 h-4" />
                          </Button>
                        </TooltipTrigger>
                        <TooltipContent>Delete Theme</TooltipContent>
                      </Tooltip>
                    </>
                  )}
                </div>
              </div>
            </div>
          );
        })}
      </div>

      {filteredPresets.length === 0 && (
        <div className="text-center py-8 text-text-secondary">
          No themes found matching your filter.
        </div>
      )}
    </div>
  );
}
