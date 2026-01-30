import React, { useEffect, useState, useRef } from 'react';

export const CyberpunkWidgets: React.FC = () => {
  const [time, setTime] = useState(new Date());
  const [cpuData, setCpuData] = useState<number[]>([]);
  const [networkData, setNetworkData] = useState<number[]>([]);
  const canvasRef = useRef<HTMLCanvasElement>(null);

  // Update time every second
  useEffect(() => {
    const timer = setInterval(() => {
      setTime(new Date());
    }, 1000);
    return () => clearInterval(timer);
  }, []);

  // Generate random data for graphs
  useEffect(() => {
    const interval = setInterval(() => {
      setCpuData(prev => {
        const newData = [...prev, Math.random() * 100];
        return newData.slice(-20); // Keep last 20 data points
      });
      setNetworkData(prev => {
        const newData = [...prev, Math.random() * 100];
        return newData.slice(-20);
      });
    }, 500);
    return () => clearInterval(interval);
  }, []);

  // Draw graph on canvas
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || cpuData.length === 0) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const width = canvas.width;
    const height = canvas.height;

    // Clear canvas
    ctx.clearRect(0, 0, width, height);

    // Draw grid
    ctx.strokeStyle = 'rgba(255, 0, 255, 0.1)';
    ctx.lineWidth = 1;
    for (let i = 0; i < 5; i++) {
      const y = (height / 4) * i;
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(width, y);
      ctx.stroke();
    }

    // Draw CPU line
    ctx.strokeStyle = '#ff00ff';
    ctx.lineWidth = 2;
    ctx.beginPath();
    cpuData.forEach((value, index) => {
      const x = (width / 20) * index;
      const y = height - (value / 100) * height;
      if (index === 0) {
        ctx.moveTo(x, y);
      } else {
        ctx.lineTo(x, y);
      }
    });
    ctx.stroke();

    // Draw Network line
    ctx.strokeStyle = '#00ffff';
    ctx.lineWidth = 2;
    ctx.beginPath();
    networkData.forEach((value, index) => {
      const x = (width / 20) * index;
      const y = height - (value / 100) * height;
      if (index === 0) {
        ctx.moveTo(x, y);
      } else {
        ctx.lineTo(x, y);
      }
    });
    ctx.stroke();
  }, [cpuData, networkData]);

  const formatTime = (date: Date) => {
    return date.toLocaleTimeString('en-US', { 
      hour12: false,
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit'
    });
  };

  const formatDate = (date: Date) => {
    return date.toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric'
    });
  };

  return (
    <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 flex flex-col gap-3 z-20 pointer-events-none">
      {/* Digital Clock Widget */}
      <div className="bg-gradient-to-br from-[#0a0015]/90 to-[#1a0033]/90 backdrop-blur-md border-2 border-[#ff00ff]/30 rounded-lg p-3 shadow-[0_0_20px_rgba(255,0,255,0.3)]">
        <div className="text-transparent bg-clip-text bg-gradient-to-r from-[#ff00ff] to-[#00ffff] text-2xl font-bold font-mono tracking-tight">
          {formatTime(time)}
        </div>
        <div className="text-[#ff00ff]/60 text-xs font-mono mt-1">{formatDate(time)}</div>
      </div>

      {/* System Monitor Widget */}
      <div className="bg-gradient-to-br from-[#0a0015]/90 to-[#1a0033]/90 backdrop-blur-md border-2 border-[#00ffff]/30 rounded-lg p-3 shadow-[0_0_20px_rgba(0,255,255,0.3)] w-64">
        <canvas 
          ref={canvasRef} 
          width={240} 
          height={80}
          className="w-full h-20"
        />
        <div className="flex justify-between mt-2 text-xs font-mono">
          <div className="flex items-center gap-1">
            <div className="w-2 h-2 bg-[#ff00ff] rounded-full animate-pulse"></div>
            <span className="text-[#ff00ff]">CPU</span>
          </div>
          <div className="flex items-center gap-1">
            <div className="w-2 h-2 bg-[#00ffff] rounded-full animate-pulse"></div>
            <span className="text-[#00ffff]">NET</span>
          </div>
        </div>
      </div>

      {/* Status Widget */}
      <div className="bg-gradient-to-br from-[#0a0015]/90 to-[#1a0033]/90 backdrop-blur-md border-2 border-[#ff00ff]/30 rounded-lg p-3 shadow-[0_0_20px_rgba(255,0,255,0.3)]">
        <div className="space-y-1">
          <div className="flex items-center justify-between text-xs font-mono">
            <span className="text-[#ff00ff]/70">NEURAL LINK</span>
            <span className="text-[#00ffff]">ACTIVE</span>
          </div>
          <div className="flex items-center justify-between text-xs font-mono">
            <span className="text-[#ff00ff]/70">ENCRYPTION</span>
            <span className="text-[#00ff00]">256-BIT</span>
          </div>
          <div className="flex items-center justify-between text-xs font-mono">
            <span className="text-[#ff00ff]/70">UPLINK</span>
            <span className="text-[#00ffff]">98.7%</span>
          </div>
        </div>
      </div>

      {/* Hex Display Widget */}
      <div className="bg-gradient-to-br from-[#0a0015]/90 to-[#1a0033]/90 backdrop-blur-md border-2 border-[#00ffff]/30 rounded-lg p-3 shadow-[0_0_20px_rgba(0,255,255,0.3)]">
        <div className="text-[#00ffff] text-xs font-mono leading-tight opacity-70">
          {Array.from({ length: 3 }, (_, i) => (
            <div key={i}>
              {Math.random().toString(16).substring(2, 18).toUpperCase()}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
