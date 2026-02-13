/**
 * Theme Preset Gallery
 * 
 * Browse and apply built-in theme presets with one click
 */

import { useState, useEffect } from 'react';
import { Button } from '../../../../ui/button';
import { toast } from 'react-toastify';
import { ThemePreset } from '../../../../../themes/presets/types';
import { getThemePresets, applyThemePreset } from '../../../../../api';
import { useTheme } from '../../../../../contexts/ThemeContext';
import { Check, Download } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipTrigger } from '../../../../ui/Tooltip';

interface PresetGalleryProps {
  onApply?: () => void;
}

export function PresetGallery({ onApply }: PresetGalleryProps) {
  const { resolvedTheme } = useTheme();
  const [presets, setPresets] = useState<ThemePreset[]>([]);
  const [loading, setLoading] = useState(true);
  const [applying, setApplying] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedTag, setSelectedTag] = useState<string | null>(null);

  useEffect(() => {
    loadPresets();
  }, []);

  const loadPresets = async () => {
    try {
      setLoading(true);
      const response = await getThemePresets();
      setPresets(response.data?.presets || []);
    } catch (error) {
      console.error('Failed to load theme presets:', error);
      toast.error('Failed to load theme presets');
    } finally {
      setLoading(false);
    }
  };

  const handleApplyPreset = async (presetId: string) => {
    try {
      setApplying(presetId);
      
      await applyThemePreset({
        body: {
          preset_id: presetId,
        },
      });

      toast.success('Theme applied successfully! Reloading...');
      
      // Reload the page to apply changes
      setTimeout(() => {
        window.location.reload();
      }, 1000);
      
      onApply?.();
    } catch (error) {
      console.error('Failed to apply theme preset:', error);
      toast.error('Failed to apply theme');
      setApplying(null);
    }
  };

  // Get all unique tags
  const allTags = Array.from(
    new Set(presets.flatMap(preset => preset.tags))
  ).sort();

  // Filter presets
  const filteredPresets = presets.filter(preset => {
    const matchesSearch = searchQuery === '' || 
      preset.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      preset.description.toLowerCase().includes(searchQuery.toLowerCase());
    
    const matchesTag = !selectedTag || preset.tags.includes(selectedTag);
    
    return matchesSearch && matchesTag;
  });

  if (loading) {
    return (
      <div className="flex items-center justify-center p-8">
        <div className="text-text-secondary">Loading themes...</div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      {/* Filter Tags Only */}
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
          {allTags.map(tag => (
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

      {/* Theme Grid - Full Height */}
      <div className="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4 flex-1 overflow-y-auto pr-2">
        {filteredPresets.map(preset => {
          const isApplied = preset.id === 'goose-classic'; // TODO: Track which theme is currently applied
          
          return (
            <div
              key={preset.id}
              className="border border-border-primary rounded-lg p-4 flex flex-col hover:border-border-secondary transition-colors"
            >
              {/* Theme Preview Colors - Show only current mode (4 colors) */}
              <div className="grid grid-cols-4 h-8 rounded border border-border-primary overflow-hidden">
                <div 
                  style={{ backgroundColor: preset.colors[resolvedTheme]['color-background-primary'] }}
                />
                <div 
                  style={{ backgroundColor: preset.colors[resolvedTheme]['color-background-secondary'] }}
                />
                <div 
                  style={{ backgroundColor: preset.colors[resolvedTheme]['color-text-primary'] }}
                />
                <div 
                  style={{ backgroundColor: preset.colors[resolvedTheme]['color-background-inverse'] }}
                />
              </div>

              {/* Theme Info */}
              <div className="mt-3 flex-1">
                <h3 className="text-sm font-semibold text-text-primary">
                  {preset.name}
                </h3>
                <p className="text-xs text-text-secondary mt-1">
                  {preset.description}
                </p>
                <p className="text-xs text-text-secondary mt-1">
                  by {preset.author}
                </p>
              </div>

              {/* Tags */}
              <div className="flex flex-wrap gap-1 mt-3">
                {preset.tags.map(tag => (
                  <span
                    key={tag}
                    className="px-2 py-0.5 text-xs bg-background-secondary text-text-secondary rounded capitalize"
                  >
                    {tag}
                  </span>
                ))}
              </div>

              {/* Apply Button - Bottom Aligned */}
              <div className="mt-3">
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      onClick={() => handleApplyPreset(preset.id)}
                      disabled={applying !== null}
                      variant={isApplied ? 'default' : 'secondary'}
                      size="sm"
                      shape="round"
                      className="w-full"
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
              </div>
            </div>
          );
        })}
      </div>

      {filteredPresets.length === 0 && (
        <div className="text-center py-8 text-text-secondary">
          No themes found matching your search.
        </div>
      )}
    </div>
  );
}
