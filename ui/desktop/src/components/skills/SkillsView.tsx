import { useState, useEffect, useMemo } from 'react';
import { listAvailableSkills } from '../../skills/skills_management';
import {
  Sparkles,
  FolderOpen,
  Globe,
  Cpu,
  AlertCircle,
  ExternalLink,
} from 'lucide-react';
import { ScrollArea } from '../ui/scroll-area';
import { Card } from '../ui/card';
import { Button } from '../ui/button';
import { Skeleton } from '../ui/skeleton';
import { MainPanelLayout } from '../Layout/MainPanelLayout';
import { SearchView } from '../conversation/SearchView';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '../ui/dialog';
import { getSearchShortcutText } from '../../utils/keyboardShortcuts';
import { errorMessage } from '../../utils/conversionUtils';
import MarkdownContent from '../MarkdownContent';
import type { SkillInfo, SkillScope } from '../../api';

const SCOPE_CONFIG: Record<SkillScope, { label: string; icon: typeof FolderOpen }> = {
  Project: { label: 'Project', icon: FolderOpen },
  Global: { label: 'Global', icon: Globe },
  Builtin: { label: 'Built-in', icon: Cpu },
};

const SCOPE_ORDER: SkillScope[] = ['Project', 'Global', 'Builtin'];

export default function SkillsView() {
  const [skills, setSkills] = useState<SkillInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [showSkeleton, setShowSkeleton] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showContent, setShowContent] = useState(false);
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedSkill, setSelectedSkill] = useState<SkillInfo | null>(null);

  const filteredSkills = useMemo(() => {
    if (!searchTerm) return skills;
    const searchLower = searchTerm.toLowerCase();
    return skills.filter((skill) => {
      return (
        skill.name.toLowerCase().includes(searchLower) ||
        skill.description.toLowerCase().includes(searchLower)
      );
    });
  }, [skills, searchTerm]);

  const groupedSkills = useMemo(() => {
    const groups: Partial<Record<SkillScope, SkillInfo[]>> = {};
    for (const skill of filteredSkills) {
      if (!groups[skill.scope]) {
        groups[skill.scope] = [];
      }
      groups[skill.scope]!.push(skill);
    }
    return groups;
  }, [filteredSkills]);

  useEffect(() => {
    loadSkills();
  }, []);

  useEffect(() => {
    if (!loading && showSkeleton) {
      const timer = setTimeout(() => {
        setShowSkeleton(false);
        setTimeout(() => {
          setShowContent(true);
        }, 50);
      }, 300);
      return () => clearTimeout(timer);
    }
    return () => void 0;
  }, [loading, showSkeleton]);

  const loadSkills = async () => {
    try {
      setLoading(true);
      setShowSkeleton(true);
      setShowContent(false);
      setError(null);
      const result = await listAvailableSkills();
      setSkills(result);
    } catch (err) {
      setError(errorMessage(err, 'Failed to load skills'));
      console.error('Failed to load skills:', err);
    } finally {
      setLoading(false);
    }
  };

  const handleBrowseSkills = () => {
    window.electron.openExternal(
      'https://block.github.io/goose/docs/getting-started/using-skills'
    );
  };

  const SkillTile = ({ skill }: { skill: SkillInfo }) => {
    const config = SCOPE_CONFIG[skill.scope];
    const ScopeIcon = config.icon;

    return (
      <Card
        className="min-h-[120px] py-4 px-4 bg-background-primary hover:bg-background-secondary transition-all duration-150 cursor-pointer flex flex-col"
        onClick={() => setSelectedSkill(skill)}
      >
        <div className="flex items-center gap-2 mb-2">
          <Sparkles className="w-4 h-4 text-purple-500 shrink-0" />
          <h3 className="text-sm font-medium truncate">{skill.name}</h3>
        </div>
        <p className="text-text-secondary text-xs line-clamp-3 flex-1">{skill.description}</p>
        <div className="mt-2 pt-2 border-t border-border-primary">
          <span className="flex items-center gap-1 text-xs text-text-secondary">
            <ScopeIcon className="w-3 h-3" />
            {config.label}
          </span>
        </div>
      </Card>
    );
  };

  const SkillSkeleton = () => (
    <Card className="min-h-[120px] py-4 px-4 bg-background-primary flex flex-col">
      <div className="flex items-center gap-2 mb-2">
        <Skeleton className="h-4 w-4 rounded" />
        <Skeleton className="h-4 w-3/4" />
      </div>
      <Skeleton className="h-3 w-full mb-1" />
      <Skeleton className="h-3 w-2/3" />
      <div className="mt-auto pt-2">
        <Skeleton className="h-3 w-16" />
      </div>
    </Card>
  );

  const renderContent = () => {
    if (loading || showSkeleton) {
      return (
        <div className="space-y-6">
          <div>
            <Skeleton className="h-6 w-24 mb-3" />
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-2">
              <SkillSkeleton />
              <SkillSkeleton />
              <SkillSkeleton />
              <SkillSkeleton />
            </div>
          </div>
        </div>
      );
    }

    if (error) {
      return (
        <div className="flex flex-col items-center justify-center h-full text-text-secondary">
          <AlertCircle className="h-12 w-12 text-red-500 mb-4" />
          <p className="text-lg mb-2">Error Loading Skills</p>
          <p className="text-sm text-center mb-4">{error}</p>
          <Button onClick={loadSkills} variant="default">
            Try Again
          </Button>
        </div>
      );
    }

    if (skills.length === 0) {
      return (
        <div className="flex flex-col justify-center pt-2 h-full">
          <p className="text-lg">No skills found</p>
          <p className="text-sm text-text-secondary">
            Skills are SKILL.md files that teach Goose how to perform tasks. Add them to your project
            or global config directory.
          </p>
        </div>
      );
    }

    if (filteredSkills.length === 0 && searchTerm) {
      return (
        <div className="flex flex-col items-center justify-center h-full text-text-secondary mt-4">
          <Sparkles className="h-12 w-12 mb-4" />
          <p className="text-lg mb-2">No matching skills found</p>
          <p className="text-sm">Try adjusting your search terms</p>
        </div>
      );
    }

    return (
      <div className="space-y-6">
        {SCOPE_ORDER.map((scope) => {
          const scopeSkills = groupedSkills[scope];
          if (!scopeSkills || scopeSkills.length === 0) return null;
          const config = SCOPE_CONFIG[scope];
          const ScopeIcon = config.icon;

          return (
            <div key={scope}>
              <div className="flex items-center gap-2 mb-3">
                <ScopeIcon className="w-4 h-4 text-text-secondary" />
                <h2 className="text-lg font-medium">{config.label}</h2>
                <span className="text-sm text-text-secondary">({scopeSkills.length})</span>
              </div>
              <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-2">
                {scopeSkills.map((skill) => (
                  <SkillTile key={skill.path} skill={skill} />
                ))}
              </div>
            </div>
          );
        })}
      </div>
    );
  };

  const selectedScopeConfig = selectedSkill ? SCOPE_CONFIG[selectedSkill.scope] : null;
  const SelectedScopeIcon = selectedScopeConfig?.icon;

  return (
    <>
      <MainPanelLayout>
        <div className="flex-1 flex flex-col min-h-0">
          <div className="bg-background-primary px-8 pb-8 pt-16">
            <div className="flex flex-col page-transition">
              <div className="flex justify-between items-center mb-1">
                <h1 className="text-4xl font-light">Skills</h1>
                <Button
                  onClick={handleBrowseSkills}
                  variant="outline"
                  size="sm"
                  className="flex items-center gap-2"
                >
                  <ExternalLink className="w-4 h-4" />
                  Browse Skills
                </Button>
              </div>
              <p className="text-sm text-text-secondary mb-1">
                Skills are reusable instruction sets that teach Goose how to perform tasks.{' '}
                {getSearchShortcutText()} to search.
              </p>
            </div>
          </div>

          <div className="flex-1 min-h-0 relative px-8">
            <ScrollArea className="h-full">
              <SearchView onSearch={(term) => setSearchTerm(term)} placeholder="Search skills...">
                <div
                  className={`h-full relative transition-all duration-300 ${
                    showContent ? 'opacity-100 animate-in fade-in ' : 'opacity-0'
                  }`}
                >
                  {renderContent()}
                </div>
              </SearchView>
            </ScrollArea>
          </div>
        </div>
      </MainPanelLayout>

      {selectedSkill && (
        <Dialog open={true} onOpenChange={() => setSelectedSkill(null)}>
          <DialogContent className="sm:max-w-[700px] max-h-[80vh] !flex !flex-col overflow-hidden">
            <DialogHeader>
              <DialogTitle className="flex items-center gap-2">
                <Sparkles className="w-5 h-5 text-purple-500" />
                {selectedSkill.name}
              </DialogTitle>
            </DialogHeader>
            <div className="flex items-center gap-2 text-sm text-text-secondary shrink-0">
              {SelectedScopeIcon && <SelectedScopeIcon className="w-3.5 h-3.5" />}
              <span>{selectedScopeConfig?.label}</span>
              <span className="text-text-secondary/50">·</span>
              <span className="truncate">{selectedSkill.description}</span>
            </div>
            <div className="flex-1 min-h-0 mt-2 overflow-y-auto">
              <div className="text-sm pr-2">
                <MarkdownContent content={selectedSkill.content} />
              </div>
            </div>
          </DialogContent>
        </Dialog>
      )}
    </>
  );
}
