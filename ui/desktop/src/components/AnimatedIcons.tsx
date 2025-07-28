import { useState, useEffect } from 'react';
import {
  CodeXml,
  Cog,
  Fuel,
  GalleryHorizontalEnd,
  Gavel,
  GlassWater,
  Grape,
  Watch0,
  Watch1,
  Watch2,
  Watch3,
  Watch4,
  Watch5,
  Watch6,
} from './icons';

interface AnimatedIconsProps {
  className?: string;
  cycleInterval?: number; // milliseconds between icon changes
  variant?: 'thinking' | 'waiting';
}

const thinkingIcons = [CodeXml, Cog, Fuel, GalleryHorizontalEnd, Gavel, GlassWater, Grape];
const waitingIcons = [Watch0, Watch1, Watch2, Watch3, Watch4, Watch5, Watch6];

export default function AnimatedIcons({
  className = '',
  cycleInterval = 500,
  variant = 'thinking',
}: AnimatedIconsProps) {
  const [currentIconIndex, setCurrentIconIndex] = useState(0);
  const icons = variant === 'thinking' ? thinkingIcons : waitingIcons;

  useEffect(() => {
    const interval = setInterval(() => {
      setCurrentIconIndex((prevIndex) => (prevIndex + 1) % icons.length);
    }, cycleInterval);

    return () => clearInterval(interval);
  }, [cycleInterval, icons]);

  const CurrentIcon = icons[currentIconIndex];

  return (
    <div className={`transition-opacity duration-200 w-4 h-4 ${className}`}>
      <CurrentIcon className="w-full h-full" />
    </div>
  );
}
