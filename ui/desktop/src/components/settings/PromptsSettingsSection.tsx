import { useState, useEffect, useCallback, useRef } from 'react';
import {
  getPrompt,
  getPrompts,
  PromptContentResponse,
  Template,
  resetPrompt,
  savePrompt,
} from '../../api';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/card';
import { Button } from '../ui/button';
import { AlertTriangle, RotateCcw, ArrowLeft } from 'lucide-react';
import { toast } from 'react-toastify';
import { useLocalization } from '../../contexts/LocalizationContext';

export default function PromptsSettingsSection() {
  const { t } = useLocalization();
  const tRef = useRef(t);
  const [prompts, setPrompts] = useState<Template[]>([]);
  const [selectedPrompt, setSelectedPrompt] = useState<string | null>(null);
  const [promptData, setPromptData] = useState<PromptContentResponse | null>(null);
  const [content, setContent] = useState('');
  const [hasChanges, setHasChanges] = useState(false);

  useEffect(() => {
    tRef.current = t;
  }, [t]);

  const fetchPrompts = useCallback(async () => {
    try {
      const response = await getPrompts();
      if (response.data) {
        setPrompts(response.data.prompts);
      }
    } catch (error) {
      console.error('Failed to fetch prompts:', error);
      toast.error(tRef.current('prompts.loadPromptsFailed'));
    }
  }, []);

  useEffect(() => {
    fetchPrompts();
  }, [fetchPrompts]);

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
          toast.error(tRef.current('prompts.loadPromptFailed'));
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
      !window.confirm(t('prompts.resetAllConfirm'))
    ) {
      return;
    }

    try {
      const customizedPrompts = prompts.filter((p) => p.is_customized);
      for (const prompt of customizedPrompts) {
        await resetPrompt({ path: { name: prompt.name } });
      }
      toast.success(t('prompts.resetAllSuccess'));
      fetchPrompts();
    } catch (error) {
      console.error('Failed to reset all prompts:', error);
      toast.error(t('prompts.resetAllFailed'));
    }
  };

  const handleSave = async () => {
    if (!selectedPrompt) return;
    try {
      await savePrompt({
        path: { name: selectedPrompt },
        body: { content },
      });
      toast.success(t('prompts.promptSaved'));
      setPromptData((prev) => (prev ? { ...prev, content, is_customized: true } : null));
      fetchPrompts();
    } catch (error) {
      console.error('Failed to save prompt:', error);
      toast.error(t('prompts.saveFailed'));
    }
  };

  const handleReset = async () => {
    if (!selectedPrompt) return;
    if (!window.confirm(t('prompts.resetConfirm'))) {
      return;
    }

    try {
      await resetPrompt({ path: { name: selectedPrompt } });
      if (promptData) {
        setContent(promptData.default_content);
        setPromptData({ ...promptData, content: promptData.default_content, is_customized: false });
      }
      fetchPrompts();
      toast.success(t('prompts.resetSuccess'));
    } catch (error) {
      console.error('Failed to reset prompt:', error);
      toast.error(t('prompts.resetFailed'));
    }
  };

  const handleRestoreDefault = () => {
    if (promptData) {
      if (hasChanges) {
        if (!window.confirm(t('prompts.restoreConfirm'))) {
          return;
        }
      }
      setContent(promptData.default_content);
    }
  };

  const handleBack = () => {
    if (hasChanges) {
      if (!window.confirm(t('prompts.unsavedBackConfirm'))) {
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
      <div className="space-y-4 pr-4 pb-8 mt-1">
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
                {t('prompts.backToList')}
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
                    {t('prompts.resetToDefault')}
                  </Button>
                )}
                <Button onClick={handleSave} disabled={!hasChanges} size="sm">
                  {t('common.actions.save')}
                </Button>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <CardTitle>{t('prompts.editTitle', { name: selectedPrompt })}</CardTitle>
              {promptData?.is_customized && (
                <span className="px-2 py-0.5 text-xs rounded-full bg-blue-500/20 text-blue-600 dark:text-blue-400">
                  {t('prompts.customized')}
                </span>
              )}
            </div>
          </CardHeader>
          <CardContent className="px-4 space-y-4 flex flex-col h-full">
            <div className="text-sm text-text-secondary bg-background-secondary p-3 rounded-lg">
              <p>
                <strong>{t('prompts.tipTitle')}</strong>{' '}
                {t('prompts.tipBody', {
                  extensionsExample: '{{ extensions }}',
                  loopExample: '{% for item in list %}',
                })}
              </p>
            </div>

            <div className="space-y-2 flex-1 flex flex-col min-h-0">
              <div className="flex items-center justify-between">
                <label className="text-sm font-medium">
                  {t('prompts.editingLabel', { name: selectedPrompt })}
                </label>
                {promptData?.is_customized && content !== promptData.default_content && (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={handleRestoreDefault}
                    className="text-xs"
                  >
                    {t('prompts.restoreDefault')}
                  </Button>
                )}
              </div>
              <textarea
                value={content}
                className="w-full flex-1 min-h-[500px] border rounded-md p-3 text-sm font-mono resize-y bg-background-primary text-text-primary border-border-primary focus:outline-none focus:ring-2 focus:ring-blue-500"
                onChange={(e) => setContent(e.target.value)}
                placeholder={t('prompts.contentPlaceholder')}
                spellCheck={false}
              />
            </div>

            {hasChanges && (
              <div className="text-sm text-yellow-600 dark:text-yellow-400">
                {t('prompts.unsavedChanges')}
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-4 pr-4 pb-8 mt-1">
      <Card className="pb-2 rounded-lg border-yellow-500/50 bg-yellow-500/10">
        <CardHeader className="pb-2">
          <div className="flex items-start gap-3">
            <AlertTriangle className="h-5 w-5 text-yellow-500 flex-shrink-0 mt-1" />
            <div className="flex-1">
              <CardTitle className="text-yellow-600 dark:text-yellow-400">
                {t('prompts.warningTitle')}
              </CardTitle>
              <p className="text-sm text-text-secondary mt-2">{t('prompts.warningDescription')}</p>
            </div>
            {hasCustomizedPrompts && (
              <Button
                variant="outline"
                size="sm"
                onClick={handleResetAll}
                className="flex items-center gap-2 border-yellow-500/50 hover:bg-yellow-500/20"
              >
                <RotateCcw className="h-4 w-4" />
                {t('prompts.resetAll')}
              </Button>
            )}
          </div>
        </CardHeader>
        <CardContent className="px-4 pt-4">
          <div className="space-y-2">
            {prompts.map((prompt) => (
              <div
                key={prompt.name}
                className="flex items-center justify-between p-3 rounded-lg border border-border-primary hover:bg-background-secondary transition-colors"
              >
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <h4 className="font-medium text-text-primary truncate">{prompt.name}</h4>
                    {prompt.is_customized && (
                      <span className="px-2 py-0.5 text-xs rounded-full bg-blue-500/20 text-blue-600 dark:text-blue-400">
                        {t('prompts.customized')}
                      </span>
                    )}
                  </div>
                  <p className="text-sm text-text-secondary mt-0.5 truncate">
                    {prompt.description}
                  </p>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setSelectedPrompt(prompt.name)}
                  className="ml-4"
                >
                  {t('prompts.edit')}
                </Button>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
