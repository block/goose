import { useCallback, useEffect, useState } from 'react';

const STORAGE_KEY = 'goose-project-preferences';
const RECENT_DIRS_KEY = 'goose-recent-project-dirs';
const MAX_RECENT_DIRS = 10;

interface ProjectPreferences {
  pinnedProjects: string[];
  collapsedProjects: string[];
}

function loadPreferences(): ProjectPreferences {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return JSON.parse(raw);
  } catch {
    // ignore parse errors
  }
  return { pinnedProjects: [], collapsedProjects: [] };
}

function savePreferences(prefs: ProjectPreferences) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(prefs));
}

function loadRecentDirs(): string[] {
  try {
    const raw = localStorage.getItem(RECENT_DIRS_KEY);
    if (raw) return JSON.parse(raw);
  } catch {
    // ignore
  }
  return [];
}

function saveRecentDirs(dirs: string[]) {
  localStorage.setItem(RECENT_DIRS_KEY, JSON.stringify(dirs));
}

export function useProjectPreferences() {
  const [prefs, setPrefs] = useState<ProjectPreferences>(loadPreferences);
  const [recentDirs, setRecentDirs] = useState<string[]>(loadRecentDirs);

  useEffect(() => {
    savePreferences(prefs);
  }, [prefs]);

  useEffect(() => {
    saveRecentDirs(recentDirs);
  }, [recentDirs]);

  const togglePin = useCallback((project: string) => {
    setPrefs((prev) => {
      const isPinned = prev.pinnedProjects.includes(project);
      return {
        ...prev,
        pinnedProjects: isPinned
          ? prev.pinnedProjects.filter((p) => p !== project)
          : [...prev.pinnedProjects, project],
      };
    });
  }, []);

  const toggleCollapsed = useCallback((project: string) => {
    setPrefs((prev) => {
      const isCollapsed = prev.collapsedProjects.includes(project);
      return {
        ...prev,
        collapsedProjects: isCollapsed
          ? prev.collapsedProjects.filter((p) => p !== project)
          : [...prev.collapsedProjects, project],
      };
    });
  }, []);

  const isCollapsed = useCallback(
    (project: string) => prefs.collapsedProjects.includes(project),
    [prefs.collapsedProjects]
  );

  const isPinned = useCallback(
    (project: string) => prefs.pinnedProjects.includes(project),
    [prefs.pinnedProjects]
  );

  const addRecentDir = useCallback((dir: string) => {
    setRecentDirs((prev) => {
      const filtered = prev.filter((d) => d !== dir);
      return [dir, ...filtered].slice(0, MAX_RECENT_DIRS);
    });
  }, []);

  return {
    pinnedProjects: prefs.pinnedProjects,
    togglePin,
    isPinned,
    toggleCollapsed,
    isCollapsed,
    recentDirs,
    addRecentDir,
  };
}
