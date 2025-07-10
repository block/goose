import React, { useEffect, useRef } from 'react';

interface WaveformVisualizerProps {
  audioContext: AudioContext | null;
  analyser: AnalyserNode | null;
  isRecording: boolean;
}

export const WaveformVisualizer: React.FC<WaveformVisualizerProps> = ({
  analyser,
  isRecording,
}) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animationRef = useRef<number>();

  useEffect(() => {
    if (!canvasRef.current || !analyser || !isRecording) return;

    const canvas = canvasRef.current;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Set canvas size
    const dpr = window.devicePixelRatio || 1;
    const rect = canvas.getBoundingClientRect();
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    ctx.scale(dpr, dpr);

    // Configure analyser
    analyser.fftSize = 256;
    const bufferLength = analyser.frequencyBinCount;
    const dataArray = new Uint8Array(bufferLength);

    // Visual settings
    const barWidth = 3;
    const barSpacing = 2;
    const barCount = Math.floor(rect.width / (barWidth + barSpacing));
    const barMaxHeight = rect.height * 0.8;
    const barMinHeight = 2;

    // Smoothing for bars
    const smoothedHeights = new Array(barCount).fill(0);
    const targetHeights = new Array(barCount).fill(0);

    const draw = () => {
      if (!isRecording) return;

      animationRef.current = requestAnimationFrame(draw);

      // Get frequency data
      analyser.getByteFrequencyData(dataArray);

      // Clear canvas
      ctx.clearRect(0, 0, rect.width, rect.height);

      // Calculate target heights based on frequency data
      for (let i = 0; i < barCount; i++) {
        const dataIndex = Math.floor((i / barCount) * bufferLength * 0.5); // Use lower frequencies
        const value = dataArray[dataIndex] / 255;

        // Apply some randomness and minimum height for visual interest
        const randomFactor = 0.85 + Math.random() * 0.3;
        targetHeights[i] = Math.max(barMinHeight, value * barMaxHeight * randomFactor);
      }

      // Smooth the bar heights
      for (let i = 0; i < barCount; i++) {
        const diff = targetHeights[i] - smoothedHeights[i];
        smoothedHeights[i] += diff * 0.3; // Smoothing factor
      }

      // Draw bars
      for (let i = 0; i < barCount; i++) {
        const x = i * (barWidth + barSpacing) + barSpacing;
        const barHeight = smoothedHeights[i];
        const y = (rect.height - barHeight) / 2;

        // Create gradient for each bar
        const gradient = ctx.createLinearGradient(0, y, 0, y + barHeight);

        // Dynamic color based on height
        const intensity = barHeight / barMaxHeight;
        const hue = 200 + intensity * 20; // Blue to cyan
        const saturation = 50 + intensity * 50;
        const lightness = 50 + intensity * 20;

        gradient.addColorStop(0, `hsla(${hue}, ${saturation}%, ${lightness}%, 0.3)`);
        gradient.addColorStop(0.5, `hsla(${hue}, ${saturation}%, ${lightness}%, 0.8)`);
        gradient.addColorStop(1, `hsla(${hue}, ${saturation}%, ${lightness}%, 0.3)`);

        ctx.fillStyle = gradient;
        ctx.fillRect(x, y, barWidth, barHeight);
      }
    };

    draw();

    return () => {
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current);
      }
    };
  }, [analyser, isRecording]);

  return (
    <canvas
      ref={canvasRef}
      className="absolute inset-0 w-full h-full pointer-events-none"
      style={{ opacity: 0.9 }}
    />
  );
};
