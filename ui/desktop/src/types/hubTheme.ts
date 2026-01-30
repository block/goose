/**
 * Hub Theme Configuration
 * 
 * Defines the visual themes available for the Hub/Home page
 */

export interface HubTheme {
  id: string;
  name: string;
  description: string;
  
  // ASCII Canvas settings
  ascii: {
    enabled: boolean;
    cellSize: number;
    characters: string;
    color: string;
    animationSpeed: number;
    glitchIntensity: number;
  };
  
  // Background settings
  background: {
    color: string;
    gradient?: string;
  };
  
  // Text styling
  greeting: {
    show: boolean;
    fadeDelay: number;
    className: string;
    position: 'center' | 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right';
  };
  
  // Input styling and layout
  input: {
    className: string;
    position: 'center' | 'bottom-left' | 'bottom-right' | 'top-left' | 'top-right' | 'bottom-full';
    width: 'narrow' | 'medium' | 'wide' | 'full';
    rounded: 'none' | 'sm' | 'md' | 'lg' | 'xl' | '2xl' | 'full';
    border: boolean;
    borderColor?: string;
    prompt?: string; // Custom prompt text like "$ " or "> "
    
    // Advanced styling - inject custom CSS into ChatInput
    customStyles?: {
      // ChatInput container
      container?: string;
      // Textarea/contenteditable
      textarea?: string;
      // Send button
      sendButton?: string;
      // Placeholder text
      placeholder?: string;
      // Bottom controls (file attach, settings, etc)
      bottomControls?: string;
    };
  };
  
  // Layout configuration
  layout: {
    contentAlignment: 'center' | 'flex-start' | 'flex-end' | 'space-between';
    padding: 'none' | 'sm' | 'md' | 'lg';
  };
  
  // Typography overrides
  typography?: {
    fontFamily?: string; // Custom font family
    fontSize?: string; // Base font size
    fontWeight?: string; // Font weight
    letterSpacing?: string; // Letter spacing
    lineHeight?: string; // Line height
  };
  
  // Animation overrides
  animations?: {
    cursorBlink?: boolean; // Show blinking cursor
    cursorStyle?: 'block' | 'underline' | 'bar'; // Cursor style
    typewriterEffect?: boolean; // Typewriter effect on greeting
    glitchText?: boolean; // Glitch effect on text
    scanlines?: boolean; // CRT scanlines effect
    vignette?: boolean; // Vignette effect
  };
  
  // Effects and filters
  effects?: {
    blur?: number; // Background blur
    brightness?: number; // Brightness adjustment
    contrast?: number; // Contrast adjustment
    saturate?: number; // Saturation adjustment
    hueRotate?: number; // Hue rotation
    noise?: boolean; // Film grain/noise overlay
  };
  
  // Custom CSS injection (for advanced themes)
  customCSS?: string;
}

