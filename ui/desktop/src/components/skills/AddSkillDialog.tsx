import { useState } from 'react';
import type { SourceEntry } from '@aaif/goose-sdk';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/dialog';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Switch } from '../ui/switch';
import { defineMessages, useIntl } from '../../i18n';
import { createSkillSource, validateSkillName } from '../../acp/sources';
import { getInitialWorkingDir } from '../../utils/workingDir';
import { errorMessage } from '../../utils/conversionUtils';

const i18n = defineMessages({
  title: {
    id: 'addSkillDialog.title',
    defaultMessage: 'Add Skill',
  },
  description: {
    id: 'addSkillDialog.description',
    defaultMessage:
      'Creates a SKILL.md under .agents/skills/ (project) or ~/.agents/skills/ (global).',
  },
  nameLabel: {
    id: 'addSkillDialog.nameLabel',
    defaultMessage: 'Name',
  },
  namePlaceholder: {
    id: 'addSkillDialog.namePlaceholder',
    defaultMessage: 'my-skill-name',
  },
  descriptionLabel: {
    id: 'addSkillDialog.descriptionLabel',
    defaultMessage: 'Description',
  },
  descriptionPlaceholder: {
    id: 'addSkillDialog.descriptionPlaceholder',
    defaultMessage: 'What it does and when to use it',
  },
  contentLabel: {
    id: 'addSkillDialog.contentLabel',
    defaultMessage: 'Instructions',
  },
  contentPlaceholder: {
    id: 'addSkillDialog.contentPlaceholder',
    defaultMessage: '# My Skill\n\nStep-by-step guidance for the agent…',
  },
  globalLabel: {
    id: 'addSkillDialog.globalLabel',
    defaultMessage: 'Global skill (available in all projects)',
  },
  cancel: {
    id: 'addSkillDialog.cancel',
    defaultMessage: 'Cancel',
  },
  create: {
    id: 'addSkillDialog.create',
    defaultMessage: 'Create skill',
  },
  creating: {
    id: 'addSkillDialog.creating',
    defaultMessage: 'Creating…',
  },
});

type AddSkillDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreated: (source: SourceEntry) => void;
};

export function AddSkillDialog({ open, onOpenChange, onCreated }: AddSkillDialogProps) {
  const intl = useIntl();
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [content, setContent] = useState('');
  const [global, setGlobal] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const reset = () => {
    setName('');
    setDescription('');
    setContent('');
    setGlobal(false);
    setError(null);
    setSubmitting(false);
  };

  const handleOpenChange = (next: boolean) => {
    if (!next) {
      reset();
    }
    onOpenChange(next);
  };

  const handleSubmit = async () => {
    const nameError = validateSkillName(name);
    if (nameError) {
      setError(nameError);
      return;
    }
    if (!description.trim()) {
      setError('Description is required');
      return;
    }
    if (!content.trim()) {
      setError('Instructions are required');
      return;
    }

    setSubmitting(true);
    setError(null);
    try {
      const source = await createSkillSource({
        name,
        description,
        content,
        projectDir: getInitialWorkingDir(),
        global,
      });
      onCreated(source);
      handleOpenChange(false);
    } catch (err) {
      setError(errorMessage(err, 'Failed to create skill'));
      setSubmitting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-lg" data-testid="add-skill-dialog">
        <DialogHeader>
          <DialogTitle>{intl.formatMessage(i18n.title)}</DialogTitle>
          <DialogDescription>{intl.formatMessage(i18n.description)}</DialogDescription>
        </DialogHeader>

        <div className="space-y-3">
          <label className="block space-y-1">
            <span className="text-sm text-text-secondary">{intl.formatMessage(i18n.nameLabel)}</span>
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={intl.formatMessage(i18n.namePlaceholder)}
              autoFocus
              data-testid="add-skill-name"
            />
          </label>

          <label className="block space-y-1">
            <span className="text-sm text-text-secondary">
              {intl.formatMessage(i18n.descriptionLabel)}
            </span>
            <Input
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder={intl.formatMessage(i18n.descriptionPlaceholder)}
              data-testid="add-skill-description"
            />
          </label>

          <label className="block space-y-1">
            <span className="text-sm text-text-secondary">
              {intl.formatMessage(i18n.contentLabel)}
            </span>
            <textarea
              value={content}
              onChange={(e) => setContent(e.target.value)}
              placeholder={intl.formatMessage(i18n.contentPlaceholder)}
              rows={8}
              className="flex w-full rounded-md border focus:border-border-secondary hover:border-border-secondary bg-background-primary px-3 py-2 text-sm transition-colors placeholder:text-text-secondary placeholder:font-light focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50 font-mono"
              data-testid="add-skill-content"
            />
          </label>

          <label className="flex items-center justify-between gap-3 py-1">
            <span className="text-sm text-text-secondary">
              {intl.formatMessage(i18n.globalLabel)}
            </span>
            <Switch
              checked={global}
              onCheckedChange={setGlobal}
              data-testid="add-skill-global"
            />
          </label>

          {error ? (
            <p className="text-sm text-red-500" data-testid="add-skill-error">
              {error}
            </p>
          ) : null}
        </div>

        <DialogFooter className="pt-2">
          <Button variant="outline" onClick={() => handleOpenChange(false)} disabled={submitting}>
            {intl.formatMessage(i18n.cancel)}
          </Button>
          <Button onClick={handleSubmit} disabled={submitting} data-testid="add-skill-submit">
            {submitting
              ? intl.formatMessage(i18n.creating)
              : intl.formatMessage(i18n.create)}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
