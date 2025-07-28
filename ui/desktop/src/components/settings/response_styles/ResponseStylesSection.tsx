import { useEffect, useState } from 'react';
import { all_response_styles, ResponseStyleSelectionItem } from './ResponseStyleSelectionItem';

export const ResponseStylesSection = () => {
  const [currentStyle, setCurrentStyle] = useState('concise');

  useEffect(() => {
    const savedStyle = localStorage.getItem('response_style');
    if (savedStyle) {
      try {
        setCurrentStyle(savedStyle);
      } catch (error) {
        console.error('Error parsing response style:', error);
      }
    } else {
      // Set default to concise for new users
      localStorage.setItem('response_style', 'concise');
      setCurrentStyle('concise');
    }
  }, []);

  const handleStyleChange = async (newStyle: string) => {
    setCurrentStyle(newStyle);
    localStorage.setItem('response_style', newStyle);

    // Dispatch custom event to notify other components of the change
    window.dispatchEvent(new CustomEvent('responseStyleChanged'));
  };

  return (
    <div className="space-y-1">
      {all_response_styles.map((style) => (
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
