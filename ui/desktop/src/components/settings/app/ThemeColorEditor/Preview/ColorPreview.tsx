/**
 * ColorPreview Component
 * 
 * Shows live preview of how a selected color variable is used in the UI.
 * Displays real component examples with the color applied.
 */

import { ColorVariable } from '../types';

interface ColorPreviewProps {
  variable: ColorVariable;
  lightColor: string;
  darkColor: string;
  currentMode: 'light' | 'dark';
}

export function ColorPreview({ variable, lightColor, darkColor, currentMode }: ColorPreviewProps) {
  const currentColor = currentMode === 'light' ? lightColor : darkColor;
  
  // Render different previews based on color category
  const renderPreview = () => {
    switch (variable.category) {
      case 'background':
        return <BackgroundPreview variable={variable} color={currentColor} />;
      case 'text':
        return <TextPreview variable={variable} color={currentColor} />;
      case 'border':
        return <BorderPreview variable={variable} color={currentColor} />;
      case 'ring':
        return <RingPreview variable={variable} color={currentColor} />;
      default:
        return null;
    }
  };

  return (
    <div className="space-y-4">
      <div className="space-y-2">
        <h3 className="text-sm font-semibold text-text-primary">
          {variable.label}
        </h3>
        <p className="text-xs text-text-secondary">
          {variable.description}
        </p>
        <div className="flex items-center gap-2">
          <div 
            className="w-12 h-12 rounded border-2 border-border-primary"
            style={{ backgroundColor: currentColor }}
          />
          <div className="text-xs font-mono text-text-secondary">
            {currentColor}
          </div>
        </div>
      </div>
      
      <div className="space-y-3">
        <h4 className="text-xs font-semibold text-text-secondary uppercase tracking-wide">
          Used In:
        </h4>
        {renderPreview()}
      </div>
    </div>
  );
}

// Background color previews
function BackgroundPreview({ variable, color }: { variable: ColorVariable; color: string }) {
  const varName = variable.name;
  
  if (varName === 'color-background-primary') {
    return (
      <div className="space-y-3">
        <PreviewCard title="Main Background">
          <div className="p-4 rounded-lg border border-border-primary" style={{ backgroundColor: color }}>
            <p className="text-text-primary text-sm">
              This is the primary background color used throughout the app.
            </p>
            <p className="text-text-secondary text-xs mt-2">
              It appears in the main chat area, settings panels, and most content areas.
            </p>
          </div>
        </PreviewCard>
        
        <PreviewCard title="Chat Message">
          <div className="flex gap-3">
            <div className="w-8 h-8 rounded-full bg-background-inverse flex items-center justify-center text-text-inverse text-xs">
              AI
            </div>
            <div className="flex-1 p-3 rounded-lg border border-border-primary" style={{ backgroundColor: color }}>
              <p className="text-text-primary text-sm">
                This is how AI messages appear on the primary background.
              </p>
            </div>
          </div>
        </PreviewCard>
      </div>
    );
  }
  
  if (varName === 'color-background-secondary') {
    return (
      <div className="space-y-3">
        <PreviewCard title="Sidebar">
          <div className="flex gap-2 h-32">
            <div className="w-48 rounded-lg border border-border-primary p-3 space-y-2" style={{ backgroundColor: color }}>
              <div className="text-text-primary text-xs font-semibold">Navigation</div>
              <div className="space-y-1">
                <div className="text-text-secondary text-xs px-2 py-1 rounded hover:bg-background-tertiary">
                  Home
                </div>
                <div className="text-text-secondary text-xs px-2 py-1 rounded hover:bg-background-tertiary">
                  Chat
                </div>
                <div className="text-text-secondary text-xs px-2 py-1 rounded hover:bg-background-tertiary">
                  Settings
                </div>
              </div>
            </div>
          </div>
        </PreviewCard>
        
        <PreviewCard title="Card Background">
          <div className="p-4 rounded-lg border border-border-primary" style={{ backgroundColor: color }}>
            <h3 className="text-text-primary text-sm font-semibold mb-2">Settings Card</h3>
            <p className="text-text-secondary text-xs">
              Cards and panels use the secondary background for visual hierarchy.
            </p>
          </div>
        </PreviewCard>
      </div>
    );
  }
  
  if (varName === 'color-background-tertiary') {
    return (
      <PreviewCard title="Nested Elements">
        <div className="p-3 rounded-lg bg-background-secondary border border-border-primary">
          <div className="p-3 rounded border border-border-primary" style={{ backgroundColor: color }}>
            <p className="text-text-primary text-xs">
              Tertiary background for nested panels and hover states.
            </p>
          </div>
        </div>
      </PreviewCard>
    );
  }
  
  if (varName === 'color-background-danger') {
    return (
      <PreviewCard title="Error States">
        <div className="p-3 rounded-lg border border-border-danger" style={{ backgroundColor: color }}>
          <p className="text-text-danger text-sm font-semibold">⚠️ Error</p>
          <p className="text-text-primary text-xs mt-1">
            This background is used for error messages and dangerous actions.
          </p>
        </div>
      </PreviewCard>
    );
  }
  
  if (varName === 'color-background-info') {
    return (
      <PreviewCard title="Info States">
        <div className="p-3 rounded-lg border border-border-info" style={{ backgroundColor: color }}>
          <p className="text-text-info text-sm font-semibold">ℹ️ Information</p>
          <p className="text-text-primary text-xs mt-1">
            This background is used for informational messages and tips.
          </p>
        </div>
      </PreviewCard>
    );
  }
  
  return (
    <PreviewCard title="Background Example">
      <div className="p-4 rounded-lg border border-border-primary" style={{ backgroundColor: color }}>
        <p className="text-text-primary text-sm">Background color preview</p>
      </div>
    </PreviewCard>
  );
}

