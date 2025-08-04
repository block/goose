import React, { useState } from 'react';
import { Plus, Search, Trash2, FolderOpen } from 'lucide-react';
import { Button } from './ui/button';
import { Input } from './ui/input';
import { Select } from './ui/Select';

interface CodeArtifactData {
  id: string;
  code: string;
  language: string;
  title: string;
  description?: string;
  createdAt: Date;
  updatedAt: Date;
}

interface CodeArtifactManagerProps {
  artifacts?: CodeArtifactData[];
  onSaveArtifact?: (artifact: CodeArtifactData) => void;
  onDeleteArtifact?: (id: string) => void;
  onOpenArtifact?: (artifact: CodeArtifactData) => void;
}

export const CodeArtifactManager: React.FC<CodeArtifactManagerProps> = ({
  artifacts = [],
  onDeleteArtifact,
  onOpenArtifact,
}) => {
  const [searchTerm, setSearchTerm] = useState('');
  const [languageFilter, setLanguageFilter] = useState<string>('all');
  const [sortBy, setSortBy] = useState<'date' | 'name' | 'language'>('date');
  const [sortOrder, setSortOrder] = useState<'asc' | 'desc'>('desc');

  // Filter and sort artifacts
  const filteredArtifacts = artifacts
    .filter((artifact) => {
      const matchesSearch =
        artifact.title.toLowerCase().includes(searchTerm.toLowerCase()) ||
        artifact.description?.toLowerCase().includes(searchTerm.toLowerCase()) ||
        artifact.code.toLowerCase().includes(searchTerm.toLowerCase());
      const matchesLanguage = languageFilter === 'all' || artifact.language === languageFilter;
      return matchesSearch && matchesLanguage;
    })
    .sort((a, b) => {
      let comparison = 0;

      switch (sortBy) {
        case 'date':
          comparison = new Date(a.updatedAt).getTime() - new Date(b.updatedAt).getTime();
          break;
        case 'name':
          comparison = a.title.localeCompare(b.title);
          break;
        case 'language':
          comparison = a.language.localeCompare(b.language);
          break;
      }

      return sortOrder === 'asc' ? comparison : -comparison;
    });

  const languages = Array.from(new Set(artifacts.map((a) => a.language))).sort();

  const formatDate = (date: Date) => {
    return new Intl.DateTimeFormat('de-DE', {
      day: '2-digit',
      month: '2-digit',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    }).format(date);
  };

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="p-4 border-b border-border-default bg-background-muted">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-text-default">Code Artifacts</h2>
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() =>
                onOpenArtifact?.({
                  id: 'new',
                  code: '',
                  language: 'html',
                  title: 'New Code Artifact',
                  createdAt: new Date(),
                  updatedAt: new Date(),
                })
              }
            >
              <Plus className="h-4 w-4 mr-2" />
              New Artifact
            </Button>
          </div>
        </div>

        {/* Search and Filters */}
        <div className="flex items-center gap-4">
          <div className="flex-1 relative">
            <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-text-muted" />
            <Input
              placeholder="Search artifacts..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="pl-10"
            />
          </div>

          <div className="w-32">
            <Select
              value={{
                value: languageFilter,
                label: languageFilter === 'all' ? 'All Languages' : languageFilter.toUpperCase(),
              }}
              onChange={(option: unknown) => {
                const selectedOption = option as { value: string; label: string } | null;
                setLanguageFilter(selectedOption?.value || 'all');
              }}
              options={[
                { value: 'all', label: 'All Languages' },
                ...languages.map((lang) => ({ value: lang, label: lang.toUpperCase() })),
              ]}
            />
          </div>

          <div className="w-32">
            <Select
              value={{ value: sortBy, label: sortBy.charAt(0).toUpperCase() + sortBy.slice(1) }}
              onChange={(option: unknown) => {
                const selectedOption = option as { value: string; label: string } | null;
                setSortBy((selectedOption?.value as 'date' | 'name' | 'language') || 'date');
              }}
              options={[
                { value: 'date', label: 'Date' },
                { value: 'name', label: 'Name' },
                { value: 'language', label: 'Language' },
              ]}
            />
          </div>

          <Button
            variant="ghost"
            size="sm"
            onClick={() => setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc')}
            className="w-8 h-8 p-0"
          >
            {sortOrder === 'asc' ? '↑' : '↓'}
          </Button>
        </div>
      </div>

      {/* Artifacts List */}
      <div className="flex-1 overflow-y-auto p-4">
        {filteredArtifacts.length === 0 ? (
          <div className="text-center py-8">
            <FolderOpen className="h-12 w-12 text-text-muted mx-auto mb-4" />
            <h3 className="text-lg font-medium text-text-default mb-2">
              {artifacts.length === 0 ? 'No Code Artifacts Yet' : 'No Artifacts Found'}
            </h3>
            <p className="text-text-muted">
              {artifacts.length === 0
                ? 'Generated code will appear here as artifacts that you can preview, edit, and download.'
                : 'Try adjusting your search or filters.'}
            </p>
          </div>
        ) : (
          <div className="space-y-4">
            {filteredArtifacts.map((artifact) => (
              <div
                key={artifact.id}
                className="border border-border-default rounded-lg bg-background-default p-4 hover:border-border-prominent transition-colors"
              >
                <div className="flex items-start justify-between mb-3">
                  <div className="flex-1">
                    <h3 className="font-medium text-text-default mb-1">{artifact.title}</h3>
                    {artifact.description && (
                      <p className="text-sm text-text-muted mb-2">{artifact.description}</p>
                    )}
                    <div className="flex items-center gap-4 text-xs text-text-muted">
                      <span className="px-2 py-1 bg-background-muted rounded">
                        {artifact.language.toUpperCase()}
                      </span>
                      <span>Updated: {formatDate(artifact.updatedAt)}</span>
                      <span>Created: {formatDate(artifact.createdAt)}</span>
                    </div>
                  </div>

                  <div className="flex items-center gap-2">
                    <Button variant="ghost" size="sm" onClick={() => onOpenArtifact?.(artifact)}>
                      <FolderOpen className="h-4 w-4" />
                    </Button>
                    {onDeleteArtifact && (
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => onDeleteArtifact(artifact.id)}
                        className="text-red-500 hover:text-red-600"
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    )}
                  </div>
                </div>

                {/* Code Preview */}
                <div className="bg-background-muted border border-border-default rounded p-3">
                  <pre className="text-sm font-mono text-text-muted overflow-hidden">
                    <code className="line-clamp-3">
                      {artifact.code.length > 200
                        ? `${artifact.code.substring(0, 200)}...`
                        : artifact.code}
                    </code>
                  </pre>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="p-4 border-t border-border-default bg-background-muted">
        <div className="flex items-center justify-between text-sm text-text-muted">
          <span>
            {filteredArtifacts.length} of {artifacts.length} artifacts
          </span>
          <span>Total: {artifacts.length}</span>
        </div>
      </div>
    </div>
  );
};

export default CodeArtifactManager;
