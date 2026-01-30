import React, { useEffect, useRef, useState } from 'react';

interface AmberGlowProps {
  className?: string;
}

export const AmberGlow: React.FC<AmberGlowProps> = ({ className = '' }) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [mousePos, setMousePos] = useState({ x: 0, y: 0 });
  const [isHovering, setIsHovering] = useState(false);
  const trailRef = useRef<Array<{ x: number; y: number; age: number }>>([]);
  const animationFrameRef = useRef<number>();

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const parent = canvas.parentElement;
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
      setMousePos({
        x: e.clientX - rect.left,
        y: e.clientY - rect.top
      });
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

    let lastTime = performance.now();
    const maxTrailLength = 20;

    const animate = (currentTime: number) => {
      const deltaTime = currentTime - lastTime;
      lastTime = currentTime;

      // Clear canvas
      ctx.clearRect(0, 0, canvas.width, canvas.height);

      if (isHovering) {
        // Add new trail point
        trailRef.current.push({ x: mousePos.x, y: mousePos.y, age: 0 });
        
        // Keep only recent trail points
        if (trailRef.current.length > maxTrailLength) {
          trailRef.current.shift();
        }
      }

      // Update and draw trail
      trailRef.current = trailRef.current.filter(point => {
        point.age += deltaTime;
        return point.age < 1000; // Keep for 1 second
      });

      // Draw trail with amber glow
      trailRef.current.forEach((point, index) => {
        const progress = point.age / 1000;
        const opacity = (1 - progress) * 0.8;
        const size = 80 - (progress * 40);

        // Create radial gradient for glow effect
        const gradient = ctx.createRadialGradient(
          point.x, point.y, 0,
          point.x, point.y, size
        );

        // Amber color palette
        gradient.addColorStop(0, `rgba(255, 191, 0, ${opacity * 0.8})`);
        gradient.addColorStop(0.3, `rgba(255, 140, 0, ${opacity * 0.5})`);
        gradient.addColorStop(0.6, `rgba(255, 100, 0, ${opacity * 0.2})`);
        gradient.addColorStop(1, `rgba(255, 69, 0, 0)`);

        ctx.fillStyle = gradient;
        ctx.fillRect(point.x - size, point.y - size, size * 2, size * 2);
      });

      // Draw main cursor glow
      if (isHovering) {
        const mainGradient = ctx.createRadialGradient(
          mousePos.x, mousePos.y, 0,
          mousePos.x, mousePos.y, 100
        );

        mainGradient.addColorStop(0, 'rgba(255, 191, 0, 0.9)');
        mainGradient.addColorStop(0.2, 'rgba(255, 140, 0, 0.6)');
        mainGradient.addColorStop(0.5, 'rgba(255, 100, 0, 0.3)');
        mainGradient.addColorStop(0.8, 'rgba(255, 69, 0, 0.1)');
        mainGradient.addColorStop(1, 'rgba(255, 69, 0, 0)');

        ctx.fillStyle = mainGradient;
        ctx.fillRect(mousePos.x - 100, mousePos.y - 100, 200, 200);

        // Add bright center point
        const centerGradient = ctx.createRadialGradient(
          mousePos.x, mousePos.y, 0,
          mousePos.x, mousePos.y, 20
        );

        centerGradient.addColorStop(0, 'rgba(255, 220, 100, 1)');
        centerGradient.addColorStop(0.5, 'rgba(255, 191, 0, 0.8)');
        centerGradient.addColorStop(1, 'rgba(255, 140, 0, 0)');

        ctx.fillStyle = centerGradient;
        ctx.fillRect(mousePos.x - 20, mousePos.y - 20, 40, 40);
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
  }, [mousePos, isHovering]);

  return (
    <canvas
      ref={canvasRef}
      className={`absolute inset-0 pointer-events-none z-5 ${className}`}
      style={{ mixBlendMode: 'screen' }}
    />
  );
};
