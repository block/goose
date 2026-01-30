import React, { useRef, useEffect } from 'react';

interface HackerASCIITextProps {
  text?: string;
}

export const HackerASCIIText: React.FC<HackerASCIITextProps> = ({ text = 'GOOSE' }) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const asciiRef = useRef<HTMLPreElement>(null);

  useEffect(() => {
    if (!canvasRef.current || !asciiRef.current) return;

    const canvas = canvasRef.current;
    const asciiElement = asciiRef.current;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const width = 800;
    const height = 400;
    canvas.width = width;
    canvas.height = height;

    const characters = ' .:-=+*#%@';
    const cellSize = 8;
    const cols = Math.floor(width / cellSize);
    const rows = Math.floor(height / cellSize);

    let time = 0;

    const drawText = () => {
      // Clear canvas
      ctx.fillStyle = 'black';
      ctx.fillRect(0, 0, width, height);

      // Draw text with wave effect
      ctx.font = 'bold 120px monospace';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillStyle = 'white';

      const centerX = width / 2;
      const centerY = height / 2;

      // Apply wave distortion
      ctx.save();
      ctx.translate(centerX, centerY);
      ctx.rotate(Math.sin(time * 0.5) * 0.05);
      
      // Draw each letter with individual wave
      const letters = text.split('');
      const letterWidth = 100;
      const totalWidth = letters.length * letterWidth;
      const startX = -totalWidth / 2;

      letters.forEach((letter, i) => {
        const x = startX + i * letterWidth + letterWidth / 2;
        const y = Math.sin(time + i * 0.5) * 20;
        ctx.fillText(letter, x, y);
      });

      ctx.restore();
    };

    const convertToASCII = () => {
      const imageData = ctx.getImageData(0, 0, width, height);
      let asciiArt = '';

      for (let y = 0; y < rows; y++) {
        for (let x = 0; x < cols; x++) {
          const pixelX = x * cellSize;
          const pixelY = y * cellSize;
          const pixelIndex = (pixelY * width + pixelX) * 4;

          const r = imageData.data[pixelIndex];
          const g = imageData.data[pixelIndex + 1];
          const b = imageData.data[pixelIndex + 2];
          const brightness = (r + g + b) / 3;

          const charIndex = Math.floor((brightness / 255) * (characters.length - 1));
          asciiArt += characters[charIndex];
        }
        asciiArt += '\n';
      }

      return asciiArt;
    };

    let animationFrameId: number;

    const animate = () => {
      animationFrameId = requestAnimationFrame(animate);
      time += 0.02;

      drawText();
      const asciiArt = convertToASCII();
      asciiElement.textContent = asciiArt;
    };

    animate();

    return () => {
      cancelAnimationFrame(animationFrameId);
    };
  }, [text]);

  return (
    <div className="absolute inset-0 flex items-center justify-center pointer-events-none z-10">
      <canvas ref={canvasRef} className="hidden" />
      <pre
        ref={asciiRef}
        className="text-[#39ff14] text-[8px] leading-[8px] font-mono whitespace-pre opacity-60"
        style={{
          textShadow: '0 0 10px rgba(57, 255, 20, 0.5)',
        }}
      />
    </div>
  );
};

export default HackerASCIIText;
