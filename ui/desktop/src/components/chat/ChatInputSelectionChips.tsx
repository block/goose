import { Sparkles } from 'lucide-react';
import type { ChatSkillDraft } from '../skills/lib/skillChatPrompt';
import { ComposerChip } from './ComposerChip';

interface ChatInputSelectionChipsProps {
  skills: ChatSkillDraft[];
  onRemoveSkill: (skillId: string) => void;
}

export function ChatInputSelectionChips({ skills, onRemoveSkill }: ChatInputSelectionChipsProps) {
  if (skills.length === 0) {
    return null;
  }

  return (
    <div className="flex flex-wrap gap-1.5 px-1 pb-2">
      {skills.map((skill) => (
        <ComposerChip
          key={skill.id}
          tone="skill"
          label={skill.name}
          title={skill.description ?? skill.name}
          removeLabel={`Remove ${skill.name} skill`}
          onRemove={() => onRemoveSkill(skill.id)}
          leading={<Sparkles className="size-3" />}
        />
      ))}
    </div>
  );
}
