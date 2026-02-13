/**
 * Theme Preset Gallery
 * 
 * Browse and apply built-in theme presets with one click
 */

import { useState, useEffect } from 'react';
import { Button } from '../../../../ui/button';
import { toast } from 'react-toastify';
import { ThemePreset } from '../../../../../themes/presets/types';

interface PresetGalleryProps {
  onApply?: () => void;
}

export function PresetGallery({ onApply }: PresetGalleryProps) {
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
      const response = await fetch('/theme/presets');
      const data = await response.json();
      setPresets(data.presets || []);
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
      
      const response = await fetch('/theme/apply-preset', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ preset_id: presetId }),
      });

      if (!response.ok) {
        throw new Error('Failed to apply theme');
      }

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
    <div className="space-y-4">
      {/* Search and Filter */}
      <div className="space-y-3">
        <input
          type="text"
          placeholder="Search themes..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="w-full px-3 py-2 border border-border-primary rounded-lg bg-background-primary text-text-primary text-sm"
        />
        
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

      {/* Theme Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 max-h-[500px] overflow-y-auto pr-2">
        {filteredPresets.map(preset => (
          <div
            key={preset.id}
            className="border border-border-primary rounded-lg p-4 space-y-3 hover:border-border-secondary transition-colors"
          >
            {/* Theme Preview Colors */}
            <div className="flex gap-2 h-12">
              <div className="flex-1 rounded border border-border-primary overflow-hidden">
                <div 
                  className="h-1/2" 
                  style={{ backgroundColor: preset.colors.light['color-background-primary'] }}
                />
                <div 
                  className="h-1/2" 
                  style={{ backgroundColor: preset.colors.light['color-background-secondary'] }}
                />
              </div>
              <div className="flex-1 rounded border border-border-primary overflow-hidden">
                <div 
                  className="h-1/2" 
                  style={{ backgroundColor: preset.colors.dark['color-background-primary'] }}
                />
                <div 
                  className="h-1/2" 
                  style={{ backgroundColor: preset.colors.dark['color-background-secondary'] }}
                />
              </div>
            </div>

            {/* Theme Info */}
            <div>
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
            <div className="flex flex-wrap gap-1">
              {preset.tags.map(tag => (
                <span
                  key={tag}
                  className="px-2 py-0.5 text-xs bg-background-secondary text-text-secondary rounded capitalize"
                >
                  {tag}
                </span>
              ))}
            </div>

            {/* Apply Button */}
            <Button
              onClick={() => handleApplyPreset(preset.id)}
              disabled={applying !== null}
              className="w-full"
              size="sm"
            >
              {applying === preset.id ? 'Applying...' : 'Apply Theme'}
            </Button>
          </div>
        ))}
      </div>

      {filteredPresets.length === 0 && (
        <div className="text-center py-8 text-text-secondary">
          No themes found matching your search.
        </div>
      )}
    </div>
  );
}
