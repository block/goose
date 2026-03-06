import React from 'react';
import { Languages } from 'lucide-react';
import { Button } from '../../ui/button';
import { useLocalization } from '../../../contexts/LocalizationContext';

interface LanguageSelectorProps {
  className?: string;
}

const LanguageSelector: React.FC<LanguageSelectorProps> = ({ className = '' }) => {
  const { language, setLanguage, t } = useLocalization();

  return (
    <div className={`flex gap-1 ${className}`}>
      <Button
        data-testid="language-en-button"
        onClick={() => setLanguage('en')}
        className={`flex items-center justify-center gap-1 p-2 rounded-md border transition-colors text-xs ${
          language === 'en'
            ? 'bg-background-inverse text-text-inverse border-text-inverse hover:!bg-background-inverse hover:!text-text-inverse'
            : 'border-border-primary hover:!bg-background-secondary text-text-secondary hover:text-text-primary'
        }`}
        variant="ghost"
        size="sm"
      >
        <Languages className="h-3 w-3" />
        <span>{t('language.english')}</span>
      </Button>

      <Button
        data-testid="language-zh-cn-button"
        onClick={() => setLanguage('zh-CN')}
        className={`flex items-center justify-center gap-1 p-2 rounded-md border transition-colors text-xs ${
          language === 'zh-CN'
            ? 'bg-background-inverse text-text-inverse border-text-inverse hover:!bg-background-inverse hover:!text-text-inverse'
            : 'border-border-primary hover:!bg-background-secondary text-text-secondary hover:text-text-primary'
        }`}
        variant="ghost"
        size="sm"
      >
        <Languages className="h-3 w-3" />
        <span>{t('language.chineseSimplified')}</span>
      </Button>
    </div>
  );
};

export default LanguageSelector;