export const HUB_THEMES: Record<string, HubTheme> = {
  default: {
    id: 'default',
    name: 'Default',
    description: 'Clean and modern interface',
    ascii: {
      enabled: true,
      cellSize: 16,
      characters: ' .·:',
      color: 'rgba(var(--text-muted-rgb), 0.15)',
      animationSpeed: 0.008,
      glitchIntensity: 0.01,
    },
    background: {
      color: 'transparent',
    },
    greeting: {
      show: true,
      fadeDelay: 5000,
      className: 'text-5xl font-extralight text-text-default mb-6 tracking-tight',
      position: 'center',
    },
    input: {
      className: '',
      position: 'center',
      width: 'wide',
      rounded: '2xl',
      border: false,
      customStyles: {
        container: 'bg-background-default/80 backdrop-blur-md rounded-3xl shadow-xl',
        textarea: 'text-text-default placeholder:text-text-muted/50',
      },
    },
    layout: {
      contentAlignment: 'center',
      padding: 'lg',
    },
    typography: {
      fontSize: '15px',
      lineHeight: '1.6',
    },
  },
  
  terminal: {
    id: 'terminal',
    name: 'Terminal',
    description: 'Classic green-on-black terminal',
    ascii: {
      enabled: true,
      cellSize: 8,
      characters: '01█▓▒░',
      color: '#33ff33',
      animationSpeed: 0.035,
      glitchIntensity: 0.03,
    },
    background: {
      color: '#0a0a0a',
    },
    greeting: {
      show: true,
      fadeDelay: 4000,
      className: 'text-xl font-mono text-[#33ff33] mb-2 tracking-wide',
      position: 'top-left',
    },
    input: {
      className: '',
      position: 'bottom-left',
      width: 'full',
      rounded: '2xl',
      border: true,
      borderColor: '#33ff33',
      prompt: '$ ',
      customStyles: {
        container: 'font-mono bg-black/98 backdrop-blur-none border-2 border-[#33ff33]/40 border-l-4 shadow-[0_0_25px_rgba(51,255,51,0.4)]',
        textarea: 'text-[#33ff33] font-mono text-[15px] caret-[#33ff33] placeholder:text-[#33ff33]/30 tracking-wide',
        sendButton: 'bg-[#33ff33] text-black hover:bg-[#33ff33]/90 font-mono font-semibold',
        placeholder: 'text-[#33ff33]/30',
        bottomControls: 'text-[#33ff33]/50 hover:text-[#33ff33]',
      },
    },
    layout: {
      contentAlignment: 'space-between',
      padding: 'md',
    },
    typography: {
      fontFamily: '"Courier New", Courier, monospace',
      fontSize: '15px',
      fontWeight: '400',
      letterSpacing: '0.03em',
      lineHeight: '1.4',
    },
    animations: {
      cursorBlink: true,
      cursorStyle: 'block',
      scanlines: true,
    },
    effects: {
      brightness: 1.08,
      contrast: 1.25,
      saturate: 1.1,
    },
    customCSS: `
      @keyframes terminal-blink {
        0%, 49% { opacity: 1; }
        50%, 100% { opacity: 0; }
      }
      .terminal-cursor {
        animation: terminal-blink 1s infinite;
      }
      @keyframes terminal-flicker {
        0%, 100% { opacity: 1; }
        50% { opacity: 0.97; }
      }
    `,
  },
  
  matrix: {
    id: 'matrix',
    name: 'Matrix',
    description: 'Digital rain aesthetic',
    ascii: {
      enabled: true,
      cellSize: 14,
      characters: 'ｦｱｳｴｵｶｷｹｺｻｼｽｾｿﾀﾂﾃﾅﾆﾇﾈﾊﾋﾎﾏﾐﾑﾒﾓﾔﾕﾗﾘﾜ1234567890',
      color: '#00ff41',
      animationSpeed: 0.025,
      glitchIntensity: 0.08,
    },
    background: {
      color: '#0d0208',
    },
    greeting: {
      show: false,
      fadeDelay: 0,
      className: '',
      position: 'center',
    },
    input: {
      className: '',
      position: 'bottom-full',
      width: 'full',
      rounded: '2xl',
      border: true,
      borderColor: '#00ff41',
      prompt: '> ',
      customStyles: {
        container: 'font-mono bg-black/80 backdrop-blur-md border-t-4 border-[#00ff41] shadow-[0_0_30px_rgba(0,255,65,0.4)]',
        textarea: 'text-[#00ff41] font-mono tracking-wide caret-[#00ff41] placeholder:text-[#00ff41]/30',
        sendButton: 'bg-[#00ff41] text-black hover:bg-[#00ff41]/80 font-bold',
        placeholder: 'text-[#00ff41]/30',
        bottomControls: 'text-[#00ff41]/50 hover:text-[#00ff41]',
      },
    },
    layout: {
      contentAlignment: 'flex-end',
      padding: 'none',
    },
    typography: {
      fontFamily: '"Courier New", monospace',
      fontSize: '16px',
      letterSpacing: '0.1em',
    },
    animations: {
      cursorBlink: true,
      cursorStyle: 'bar',
      glitchText: true,
    },
    effects: {
      brightness: 1.15,
      saturate: 1.3,
    },
    customCSS: `
      @keyframes matrix-glitch {
        0% { transform: translateX(0); }
        20% { transform: translateX(-2px); }
        40% { transform: translateX(2px); }
        60% { transform: translateX(-1px); }
        80% { transform: translateX(1px); }
        100% { transform: translateX(0); }
      }
      .matrix-glitch {
        animation: matrix-glitch 0.3s infinite;
      }
    `,
  },
  
  amber: {
    id: 'amber',
    name: 'Amber',
    description: 'Vintage amber monochrome',
    ascii: {
      enabled: true,
      cellSize: 11,
      characters: ' .:-=+*#%@',
      color: '#ffb000',
      animationSpeed: 0.01,
      glitchIntensity: 0.03,
    },
    background: {
      color: '#1a0f00',
    },
    greeting: {
      show: true,
      fadeDelay: 5000,
      className: 'text-3xl font-mono text-[#ffb000] mb-3 uppercase',
      position: 'top-left',
    },
    input: {
      className: '',
      position: 'bottom-left',
      width: 'medium',
      rounded: '2xl',
      border: true,
      borderColor: '#ffb000',
      prompt: '> ',
      customStyles: {
        container: 'font-mono bg-[#1a0f00]/95 backdrop-blur-sm rounded-sm border-2 border-[#ffb000]/30 shadow-[0_0_25px_rgba(255,176,0,0.3)]',
        textarea: 'text-[#ffb000] font-mono caret-[#ffb000] placeholder:text-[#ffb000]/30',
        sendButton: 'bg-[#ffb000] text-[#1a0f00] hover:bg-[#ffb000]/80 font-mono font-bold',
        placeholder: 'text-[#ffb000]/30',
        bottomControls: 'text-[#ffb000]/50 hover:text-[#ffb000]',
      },
    },
    layout: {
      contentAlignment: 'flex-start',
      padding: 'md',
    },
    typography: {
      fontFamily: '"Courier New", Courier, monospace',
      fontSize: '15px',
      fontWeight: '500',
    },
    animations: {
      cursorBlink: true,
      cursorStyle: 'underline',
      scanlines: true,
      vignette: true,
    },
    effects: {
      brightness: 1.05,
      contrast: 1.15,
      saturate: 1.2,
    },
    customCSS: `
      @keyframes amber-flicker {
        0%, 100% { opacity: 1; }
        50% { opacity: 0.95; }
      }
      .amber-flicker {
        animation: amber-flicker 0.15s infinite;
      }
    `,
  },
  
  cyberpunk: {
    id: 'cyberpunk',
    name: 'Cyberpunk',
    description: 'Neon pink and cyan',
    ascii: {
      enabled: true,
      cellSize: 12,
      characters: '▓▒░█▀▄▌▐│─┤┐└┴┬├▪●◘◙',
      color: '#ff00ff',
      animationSpeed: 0.02,
      glitchIntensity: 0.15,
    },
    background: {
      color: '#0a0015',
      gradient: 'linear-gradient(135deg, #0a0015 0%, #1a0033 100%)',
    },
    greeting: {
      show: true,
      fadeDelay: 5000,
      className: 'text-5xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-[#ff00ff] to-[#00ffff] mb-4',
      position: 'center',
    },
    input: {
      className: '',
      position: 'center',
      width: 'wide',
      rounded: 'lg',
      border: true,
      borderColor: '#ff00ff',
      customStyles: {
        container: 'font-bold bg-gradient-to-r from-[#0a0015] to-[#1a0033] backdrop-blur-lg rounded-lg border-2 border-[#ff00ff]/50 shadow-[0_0_40px_rgba(255,0,255,0.4)]',
        textarea: 'text-transparent bg-clip-text bg-gradient-to-r from-[#ff00ff] to-[#00ffff] font-bold caret-[#ff00ff] placeholder:text-[#ff00ff]/30',
        sendButton: 'bg-gradient-to-r from-[#ff00ff] to-[#00ffff] text-white hover:opacity-80 font-black',
        placeholder: 'text-[#ff00ff]/30',
        bottomControls: 'text-[#ff00ff]/60 hover:text-[#00ffff]',
      },
    },
    layout: {
      contentAlignment: 'center',
      padding: 'lg',
    },
    typography: {
      fontFamily: '"Orbitron", "Rajdhani", sans-serif',
      fontSize: '16px',
      fontWeight: '700',
      letterSpacing: '0.05em',
    },
    animations: {
      glitchText: true,
      cursorBlink: true,
      cursorStyle: 'bar',
    },
    effects: {
      saturate: 1.5,
      contrast: 1.3,
    },
    customCSS: `
      @keyframes cyberpunk-pulse {
        0%, 100% { box-shadow: 0 0 40px rgba(255,0,255,0.4); }
        50% { box-shadow: 0 0 60px rgba(0,255,255,0.6); }
      }
      .cyberpunk-pulse {
        animation: cyberpunk-pulse 2s infinite;
      }
    `,
  },
  
  minimal: {
    id: 'minimal',
    name: 'Minimal',
    description: 'Clean and distraction-free',
    ascii: {
      enabled: false,
      cellSize: 12,
      characters: '',
      color: '',
      animationSpeed: 0,
      glitchIntensity: 0,
    },
    background: {
      color: 'transparent',
    },
    greeting: {
      show: true,
      fadeDelay: 5000,
      className: 'text-5xl font-extralight text-text-default mb-6',
      position: 'center',
    },
    input: {
      className: '',
      position: 'center',
      width: 'wide',
      rounded: '2xl',
      border: false,
      customStyles: {
        container: 'bg-background-default rounded-3xl shadow-sm',
        textarea: 'text-text-default placeholder:text-text-muted',
      },
    },
    layout: {
      contentAlignment: 'center',
      padding: 'lg',
    },
  },
  
  hacker: {
    id: 'hacker',
    name: 'Hacker',
    description: 'Black and green with glitch effects - transparent input',
    ascii: {
      enabled: true,
      cellSize: 14,
      characters: '$ > - | / \\ [ ] { } < > = + * # @ ! ? & % ^ ~ ` : ; , .',
      color: '#39ff14',
      animationSpeed: 0.002,
      glitchIntensity: 0.05,
    },
    background: {
      color: '#000000',
    },
    greeting: {
      show: true,
      fadeDelay: 3000,
      className: 'text-xl font-mono text-[#39ff14] mb-2 tracking-widest uppercase',
      position: 'top-left',
    },
    input: {
      className: '',
      position: 'bottom-left',
      width: 'full',
      rounded: '2xl',
      border: true,
      borderColor: '#39ff14',
      prompt: 'root@goose:~# ',
      customStyles: {
        // NO BACKGROUND - transparent to show ASCII animation through
        container: 'font-mono bg-transparent backdrop-blur-none border-l-4 border-[#39ff14]',
        textarea: 'text-[#39ff14] font-mono text-sm tracking-wider caret-[#39ff14] placeholder:text-[#39ff14]/20 bg-transparent',
        sendButton: 'bg-[#39ff14] text-black hover:bg-[#39ff14]/80 font-mono font-black uppercase text-xs',
        placeholder: 'text-[#39ff14]/20',
        bottomControls: 'text-[#39ff14]/40 hover:text-[#39ff14]',
      },
    },
    layout: {
      contentAlignment: 'space-between',
      padding: 'sm',
    },
    typography: {
      fontFamily: '"Courier New", "Consolas", monospace',
      fontSize: '13px',
      fontWeight: '600',
      letterSpacing: '0.15em',
    },
    animations: {
      cursorBlink: true,
      cursorStyle: 'block',
      glitchText: true,
      scanlines: true,
    },
    effects: {
      brightness: 1.2,
      contrast: 1.4,
      saturate: 1.5,
    },
    customCSS: `
      @keyframes hacker-glitch {
        0% { transform: translate(0); }
        20% { transform: translate(-2px, 2px); }
        40% { transform: translate(-2px, -2px); }
        60% { transform: translate(2px, 2px); }
        80% { transform: translate(2px, -2px); }
        100% { transform: translate(0); }
      }
      .hacker-glitch {
        animation: hacker-glitch 0.2s infinite;
      }
    `,
  },
  
  retro: {
    id: 'retro',
    name: 'Retro',
    description: 'DOS-style blue and white - full bleed command line',
    ascii: {
      enabled: true,
      cellSize: 10,
      characters: '░▒▓█',
      color: '#ffffff',
      animationSpeed: 0.008,
      glitchIntensity: 0.01,
    },
    background: {
      color: '#0000aa',
    },
    greeting: {
      show: true,
      fadeDelay: 5000,
      className: 'text-3xl font-mono text-white mb-3 uppercase',
      position: 'top-left',
    },
    input: {
      // NO background, NO shadow - pure DOS command line
      className: 'border-t-4 border-white/60',
      position: 'center',
      width: 'wide',
      rounded: '2xl',
      border: true,
      borderColor: '#ffffff',
      prompt: 'C:\\> ',
      customStyles: {
        // NO BACKGROUND - transparent to show DOS blue through
        // FULL BLEED - no padding, edge to edge
        container: 'font-mono bg-transparent backdrop-blur-none border-t-4 border-white !px-0 !py-0',
        textarea: 'text-white font-mono text-base caret-white placeholder:text-white/40 bg-transparent !px-4 !py-3',
        sendButton: 'bg-white text-[#0000aa] hover:bg-white/80 font-mono font-bold uppercase',
        placeholder: 'text-white/40',
        bottomControls: 'text-white/60 hover:text-white',
      },
    },
    layout: {
      contentAlignment: 'space-between',
      padding: 'md',
    },
    typography: {
      fontFamily: '"Perfect DOS VGA 437", "Courier New", monospace',
      fontSize: '16px',
      fontWeight: '400',
    },
    animations: {
      cursorBlink: true,
      cursorStyle: 'underline',
      scanlines: true,
    },
    effects: {
      contrast: 1.1,
    },
    customCSS: `
      @keyframes retro-scanline {
        0% { transform: translateY(0); }
        100% { transform: translateY(100vh); }
      }
      .retro-scanline {
        animation: retro-scanline 8s linear infinite;
      }
    `,
  },
};

export const DEFAULT_THEME_ID = 'default';

// Storage key for theme preference
export const HUB_THEME_STORAGE_KEY = 'goose-hub-theme';
