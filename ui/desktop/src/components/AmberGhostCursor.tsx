import React, { useEffect, useRef, useState } from 'react';

interface AmberGhostCursorProps {
  trailLength?: number;
  inertia?: number;
  brightness?: number;
  color?: string;
  edgeIntensity?: number;
}

export const AmberGhostCursor: React.FC<AmberGhostCursorProps> = ({
  trailLength = 50,
  inertia = 0.5,
  brightness = 2,
  color = '#B19EEF',
  edgeIntensity = 0,
}) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [mousePos, setMousePos] = useState({ x: 0.5, y: 0.5 });
  const [isHovering, setIsHovering] = useState(false);
  const trailRef = useRef<Array<{ x: number; y: number; age: number }>>([]);
  const animationFrameRef = useRef<number>();
  const velocityRef = useRef({ x: 0, y: 0 });
  const currentPosRef = useRef({ x: 0.5, y: 0.5 });
  const timeRef = useRef(0);

  // Parse hex color to RGB
  const hexToRgb = (hex: string) => {
    const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
    return result ? {
      r: parseInt(result[1], 16) / 255,
      g: parseInt(result[2], 16) / 255,
      b: parseInt(result[3], 16) / 255
    } : { r: 0.69, g: 0.62, b: 0.94 }; // Default purple
  };

  const baseColor = hexToRgb(color);

  useEffect(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;

    const ctx = canvas.getContext('2d', { alpha: true });
    if (!ctx) return;

    const parent = container.parentElement;
    if (!parent) return;

    const resize = () => {
      const rect = parent.getBoundingClientRect();
      canvas.width = rect.width;
      canvas.height = rect.height;
    };

    resize();
    const resizeObserver = new ResizeObserver(resize);
    resizeObserver.observe(parent);

    const handleMouseMove = (e: MouseEvent) => {
      const rect = parent.getBoundingClientRect();
      const x = (e.clientX - rect.left) / rect.width;
      const y = 1 - (e.clientY - rect.top) / rect.height;
      setMousePos({ x, y });
      setIsHovering(true);
    };

    const handleMouseEnter = () => {
      setIsHovering(true);
    };

    const handleMouseLeave = () => {
      setIsHovering(false);
    };

    parent.addEventListener('mousemove', handleMouseMove);
    parent.addEventListener('mouseenter', handleMouseEnter);
    parent.addEventListener('mouseleave', handleMouseLeave);

    // Simplified noise function
    const noise = (x: number, y: number) => {
      const n = Math.sin(x * 12.9898 + y * 78.233) * 43758.5453;
      return n - Math.floor(n);
    };

    const fbm = (x: number, y: number, time: number) => {
      let value = 0;
      let amplitude = 0.5;
      let frequency = 1;
      
      for (let i = 0; i < 5; i++) {
        value += amplitude * noise(x * frequency + time * 0.1, y * frequency + time * 0.1);
        frequency *= 2;
        amplitude *= 0.5;
      }
      
      return value;
    };

    const drawBlob = (
      ctx: CanvasRenderingContext2D,
      x: number,
      y: number,
      intensity: number,
      time: number,
      width: number,
      height: number
    ) => {
      const scale = Math.max(width, height) / 600;
      const radius = (0.5 + 0.3 * scale) * 100 * intensity;
      
      // Create gradient
      const gradient = ctx.createRadialGradient(x, y, 0, x, y, radius);
      
      // Apply FBM-like noise to create smoky effect
      const noiseValue = fbm(x / 100, y / 100, time);
      const alpha = Math.pow(noiseValue, 2.5) * intensity * brightness;
      
      // Color with tint variations
      const tint1 = {
        r: Math.min(1, baseColor.r + 0.15),
        g: Math.min(1, baseColor.g + 0.15),
        b: Math.min(1, baseColor.b + 0.15)
      };
      
      const tint2 = {
        r: Math.min(1, baseColor.r * 0.8 + 0.2),
        g: Math.min(1, baseColor.g * 0.9 + 0.1),
        b: Math.min(1, baseColor.b + 0.2)
      };
      
      const mixFactor = Math.sin(time * 0.5) * 0.5 + 0.5;
      const finalColor = {
        r: Math.floor((tint1.r * (1 - mixFactor) + tint2.r * mixFactor) * 255),
        g: Math.floor((tint1.g * (1 - mixFactor) + tint2.g * mixFactor) * 255),
        b: Math.floor((tint1.b * (1 - mixFactor) + tint2.b * mixFactor) * 255)
      };
      
      gradient.addColorStop(0, `rgba(${finalColor.r}, ${finalColor.g}, ${finalColor.b}, ${alpha})`);
      gradient.addColorStop(0.5, `rgba(${finalColor.r}, ${finalColor.g}, ${finalColor.b}, ${alpha * 0.5})`);
      gradient.addColorStop(1, `rgba(${finalColor.r}, ${finalColor.g}, ${finalColor.b}, 0)`);
      
      ctx.fillStyle = gradient;
      ctx.fillRect(x - radius, y - radius, radius * 2, radius * 2);
    };

    const animate = (currentTime: number) => {
      timeRef.current = currentTime / 1000;

      // Clear canvas
      ctx.clearRect(0, 0, canvas.width, canvas.height);

      // Apply inertia to smooth movement
      if (isHovering) {
        velocityRef.current.x = (mousePos.x - currentPosRef.current.x) * (1 - inertia);
        velocityRef.current.y = (mousePos.y - currentPosRef.current.y) * (1 - inertia);
        currentPosRef.current.x += velocityRef.current.x;
        currentPosRef.current.y += velocityRef.current.y;
      } else {
        velocityRef.current.x *= inertia;
        velocityRef.current.y *= inertia;
        currentPosRef.current.x += velocityRef.current.x;
        currentPosRef.current.y += velocityRef.current.y;
      }

      // Add to trail
      if (isHovering || Math.abs(velocityRef.current.x) > 0.001 || Math.abs(velocityRef.current.y) > 0.001) {
        trailRef.current.push({
          x: currentPosRef.current.x,
          y: currentPosRef.current.y,
          age: 0
        });

        if (trailRef.current.length > trailLength) {
          trailRef.current.shift();
        }
      }

      // Update trail ages
      trailRef.current = trailRef.current.map(point => ({
        ...point,
        age: point.age + 16 // ~60fps
      })).filter(point => point.age < 2000);

      // Draw trail
      trailRef.current.forEach((point, index) => {
        const t = 1 - index / trailRef.current.length;
        const intensity = Math.pow(t, 2) * 0.8;
        const x = point.x * canvas.width;
        const y = (1 - point.y) * canvas.height;
        
        if (intensity > 0.01) {
          drawBlob(ctx, x, y, intensity, timeRef.current, canvas.width, canvas.height);
        }
      });

      // Draw main cursor blob
      if (isHovering || Math.abs(velocityRef.current.x) > 0.001 || Math.abs(velocityRef.current.y) > 0.001) {
        const x = currentPosRef.current.x * canvas.width;
        const y = (1 - currentPosRef.current.y) * canvas.height;
        drawBlob(ctx, x, y, 1.0, timeRef.current, canvas.width, canvas.height);
      }

      animationFrameRef.current = requestAnimationFrame(animate);
    };

    animationFrameRef.current = requestAnimationFrame(animate);

    return () => {
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
      parent.removeEventListener('mousemove', handleMouseMove);
      parent.removeEventListener('mouseenter', handleMouseEnter);
      parent.removeEventListener('mouseleave', handleMouseLeave);
      resizeObserver.disconnect();
    };
  }, [mousePos, isHovering, trailLength, inertia, brightness, baseColor.r, baseColor.g, baseColor.b, edgeIntensity]);

  return (
    <div ref={containerRef} className="absolute inset-0 pointer-events-none" style={{ zIndex: 10, mixBlendMode: 'screen' }}>
      <canvas ref={canvasRef} className="w-full h-full" />
    </div>
  );
};
