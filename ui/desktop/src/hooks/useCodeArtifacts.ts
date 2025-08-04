import { useState, useEffect } from 'react';

export interface CodeArtifactData {
  id: string;
  code: string;
  language: string;
  title: string;
  description?: string;
  createdAt: Date;
  updatedAt: Date;
}

const STORAGE_KEY = 'Goose-code-artifacts';

export const useCodeArtifacts = () => {
  const [artifacts, setArtifacts] = useState<CodeArtifactData[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  // Load artifacts from localStorage on mount
  useEffect(() => {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
        const parsed = JSON.parse(stored);
        // Convert date strings back to Date objects
        const artifactsWithDates = parsed.map((artifact: unknown) => {
          const typedArtifact = artifact as {
            createdAt: string;
            updatedAt: string;
            [key: string]: unknown;
          };
          return {
            ...typedArtifact,
            createdAt: new Date(typedArtifact.createdAt),
            updatedAt: new Date(typedArtifact.updatedAt),
          };
        });
        setArtifacts(artifactsWithDates);
      }
    } catch (error) {
      console.error('Failed to load code artifacts:', error);
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Save artifacts to localStorage whenever they change
  useEffect(() => {
    if (!isLoading) {
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(artifacts));
      } catch (error) {
        console.error('Failed to save code artifacts:', error);
      }
    }
  }, [artifacts, isLoading]);

  const addArtifact = (artifact: Omit<CodeArtifactData, 'id' | 'createdAt' | 'updatedAt'>) => {
    const newArtifact: CodeArtifactData = {
      ...artifact,
      id: generateId(),
      createdAt: new Date(),
      updatedAt: new Date(),
    };
    setArtifacts((prev) => [newArtifact, ...prev]);
    return newArtifact;
  };

  const updateArtifact = (
    id: string,
    updates: Partial<Omit<CodeArtifactData, 'id' | 'createdAt'>>
  ) => {
    setArtifacts((prev) =>
      prev.map((artifact) =>
        artifact.id === id ? { ...artifact, ...updates, updatedAt: new Date() } : artifact
      )
    );
  };

  const deleteArtifact = (id: string) => {
    setArtifacts((prev) => prev.filter((artifact) => artifact.id !== id));
  };

  const getArtifact = (id: string) => {
    return artifacts.find((artifact) => artifact.id === id);
  };

  const searchArtifacts = (query: string) => {
    const lowerQuery = query.toLowerCase();
    return artifacts.filter(
      (artifact) =>
        artifact.title.toLowerCase().includes(lowerQuery) ||
        artifact.description?.toLowerCase().includes(lowerQuery) ||
        artifact.code.toLowerCase().includes(lowerQuery) ||
        artifact.language.toLowerCase().includes(lowerQuery)
    );
  };

  const filterByLanguage = (language: string) => {
    if (language === 'all') return artifacts;
    return artifacts.filter((artifact) => artifact.language === language);
  };

  const sortArtifacts = (
    artifacts: CodeArtifactData[],
    sortBy: 'date' | 'name' | 'language',
    sortOrder: 'asc' | 'desc'
  ) => {
    return [...artifacts].sort((a, b) => {
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
  };

  const getLanguages = () => {
    return Array.from(new Set(artifacts.map((a) => a.language))).sort();
  };

  const clearAllArtifacts = () => {
    setArtifacts([]);
  };

  const exportArtifacts = () => {
    const dataStr = JSON.stringify(artifacts, null, 2);
    const dataBlob = new Blob([dataStr], { type: 'application/json' });
    const url = URL.createObjectURL(dataBlob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `goose-code-artifacts-${new Date().toISOString().split('T')[0]}.json`;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  };

  const importArtifacts = (file: File): Promise<void> => {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = (e) => {
        try {
          const content = e.target?.result as string;
          const imported = JSON.parse(content);

          // Validate imported data
          if (!Array.isArray(imported)) {
            throw new Error('Invalid file format: expected array of artifacts');
          }

          const validatedArtifacts = imported.map((artifact: unknown) => {
            const typedArtifact = artifact as {
              id?: string;
              code?: string;
              language?: string;
              title?: string;
              description?: string;
              createdAt?: string | number;
              updatedAt?: string | number;
              [key: string]: unknown;
            };
            return {
              id: typedArtifact.id || generateId(),
              code: typedArtifact.code || '',
              language: typedArtifact.language || 'text',
              title: typedArtifact.title || 'Imported Artifact',
              description: typedArtifact.description,
              createdAt: new Date(typedArtifact.createdAt || Date.now()),
              updatedAt: new Date(typedArtifact.updatedAt || Date.now()),
            };
          });

          setArtifacts((prev) => [...validatedArtifacts, ...prev]);
          resolve();
        } catch (error) {
          reject(error);
        }
      };
      reader.onerror = () => reject(new Error('Failed to read file'));
      reader.readAsText(file);
    });
  };

  return {
    artifacts,
    isLoading,
    addArtifact,
    updateArtifact,
    deleteArtifact,
    getArtifact,
    searchArtifacts,
    filterByLanguage,
    sortArtifacts,
    getLanguages,
    clearAllArtifacts,
    exportArtifacts,
    importArtifacts,
  };
};

// Helper function to generate unique IDs
const generateId = () => {
  return Date.now().toString(36) + Math.random().toString(36).substr(2);
};
