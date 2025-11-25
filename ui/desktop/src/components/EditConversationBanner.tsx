import { Save } from 'lucide-react';
import { Button } from './ui/button';

interface EditConversationBannerProps {
  onSave: () => void;
}

export function EditConversationBanner({ onSave }: EditConversationBannerProps) {
  const handleSaveClick = () => {
    console.log('EditConversationBanner: Save button clicked');
    onSave();
  };

  return (
    <div className="w-full bg-background-accent text-text-on-accent py-3 px-6 flex items-center justify-between border-b border-border-default">
      <div className="flex items-center gap-2">
        <span className="text-sm font-medium">Manage Conversation Context</span>
      </div>
      <Button
        onClick={handleSaveClick}
        variant="ghost"
        size="sm"
        className="flex items-center gap-2 text-text-on-accent hover:text-text-on-accent hover:bg-background-accent/80"
      >
        <Save size={14} />
        <span>Done</span>
      </Button>
    </div>
  );
}

