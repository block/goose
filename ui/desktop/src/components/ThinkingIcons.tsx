import { useState, useEffect } from 'react';
import { CodeXml, Cog, Fuel, GalleryHorizontalEnd, Gavel, GlassWater, Grape } from './icons';

interface ThinkingIconsProps {
  className?: string;
  cycleInterval?: number; // milliseconds between icon changes
}

const thinkingIcons = [
  CodeXml,
  Cog,
  Fuel,
  GalleryHorizontalEnd,
  Gavel,
  GlassWater,
  Grape,
];

export default function ThinkingIcons({ 
  className = '', 
  cycleInterval = 500 
}: ThinkingIconsProps) {
  const [currentIconIndex, setCurrentIconIndex] = useState(0);

  useEffect(() => {
    const interval = setInterval(() => {
      setCurrentIconIndex((prevIndex) => 
        (prevIndex + 1) % thinkingIcons.length
      );
    }, cycleInterval);

    return () => clearInterval(interval);
  }, [cycleInterval]);

  const CurrentIcon = thinkingIcons[currentIconIndex];

  return (
    <div className={`transition-opacity duration-200 ${className}`}>
      <CurrentIcon className="w-4 h-4" />
    </div>
  );
}
