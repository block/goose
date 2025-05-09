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
  };

  return (
    <section id="responseStyles" className="px-8">
      <div className="flex justify-between items-center mb-2">
        <h2 className="text-xl font-medium text-textStandard">Response Styles</h2>
      </div>
      <div className="pb-8">
        <p className="text-sm text-textStandard mb-6">
          Choose how Goose should format and style its responses
        </p>
        <div>
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
      </div>
    </section>
  );
};
