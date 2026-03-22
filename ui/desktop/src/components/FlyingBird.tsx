import { useState, useEffect } from 'react';
import { useGlyphPack } from '../contexts/GlyphContext';

interface FlyingBirdProps {
  className?: string;
  cycleInterval?: number; // milliseconds between bird frame changes
}

export default function FlyingBird({ className = '', cycleInterval = 150 }: FlyingBirdProps) {
  const { pack } = useGlyphPack();
  const frames = pack.AnimationFrames;
  const [currentFrameIndex, setCurrentFrameIndex] = useState(0);

  useEffect(() => {
    if (!frames) return;
    const interval = setInterval(() => {
      setCurrentFrameIndex((prevIndex) => (prevIndex + 1) % frames.length);
    }, cycleInterval);
    return () => clearInterval(interval);
  }, [cycleInterval, frames]);

  if (frames) {
    const CurrentFrame = frames[currentFrameIndex];
    return (
      <div className={`transition-opacity duration-75 ${className}`}>
        <CurrentFrame className="w-4 h-4" />
      </div>
    );
  }

  // Fallback: static glyph for packs without frame animation
  return (
    <div className={`transition-opacity duration-75 ${className}`}>
      <pack.StaticGlyph className="w-4 h-4" />
    </div>
  );
}