// Text color previews
function TextPreview({ variable, color }: { variable: ColorVariable; color: string }) {
  const varName = variable.name;
  
  if (varName === 'color-text-primary') {
    return (
      <div className="space-y-3">
        <PreviewCard title="Primary Text">
          <div className="space-y-2">
            <h1 className="text-2xl font-bold" style={{ color }}>Heading Text</h1>
            <p className="text-base" style={{ color }}>
              This is the main text color used for body content, headings, and primary information.
            </p>
            <div className="text-sm" style={{ color }}>
              It appears in chat messages, settings descriptions, and all main content.
            </div>
          </div>
        </PreviewCard>
      </div>
    );
  }
  
  if (varName === 'color-text-secondary') {
    return (
      <PreviewCard title="Secondary Text">
        <div className="space-y-2">
          <p className="text-text-primary text-sm">Main heading</p>
          <p className="text-xs" style={{ color }}>
            Secondary text is used for labels, captions, and less important information.
          </p>
          <div className="flex items-center gap-2">
            <span className="text-text-primary text-xs">Status:</span>
            <span className="text-xs" style={{ color }}>Active</span>
          </div>
        </div>
      </PreviewCard>
    );
  }
  
  if (varName === 'color-text-danger') {
    return (
      <PreviewCard title="Error Text">
        <div className="space-y-2">
          <p className="text-sm font-semibold" style={{ color }}>⚠️ Error Message</p>
          <p className="text-xs" style={{ color }}>
            This color is used for error messages, warnings, and destructive actions.
          </p>
          <button className="px-3 py-1 text-xs rounded border border-border-danger" style={{ color }}>
            Delete
          </button>
        </div>
      </PreviewCard>
    );
  }
  
  if (varName === 'color-text-success') {
    return (
      <PreviewCard title="Success Text">
        <div className="space-y-2">
          <p className="text-sm font-semibold" style={{ color }}>✓ Success</p>
          <p className="text-xs" style={{ color }}>
            Used for success messages, confirmations, and positive feedback.
          </p>
        </div>
      </PreviewCard>
    );
  }
  
  if (varName === 'color-text-warning') {
    return (
      <PreviewCard title="Warning Text">
        <div className="space-y-2">
          <p className="text-sm font-semibold" style={{ color }}>⚡ Warning</p>
          <p className="text-xs" style={{ color }}>
            Used for warnings and caution messages.
          </p>
        </div>
      </PreviewCard>
    );
  }
  
  if (varName === 'color-text-info') {
    return (
      <PreviewCard title="Info Text">
        <div className="space-y-2">
          <p className="text-sm font-semibold" style={{ color }}>ℹ️ Information</p>
          <p className="text-xs" style={{ color }}>
            Used for informational messages and tips.
          </p>
        </div>
      </PreviewCard>
    );
  }
  
  return (
    <PreviewCard title="Text Example">
      <p className="text-sm" style={{ color }}>Sample text in this color</p>
    </PreviewCard>
  );
}

