import React from 'react';

interface MatrixDancingLinesProps {
  className?: string;
}

export const MatrixDancingLines: React.FC<MatrixDancingLinesProps> = ({ className }) => {
  return (
    <svg
      fill="none"
      preserveAspectRatio="none"
      viewBox="0 0 1200 800"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
    >
      <defs>
        {/* Gradient for the flowing lines - lighter opacities for better light mode visibility */}
        <linearGradient id="flow-gradient-1" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="currentColor" stopOpacity="0.08" />
          <stop offset="50%" stopColor="currentColor" stopOpacity="0.04" />
          <stop offset="100%" stopColor="currentColor" stopOpacity="0.01" />
        </linearGradient>
        <linearGradient id="flow-gradient-2" x1="100%" y1="0%" x2="0%" y2="100%">
          <stop offset="0%" stopColor="currentColor" stopOpacity="0.06" />
          <stop offset="50%" stopColor="currentColor" stopOpacity="0.03" />
          <stop offset="100%" stopColor="currentColor" stopOpacity="0.005" />
        </linearGradient>
        <linearGradient id="flow-gradient-3" x1="0%" y1="100%" x2="100%" y2="0%">
          <stop offset="0%" stopColor="currentColor" stopOpacity="0.05" />
          <stop offset="50%" stopColor="currentColor" stopOpacity="0.025" />
          <stop offset="100%" stopColor="currentColor" stopOpacity="0.005" />
        </linearGradient>
      </defs>
      
      {/* Flowing spline curves */}
      <g opacity="0.5">
        {/* First flowing line */}
        <path
          d="M-100,200 Q200,50 400,150 T800,100 Q1000,80 1300,200"
          stroke="url(#flow-gradient-1)"
          strokeWidth="3"
          fill="none"
          opacity="0.4"
        />
        
        {/* Second flowing line */}
        <path
          d="M-50,400 Q150,250 350,350 T750,300 Q950,280 1250,400"
          stroke="url(#flow-gradient-2)"
          strokeWidth="2"
          fill="none"
          opacity="0.3"
        />
        
        {/* Third flowing line */}
        <path
          d="M-150,600 Q100,450 300,550 T700,500 Q900,480 1200,600"
          stroke="url(#flow-gradient-1)"
          strokeWidth="2.5"
          fill="none"
          opacity="0.25"
        />
        
        {/* Fourth flowing line */}
        <path
          d="M0,100 Q300,20 500,120 T900,80 Q1100,60 1400,180"
          stroke="url(#flow-gradient-3)"
          strokeWidth="1.5"
          fill="none"
          opacity="0.35"
        />
        
        {/* Fifth flowing line */}
        <path
          d="M-200,750 Q50,600 250,700 T650,650 Q850,630 1150,750"
          stroke="url(#flow-gradient-2)"
          strokeWidth="2"
          fill="none"
          opacity="0.2"
        />
      </g>
      
      {/* Additional subtle background shapes */}
      <g opacity="0.15">
        <ellipse cx="300" cy="200" rx="150" ry="80" fill="url(#flow-gradient-1)" opacity="0.05" />
        <ellipse cx="800" cy="500" rx="200" ry="100" fill="url(#flow-gradient-2)" opacity="0.04" />
        <ellipse cx="600" cy="700" rx="120" ry="60" fill="url(#flow-gradient-3)" opacity="0.06" />
      </g>
    </svg>
  );
};

export default MatrixDancingLines;
