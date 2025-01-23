import React, { useState, useEffect, useRef } from 'react';
import { Search } from 'lucide-react';
import { Switch } from '../../ui/switch';
import { goose_models } from './hardcoded_stuff';
import { useModel } from './ModelContext';
import { useHandleModelSelection } from './utils';
import { useActiveKeys } from '../api_keys/ActiveKeysContext';

// TODO: dark mode (p1)
// FIXME: arrow keys do not work to select a model (p1)
export function SearchBar() {
  const [search, setSearch] = useState('');
  const [focusedIndex, setFocusedIndex] = useState(-1);
  const [showResults, setShowResults] = useState(false);
  const resultsRef = useRef<(HTMLDivElement | null)[]>([]);
  const searchBarRef = useRef<HTMLDivElement>(null);

  const { currentModel } = useModel(); // Access global state
  const handleModelSelection = useHandleModelSelection();

  // search results filtering
  // results set will only include models that have a configured provider
  const { activeKeys } = useActiveKeys(); // Access active keys from context

  const model_options = goose_models.filter((model) => activeKeys.includes(model.provider));

  const filteredModels = model_options
    .filter((model) => model.name.toLowerCase().includes(search.toLowerCase()))
    .slice(0, 5);

  useEffect(() => {
    setFocusedIndex(-1);
  }, [search]);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (searchBarRef.current && !searchBarRef.current.contains(event.target as Node)) {
        setShowResults(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, []);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setFocusedIndex((prev) => (prev < filteredModels.length - 1 ? prev + 1 : prev));
      setShowResults(true);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setFocusedIndex((prev) => (prev > 0 ? prev - 1 : prev));
      setShowResults(true);
    } else if (e.key === 'Enter' && focusedIndex >= 0) {
      e.preventDefault();
      const selectedModel = filteredModels[focusedIndex];
      handleModelSelection(selectedModel, 'SearchBar');
    } else if (e.key === 'Escape') {
      e.preventDefault();
      setShowResults(false);
    }
  };

  useEffect(() => {
    if (focusedIndex >= 0 && focusedIndex < resultsRef.current.length) {
      resultsRef.current[focusedIndex]?.scrollIntoView({
        block: 'nearest',
      });
    }
  }, [focusedIndex]);

  return (
    <div className="relative" ref={searchBarRef}>
      <Search className="absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" />
      <input
        type="text"
        placeholder="Search models..."
        value={search}
        onChange={(e) => {
          setSearch(e.target.value);
          setShowResults(true);
        }}
        onKeyDown={handleKeyDown}
        onFocus={() => setShowResults(true)}
        className="w-full pl-12 py-2 bg-background border border-muted-foreground/20 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
      />
      {showResults && search && (
        <div className="absolute z-10 w-full mt-2 bg-white dark:bg-gray-800 border border-muted-foreground/20 rounded-md shadow-lg">
          {filteredModels.length > 0 ? (
            filteredModels.map((model, index) => (
              <div
                key={model.id}
                ref={(el) => (resultsRef.current[index] = el)}
                className={`p-2 flex justify-between items-center hover:bg-muted/50 dark:hover:bg-gray-700 cursor-pointer ${
                  model.id === currentModel?.id ? 'bg-muted/50 dark:bg-gray-700' : ''
                }`}
              >
                <div>
                  <span className="font-medium dark:text-white">{model.name}</span>
                  <span className="ml-2 text-xs text-gray-500 dark:text-gray-400 italic">
                    {model.provider}
                  </span>
                </div>
                <Switch
                  variant="mono"
                  checked={model.id === currentModel?.id}
                  onCheckedChange={() => handleModelSelection(model, 'SearchBar')}
                />
              </div>
            ))
          ) : (
            <div className="p-2 text-muted-foreground dark:text-gray-400">No models found</div>
          )}
        </div>
      )}
    </div>
  );
}
