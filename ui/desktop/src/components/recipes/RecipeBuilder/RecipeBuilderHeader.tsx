import { MessageSquare, Pencil, Play, Save, X } from 'lucide-react';
import { Button } from '../../ui/button';
import { RecipeBuilderHeaderProps } from './types';

export default function RecipeBuilderHeader({
  currentView,
  onViewChange,
  testPanelOpen,
  onTestPanelToggle,
  canTest,
  canSave,
  isSaving,
  onSave,
  onClose,
}: RecipeBuilderHeaderProps) {
  return (
    <div className="flex items-center justify-between px-4 py-3 border-b border-borderSubtle bg-background-default">
      <div className="flex items-center gap-1">
        <Button
          variant={currentView === 'chat' ? 'default' : 'ghost'}
          size="sm"
          onClick={() => onViewChange('chat')}
          className="flex items-center gap-2"
        >
          <MessageSquare className="w-4 h-4" />
          Chat
        </Button>
        <Button
          variant={currentView === 'edit' ? 'default' : 'ghost'}
          size="sm"
          onClick={() => onViewChange('edit')}
          className="flex items-center gap-2"
        >
          <Pencil className="w-4 h-4" />
          Edit
        </Button>
      </div>

      <div className="flex items-center gap-2">
        <Button
          variant={testPanelOpen ? 'default' : 'outline'}
          size="sm"
          onClick={onTestPanelToggle}
          disabled={!canTest}
          className="flex items-center gap-2"
        >
          <Play className="w-4 h-4" />
          Test
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={onSave}
          disabled={!canSave || isSaving}
          className="flex items-center gap-2"
        >
          <Save className="w-4 h-4" />
          {isSaving ? 'Saving...' : 'Save'}
        </Button>
        <Button variant="ghost" size="sm" onClick={onClose} className="h-8 w-8 p-0">
          <X className="w-5 h-5" />
        </Button>
      </div>
    </div>
  );
}
