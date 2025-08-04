import React, { useState } from 'react';
import { X, Download, Upload, Trash2 } from 'lucide-react';
import { Button } from './ui/button';
import { useCodeArtifacts, type CodeArtifactData } from '../hooks/useCodeArtifacts';
import CodeArtifact from './CodeArtifact';
import CodeArtifactManager from './CodeArtifactManager';

interface CodeArtifactViewProps {
  onClose?: () => void;
}

export const CodeArtifactView: React.FC<CodeArtifactViewProps> = ({ onClose }) => {
  const {
    artifacts,
    isLoading,
    addArtifact,
    updateArtifact,
    deleteArtifact,
    exportArtifacts,
    importArtifacts,
  } = useCodeArtifacts();

  const [selectedArtifact, setSelectedArtifact] = useState<CodeArtifactData | null>(null);
  const [isImporting, setIsImporting] = useState(false);

  const handleOpenArtifact = (artifact: CodeArtifactData) => {
    setSelectedArtifact(artifact);
  };

  const handleSaveArtifact = (code: string, title: string) => {
    if (selectedArtifact) {
      updateArtifact(selectedArtifact.id, { code, title });
    }
  };

  const handleDeleteArtifact = (id: string) => {
    deleteArtifact(id);
    if (selectedArtifact?.id === id) {
      setSelectedArtifact(null);
    }
  };

  const handleImport = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    setIsImporting(true);
    try {
      await importArtifacts(file);
    } catch (error) {
      console.error('Failed to import artifacts:', error);
      // You could show a toast notification here
    } finally {
      setIsImporting(false);
      // Reset the input
      event.target.value = '';
    }
  };

  const handleExport = () => {
    exportArtifacts();
  };

  const handleClearAll = () => {
    if (
      window.confirm(
        'Are you sure you want to delete all code artifacts? This action cannot be undone.'
      )
    ) {
      // This would need to be added to the hook
      // clearAllArtifacts();
      setSelectedArtifact(null);
    }
  };

  if (isLoading) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-center">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-border-default mx-auto mb-4"></div>
          <p className="text-text-muted">Loading code artifacts...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between p-4 h-25 border-b border-border-default bg-background-muted">
        <h1 className="text-xl font-semibold text-text-default">🤖</h1>

        <div className="flex items-center gap-2">
          {/* Import Button */}
          <div className="relative">
            <input
              type="file"
              accept=".json"
              onChange={handleImport}
              className="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
              disabled={isImporting}
            />
            <Button variant="outline" size="sm" disabled={isImporting} className="relative">
              <Upload className="h-4 w-4 mr-2" />
              {isImporting ? 'Importing...' : 'Import'}
            </Button>
          </div>

          {/* Export Button */}
          <Button
            variant="outline"
            size="sm"
            onClick={handleExport}
            disabled={artifacts.length === 0}
          >
            <Download className="h-4 w-4 mr-2" />
            Export
          </Button>

          {/* Clear All Button */}
          <Button
            variant="outline"
            size="sm"
            onClick={handleClearAll}
            disabled={artifacts.length === 0}
            className="text-red-500 hover:text-red-600"
          >
            <Trash2 className="h-4 w-4 mr-2" />
            Clear All
          </Button>

          {/* Close Button */}
          {onClose && (
            <Button variant="ghost" size="sm" onClick={onClose} className="h-8 w-8 p-0">
              <X className="h-4 w-4" />
            </Button>
          )}
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 flex overflow-hidden">
        {selectedArtifact ? (
          // Artifact Detail View
          <div className="flex-1 flex flex-col">
            <div className="flex items-center justify-between p-4 border-b border-border-default bg-background-muted">
              <div>
                <h2 className="text-lg font-medium text-text-default">{selectedArtifact.title}</h2>
                <p className="text-sm text-text-muted">
                  {selectedArtifact.language.toUpperCase()} • Updated{' '}
                  {new Date(selectedArtifact.updatedAt).toLocaleDateString()}
                </p>
              </div>
              <Button variant="ghost" size="sm" onClick={() => setSelectedArtifact(null)}>
                <X className="h-4 w-4" />
              </Button>
            </div>

            <div className="flex-1 overflow-y-auto">
              <CodeArtifact
                code={selectedArtifact.code}
                language={selectedArtifact.language}
                title={selectedArtifact.title}
                description={selectedArtifact.description}
                onSave={handleSaveArtifact}
                onDelete={() => handleDeleteArtifact(selectedArtifact.id)}
              />
            </div>
          </div>
        ) : (
          // Artifacts List View
          <div className="flex-1">
            <CodeArtifactManager
              artifacts={artifacts}
              onSaveArtifact={(artifact) => {
                if (artifact.id === 'new') {
                  addArtifact({
                    code: artifact.code,
                    language: artifact.language,
                    title: artifact.title,
                    description: artifact.description,
                  });
                } else {
                  updateArtifact(artifact.id, {
                    code: artifact.code,
                    language: artifact.language,
                    title: artifact.title,
                    description: artifact.description,
                  });
                }
              }}
              onDeleteArtifact={handleDeleteArtifact}
              onOpenArtifact={handleOpenArtifact}
            />
          </div>
        )}
      </div>
    </div>
  );
};

export default CodeArtifactView;