// Border color previews
function BorderPreview({ variable, color }: { variable: ColorVariable; color: string }) {
  const varName = variable.name;
  
  if (varName === 'color-border-primary') {
    return (
      <div className="space-y-3">
        <PreviewCard title="Card Borders">
          <div className="p-4 rounded-lg bg-background-secondary" style={{ borderWidth: '1px', borderStyle: 'solid', borderColor: color }}>
            <p className="text-text-primary text-sm">Card with primary border</p>
          </div>
        </PreviewCard>
        
        <PreviewCard title="Input Borders">
          <input
            type="text"
            placeholder="Type here..."
            className="w-full px-3 py-2 rounded bg-background-primary text-text-primary text-sm"
            style={{ borderWidth: '1px', borderStyle: 'solid', borderColor: color }}
          />
        </PreviewCard>
        
        <PreviewCard title="Dividers">
          <div className="space-y-2">
            <p className="text-text-primary text-xs">Section 1</p>
            <div style={{ height: '1px', backgroundColor: color }} />
            <p className="text-text-primary text-xs">Section 2</p>
          </div>
        </PreviewCard>
      </div>
    );
  }
  
  if (varName === 'color-border-danger') {
    return (
      <PreviewCard title="Error Borders">
        <div className="space-y-2">
          <div className="p-3 rounded-lg bg-background-danger" style={{ borderWidth: '1px', borderStyle: 'solid', borderColor: color }}>
            <p className="text-text-danger text-sm">Error state border</p>
          </div>
          <input
            type="text"
            placeholder="Invalid input..."
            className="w-full px-3 py-2 rounded bg-background-primary text-text-primary text-sm"
            style={{ borderWidth: '2px', borderStyle: 'solid', borderColor: color }}
          />
        </div>
      </PreviewCard>
    );
  }
  
  return (
    <PreviewCard title="Border Example">
      <div className="p-4 rounded-lg bg-background-secondary" style={{ borderWidth: '1px', borderStyle: 'solid', borderColor: color }}>
        <p className="text-text-primary text-sm">Element with this border color</p>
      </div>
    </PreviewCard>
  );
}

// Ring (focus) color previews
function RingPreview({ variable, color }: { variable: ColorVariable; color: string }) {
  return (
    <div className="space-y-3">
      <PreviewCard title="Focus States">
        <div className="space-y-3">
          <button
            className="px-4 py-2 rounded bg-background-secondary text-text-primary text-sm"
            style={{ 
              outline: `2px solid ${color}`,
              outlineOffset: '2px'
            }}
          >
            Focused Button
          </button>
          
          <input
            type="text"
            placeholder="Focused input..."
            className="w-full px-3 py-2 rounded border border-border-primary bg-background-primary text-text-primary text-sm"
            style={{ 
              outline: `2px solid ${color}`,
              outlineOffset: '2px'
            }}
          />
        </div>
      </PreviewCard>
      
      <PreviewCard title="Interactive Elements">
        <p className="text-xs text-text-secondary mb-2">
          The ring color appears when elements receive keyboard focus for accessibility.
        </p>
      </PreviewCard>
    </div>
  );
}

// Helper component for preview cards
function PreviewCard({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="space-y-2">
      <div className="text-xs font-medium text-text-secondary">{title}</div>
      <div className="rounded-lg bg-background-primary border border-border-primary p-3">
        {children}
      </div>
    </div>
  );
}
