import { AppEvents } from '../../../constants/events';
import { useEffect, useState } from 'react';
import { createResponseStyles, ResponseStyleSelectionItem } from './ResponseStyleSelectionItem';
import { useLocalization } from '../../../contexts/LocalizationContext';

export const ResponseStylesSection = () => {
  const { t } = useLocalization();
  const allResponseStyles = createResponseStyles(t);
  const [currentStyle, setCurrentStyle] = useState('concise');

  useEffect(() => {
    async function loadResponseStyle() {
      try {
        const savedStyle = await window.electron.getSetting('responseStyle');
        setCurrentStyle(savedStyle);
      } catch (error) {
        console.error('Error loading response style:', error);
      }
    }
    loadResponseStyle();
  }, []);

  const handleStyleChange = async (newStyle: string) => {
    setCurrentStyle(newStyle);
    try {
      await window.electron.setSetting('responseStyle', newStyle);
    } catch (error) {
      console.error('Error saving response style:', error);
    }

    // Dispatch custom event to notify other components of the change
    window.dispatchEvent(new CustomEvent(AppEvents.RESPONSE_STYLE_CHANGED));
  };

  return (
    <div className="space-y-1">
      {allResponseStyles.map((style) => (
        <ResponseStyleSelectionItem
          key={style.key}
          style={style}
          currentStyle={currentStyle}
          showDescription={true}
          handleStyleChange={handleStyleChange}
        />
      ))}
    </div>
  );
};
