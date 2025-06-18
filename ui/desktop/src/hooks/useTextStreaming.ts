import React from 'react';

export default function useTextStreaming(
  fullText: string,
  shouldStream: boolean,
  interval: number = 15
): string {
  const [displayText, setDisplayText] = React.useState<string>(shouldStream ? '' : fullText);

  React.useEffect(() => {
    if (!shouldStream) {
      // If we shouldn't stream, show full text immediately
      setDisplayText(fullText);
      return;
    }

    // Reset for new streaming
    setDisplayText('');
    let index = 0;
    const timer = setInterval(() => {
      index += 1;
      setDisplayText(fullText.slice(0, index));
      if (index >= fullText.length) {
        clearInterval(timer);
      }
    }, interval);

    return () => clearInterval(timer);
  }, [fullText, shouldStream, interval]);

  return displayText;
}
