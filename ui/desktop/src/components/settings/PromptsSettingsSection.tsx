import { useState, useEffect } from 'react';
import { getPrompt, getPrompts, resetPrompt, savePrompt } from '../../api';
import type { PromptContentResponse, Template } from '../../api';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/card';
import { Button } from '../ui/button';
import { AlertTriangle, RotateCcw, ArrowLeft } from 'lucide-react';
import { toast } from 'react-toastify';

export default function PromptsSettingsSection() {
  const [prompts, setPrompts] = useState<Template[]>([]);
  const [selectedPrompt, setSelectedPrompt] = useState<string | null>(null);
  const [promptData, setPromptData] = useState<PromptContentResponse | null>(null);
  const [content, setContent] = useState('');
  const [hasChanges, setHasChanges] = useState(false);

  const fetchPrompts = async () => {
    try {
      const response = await getPrompts();
      if (response.data) {
        setPrompts(response.data.prompts);
      }
    } catch (error) {
      console.error('Failed to fetch prompts:', error);
      toast.error('Failed to load prompts');
    }
  };

  useEffect(() => {
    fetchPrompts();
  }, []);

  useEffect(() => {
    if (selectedPrompt) {
      const fetchPrompt = async () => {
        try {
          const response = await getPrompt({ path: { name: selectedPrompt } });
          if (response.data) {
            setPromptData(response.data);
            setContent(response.data.content);
          }
        } catch (error) {
          console.error('Failed to fetch prompt:', error);
          toast.error('Failed to load prompt');
        }
      };
      fetchPrompt();
    }
  }, [selectedPrompt]);

  useEffect(() => {
    if (promptData) {
      setHasChanges(content !== promptData.content);
    }
  }, [content, promptData]);

  const handleResetAll = async () => {
    if (
      !window.confirm(
        'Are you sure you want to reset all prompts to their defaults? This cannot be undone.'
      )
    ) {
      return;
    }

    try {
      const customizedPrompts = prompts.filter((p) => p.is_customized);
      for (const prompt of customizedPrompts) {
        await resetPrompt({ path: { name: prompt.name } });
      }
      toast.success('All prompts reset to defaults');
      fetchPrompts();
    } catch (error) {
      console.error('Failed to reset all prompts:', error);
      toast.error('Failed to reset prompts');
    }
  };

  const handleSave = async () => {
    if (!selectedPrompt) return;
    try {
      await savePrompt({
        path: { name: selectedPrompt },
        body: { content },
      });
      toast.success('Prompt saved');
      setPromptData((prev) => (prev ? { ...prev, content, is_customized: true } : null));
      fetchPrompts();
    } catch (error) {
      console.error('Failed to save prompt:', error);
      toast.error('Failed to save prompt');
    }
  };

  const handleReset = async () => {
    if (!selectedPrompt) return;
    if (
      !window.confirm(
        'Are you sure you want to reset this prompt to its default? This cannot be undone.'
      )
    ) {
      return;
    }

    try {
      await resetPrompt({ path: { name: selectedPrompt } });
      if (promptData) {
        setContent(promptData.default_content);
        setPromptData({ ...promptData, content: promptData.default_content, is_customized: false });
      }
      fetchPrompts();
      toast.success('Prompt reset to default');
    } catch (error) {
      console.error('Failed to reset prompt:', error);
      toast.error('Failed to reset prompt');
    }
  };

  const handleRestoreDefault = () => {
    if (promptData) {
      if (hasChanges) {
        if (!window.confirm('Replace current content with default? Your changes will be lost.')) {
          return;
        }
      }
      setContent(promptData.default_content);
    }
  };

  const handleBack = () => {
    if (hasChanges) {
      if (!window.confirm('You have unsaved changes. Are you sure you want to go back?')) {
        return;
      }
    }
    setSelectedPrompt(null);
    setPromptData(null);
    setContent('');
  };

  const hasCustomizedPrompts = prompts.some((p) => p.is_customized);

  if (selectedPrompt) {
    return (
      <div className="space-y-4">
        <Card className="pb-2 rounded-lg">
          <CardHeader className="pb-4">
            <div className="flex items-center justify-between mb-4">
              <Button
                variant="ghost"
                size="sm"
                onClick={handleBack}
                className="flex items-center gap-2"
              >
                <ArrowLeft className="h-4 w-4" />
                Back to List
              </Button>
              <div className="flex items-center gap-2">
                {promptData?.is_customized && (
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={handleReset}
                    className="flex items-center gap-2"
                  >
                    <RotateCcw className="h-4 w-4" />
                    Reset to Default
                  </Button>
                )}
                <Button onClick={handleSave} disabled={!hasChanges} size="sm">
                  Save
                </Button>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <CardTitle>Edit: {selectedPrompt}</CardTitle>
              {promptData?.is_customized && (
                <span className="px-2 py-0.5 text-xs rounded-full bg-blue-500/20 text-blue-600 dark:text-blue-400">
                  Customized
                </span>
              )}
            </div>
          </CardHeader>
          <CardContent className="px-4 space-y-4 flex flex-col h-full">
            <div className="text-sm text-text-muted bg-background-muted p-3 rounded-lg">
              <p>
                <strong>Tip:</strong> Template variables like{' '}
                <code className="bg-background-default px-1 rounded">{'{{ extensions }}'}</code> or{' '}
                <code className="bg-background-default px-1 rounded">
                  {'{% for item in list %}'}
                </code>{' '}
                are replaced with actual values at runtime. Be careful not to remove required
                variables.
              </p>
            </div>

            <div className="space-y-2 flex-1 flex flex-col min-h-0">
              <div className="flex items-center justify-between">
                <label className="text-sm font-medium">Editing: {selectedPrompt}</label>
                {promptData?.is_customized && content !== promptData.default_content && (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={handleRestoreDefault}
                    className="text-xs"
                  >
                    Restore Default
                  </Button>
                )}
              </div>
              <textarea
                value={content}
                className="w-full flex-1 min-h-[500px] border rounded-md p-3 text-sm font-mono resize-y bg-background-default text-text-default border-border-default focus:outline-none focus:ring-2 focus:ring-blue-500"
                onChange={(e) => setContent(e.target.value)}
                placeholder="Enter prompt content..."
                spellCheck={false}
              />
            </div>

            {hasChanges && (
              <div className="text-sm text-yellow-600 dark:text-yellow-400">
                You have unsaved changes
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <Card className="pb-2 rounded-lg border-yellow-500/50 bg-yellow-500/10">
        <CardHeader className="pb-2">
          <div className="flex items-start gap-3">
            <AlertTriangle className="h-5 w-5 text-yellow-500 flex-shrink-0 mt-1" />
            <div className="flex-1">
              <CardTitle className="text-yellow-600 dark:text-yellow-400">Prompt Editing</CardTitle>
              <p className="text-sm text-text-muted mt-2">
                Customize the prompts that define goose's behavior in different contexts. These
                prompts use Jinja2 templating syntax. Be careful when modifying template variables,
                as incorrect changes can break functionality. Please share any improvements with the
                community.
              </p>
            </div>
            {hasCustomizedPrompts && (
              <Button
                variant="outline"
                size="sm"
                onClick={handleResetAll}
                className="flex items-center gap-2 border-yellow-500/50 hover:bg-yellow-500/20"
              >
                <RotateCcw className="h-4 w-4" />
                Reset All
              </Button>
            )}
          </div>
        </CardHeader>
        <CardContent className="px-4 pt-4">
          <div className="space-y-2">
            {prompts.map((prompt) => (
              <div
                key={prompt.name}
                className="flex items-center justify-between p-3 rounded-lg border border-border-default hover:bg-background-muted transition-colors"
              >
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <h4 className="font-medium text-text-default truncate">{prompt.name}</h4>
                    {prompt.is_customized && (
                      <span className="px-2 py-0.5 text-xs rounded-full bg-blue-500/20 text-blue-600 dark:text-blue-400">
                        Customized
                      </span>
                    )}
                  </div>
                  <p className="text-sm text-text-muted mt-0.5 truncate">{prompt.description}</p>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setSelectedPrompt(prompt.name)}
                  className="ml-4"
                >
                  Edit
                </Button>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
