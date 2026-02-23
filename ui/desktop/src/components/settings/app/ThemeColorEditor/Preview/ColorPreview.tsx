/**
 * ColorPreview Component
 * 
 * Shows 1:1 accurate previews of how a selected color variable is used in the actual Goose UI.
 * Uses real component structures and class names from the app.
 */

import { ColorVariable } from '../types';
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '../../../../ui/card';
import { Button } from '../../../../ui/button';
import { Home, MessageSquarePlus, FileText, AppWindow, Clock, Puzzle, Save, AlertTriangle, RotateCcw } from 'lucide-react';
import { Gear } from '../../../../icons';

interface ColorPreviewProps {
  variable: ColorVariable;
  lightColor: string;
  darkColor: string;
  currentMode: 'light' | 'dark';
  allColors: Record<string, string>; // All current theme colors for the active mode
}

export function ColorPreview({ variable, lightColor, darkColor, currentMode, allColors }: ColorPreviewProps) {
  const currentColor = currentMode === 'light' ? lightColor : darkColor;
  
  // Create inline styles for all theme colors to override CSS variables
  const themeStyles = Object.entries(allColors).reduce<Record<string, string>>((acc, [key, value]) => {
    acc[`--${key}`] = value;
    return acc;
  }, {});
  
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
    <div className="h-full flex flex-col p-8">
      {/* Color Info Header - Top Aligned */}
      <div className="flex-shrink-0 space-y-3">
        <div className="flex items-center gap-3">
          <div 
            className="w-16 h-16 rounded-lg border-2 border-border-primary shadow-sm"
            style={{ backgroundColor: currentColor }}
          />
          <div className="flex-1">
            <h3 className="text-base font-semibold text-text-primary">
              {variable.label}
            </h3>
            <p className="text-sm text-text-secondary mt-1">
              {variable.description}
            </p>
            <div className="text-xs font-mono text-text-secondary mt-2 bg-background-secondary px-2 py-1 rounded inline-block">
              {currentColor}
            </div>
          </div>
        </div>
        <div className="h-px bg-border-primary" />
      </div>
      
      {/* Usage Examples - Centered Vertically with Live Theme Colors */}
      <div className="flex-1 flex items-center justify-center py-8">
        <div className="w-full max-w-2xl space-y-6" style={themeStyles}>
          {renderPreview()}
        </div>
      </div>
    </div>
  );
}

// Background color previews - Using REAL Goose UI components
function BackgroundPreview({ variable, color }: { variable: ColorVariable; color: string }) {
  const varName = variable.name;
  
  if (varName === 'color-background-primary') {
    return (
      <>
        {/* Main app background preview */}
        <div className="w-full h-48 rounded-lg border border-border-primary" style={{ backgroundColor: color }}>
          <div className="p-4 space-y-3">
            <p className="text-text-primary text-sm">
              This is the main background color for the entire app.
            </p>
            <p className="text-text-secondary text-xs">
              Used in: Chat area, main content, message list
            </p>
          </div>
        </div>
        
        {/* Goose AI Message */}
        <div className="goose-message flex w-full justify-start min-w-0" style={{ backgroundColor: color }}>
          <div className="flex flex-col w-full min-w-0 p-4">
            <div className="flex flex-col group">
              <div className="w-full">
                <p className="text-text-primary text-sm">
                  I'll help you with that! Let me check the files in your project.
                </p>
              </div>
              <div className="relative flex justify-start">
                <div className="text-xs font-mono text-text-secondary pt-1">
                  2:45 PM
                </div>
              </div>
            </div>
          </div>
        </div>
      </>
    );
  }
  
  if (varName === 'color-background-secondary') {
    return (
      <div className="w-64 mx-auto rounded-lg border border-border-primary overflow-hidden" style={{ backgroundColor: color }}>
        <div className="p-3 space-y-1">
          <div className="w-full justify-start px-3 py-2 rounded-lg h-fit bg-background-tertiary transition-all duration-200 flex items-center gap-2">
            <Home className="w-4 h-4 text-text-primary" />
            <span className="text-text-primary text-sm">Home</span>
          </div>
          <div className="w-full justify-start px-3 py-2 rounded-lg h-fit hover:bg-background-tertiary/50 transition-all duration-200 flex items-center gap-2">
            <MessageSquarePlus className="w-4 h-4 text-text-secondary" />
            <span className="text-text-secondary text-sm">Chat</span>
          </div>
          <div className="h-px bg-border-primary my-2" />
          <div className="w-full justify-start px-3 py-2 rounded-lg h-fit hover:bg-background-tertiary/50 transition-all duration-200 flex items-center gap-2">
            <FileText className="w-4 h-4 text-text-secondary" />
            <span className="text-text-secondary text-sm">Recipes</span>
          </div>
          <div className="w-full justify-start px-3 py-2 rounded-lg h-fit hover:bg-background-tertiary/50 transition-all duration-200 flex items-center gap-2">
            <AppWindow className="w-4 h-4 text-text-secondary" />
            <span className="text-text-secondary text-sm">Apps</span>
          </div>
          <div className="w-full justify-start px-3 py-2 rounded-lg h-fit hover:bg-background-tertiary/50 transition-all duration-200 flex items-center gap-2">
            <Clock className="w-4 h-4 text-text-secondary" />
            <span className="text-text-secondary text-sm">Scheduler</span>
          </div>
          <div className="w-full justify-start px-3 py-2 rounded-lg h-fit hover:bg-background-tertiary/50 transition-all duration-200 flex items-center gap-2">
            <Puzzle className="w-4 h-4 text-text-secondary" />
            <span className="text-text-secondary text-sm">Extensions</span>
          </div>
          <div className="h-px bg-border-primary my-2" />
          <div className="w-full justify-start px-3 py-2 rounded-lg h-fit hover:bg-background-tertiary/50 transition-all duration-200 flex items-center gap-2">
            <Gear className="w-4 h-4 text-text-secondary" />
            <span className="text-text-secondary text-sm">Settings</span>
          </div>
        </div>
      </div>
    );
  }
  
  if (varName === 'color-background-tertiary') {
    return (
      <div className="p-4 rounded-lg bg-background-secondary border border-border-primary">
        <p className="text-text-primary text-xs mb-3">Hover over items:</p>
        <div className="space-y-1">
          <div className="px-3 py-2 rounded-md text-text-primary text-xs cursor-pointer" style={{ backgroundColor: color }}>
            Hovered item (tertiary background)
          </div>
          <div className="px-3 py-2 rounded-md text-text-secondary text-xs hover:bg-background-tertiary cursor-pointer">
            Normal item
          </div>
        </div>
      </div>
    );
  }
  
  if (varName === 'color-background-inverse') {
    return (
      <>
        {/* Primary Action Button with Icon (Exact Replica) */}
        <Button variant="default" className="w-full flex items-center justify-center gap-2" style={{ backgroundColor: color }}>
          <Save className="w-4 h-4 text-text-inverse" />
          <span className="text-text-inverse">Save Theme</span>
        </Button>
        
        {/* User Chat Bubble (Exact Replica from UserMessage.tsx) */}
        <div className="message flex justify-end w-full">
          <div className="flex-col max-w-[85%] w-fit">
            <div className="flex flex-col group">
              <div className="flex rounded-xl py-2.5 px-4" style={{ backgroundColor: color }}>
                <div>
                  <p className="text-sm text-text-inverse">
                    Can you help me customize the theme colors?
                  </p>
                </div>
              </div>
            </div>
          </div>
        </div>
      </>
    );
  }
  
  if (varName === 'color-background-danger') {
    return (
      <Button variant="destructive" className="w-full" style={{ backgroundColor: color }}>
        Delete Session
      </Button>
    );
  }
  
  if (varName === 'color-background-info') {
    return (
      <Card className="pb-2 rounded-lg border-border-info" style={{ backgroundColor: color }}>
        <CardHeader className="pb-2">
          <div className="flex items-start gap-3">
            <AlertTriangle className="h-5 w-5 text-text-info flex-shrink-0 mt-1" />
            <div className="flex-1">
              <CardTitle className="text-text-info text-base">Prompt Editing</CardTitle>
              <p className="text-sm text-text-secondary mt-2">
                Customize the prompts that define goose's behavior in different contexts. These
                prompts use Jinja2 templating syntax.
              </p>
            </div>
            <Button
              variant="outline"
              size="sm"
              className="flex items-center gap-2 border-border-info hover:bg-background-info"
            >
              <RotateCcw className="h-4 w-4" />
              Reset All
            </Button>
          </div>
        </CardHeader>
      </Card>
    );
  }
  
  return (
    <div className="p-4 rounded-lg border border-border-primary" style={{ backgroundColor: color }}>
      <p className="text-text-primary text-sm">Background color preview</p>
    </div>
  );
}

// Text color previews - Using REAL Goose UI patterns
function TextPreview({ variable, color }: { variable: ColorVariable; color: string }) {
  const varName = variable.name;
  
  if (varName === 'color-text-primary') {
    return (
      <>
        <div className="goose-message flex w-full justify-start min-w-0">
          <div className="flex flex-col w-full min-w-0 p-4 bg-background-primary rounded-lg">
            <div className="flex flex-col group">
              <div className="w-full">
                <p className="text-sm" style={{ color }}>
                  I'll help you with that! Let me check the files in your project and make the necessary changes.
                </p>
              </div>
            </div>
          </div>
        </div>
        
        <Card className="rounded-lg">
          <CardHeader className="pb-0">
            <CardTitle style={{ color }}>Settings Section</CardTitle>
            <CardDescription>Primary text appears in headings and main content</CardDescription>
          </CardHeader>
        </Card>
      </>
    );
  }
  
  if (varName === 'color-text-secondary') {
    return (
      <Card className="rounded-lg">
        <CardHeader className="pb-0">
          <CardTitle>Menu bar icon</CardTitle>
          <CardDescription style={{ color }}>
            Show goose in the menu bar
          </CardDescription>
        </CardHeader>
        <CardContent className="pt-4 px-4">
          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-text-primary text-xs">Setting Name</h3>
              <p className="text-xs mt-[2px]" style={{ color }}>
                This is secondary text used for descriptions and labels
              </p>
            </div>
          </div>
        </CardContent>
      </Card>
    );
  }
  
  if (varName === 'color-text-inverse') {
    return (
      <>
        <Button variant="default" className="w-full">
          <span style={{ color }}>Button Text</span>
        </Button>
        <div className="p-3 rounded-lg bg-background-inverse">
          <p className="text-sm" style={{ color }}>
            Text on dark/inverse backgrounds
          </p>
        </div>
      </>
    );
  }
  
  if (varName === 'color-text-danger') {
    return (
      <>
        <div className="p-3 rounded-lg border border-border-danger bg-background-danger">
          <p className="text-sm font-semibold" style={{ color }}>⚠️ Error</p>
          <p className="text-xs mt-1" style={{ color }}>
            Failed to load configuration file
          </p>
        </div>
        <Button variant="destructive">
          <span style={{ color }}>Delete Session</span>
        </Button>
      </>
    );
  }
  
  if (varName === 'color-text-success') {
    return (
      <div className="p-3 rounded-lg border border-border-primary bg-background-primary">
        <p className="text-sm font-semibold" style={{ color }}>✓ Success</p>
        <p className="text-xs mt-1" style={{ color }}>
          Theme saved successfully!
        </p>
      </div>
    );
  }
  
  if (varName === 'color-text-warning') {
    return (
      <div className="p-3 rounded-lg border border-border-primary bg-background-primary">
        <p className="text-sm font-semibold" style={{ color }}>⚡ Warning</p>
        <p className="text-xs mt-1" style={{ color }}>
          This action cannot be undone
        </p>
      </div>
    );
  }
  
  if (varName === 'color-text-info') {
    return (
      <>
        <div className="p-3 rounded-lg border border-border-primary bg-background-secondary">
          <p className="text-sm font-semibold" style={{ color }}>ℹ️ Information</p>
          <p className="text-xs mt-1 text-text-secondary">
            Info text is used for helpful tips, notifications, and informational messages
          </p>
        </div>
        
        <div className="flex items-start gap-2 p-3 rounded-lg bg-background-primary border border-border-primary">
          <div className="w-1 h-full rounded-full" style={{ backgroundColor: color }} />
          <div>
            <p className="text-xs font-semibold" style={{ color }}>Pro Tip</p>
            <p className="text-xs text-text-secondary mt-1">
              You can use keyboard shortcuts to navigate faster through the app
            </p>
          </div>
        </div>
      </>
    );
  }
  
  return (
    <p className="text-sm" style={{ color }}>Sample text in this color</p>
  );
}

// Border color previews - Using REAL Goose UI patterns
function BorderPreview({ variable, color }: { variable: ColorVariable; color: string }) {
  const varName = variable.name;
  
  if (varName === 'color-border-primary') {
    return (
      <>
        <Card className="rounded-lg" style={{ borderColor: color }}>
          <CardHeader className="pb-0">
            <CardTitle>Card Title</CardTitle>
            <CardDescription>This card uses the primary border color</CardDescription>
          </CardHeader>
          <CardContent className="pt-4 px-4">
            <p className="text-text-primary text-xs">Card content goes here</p>
          </CardContent>
        </Card>
        
        <input
          type="text"
          placeholder="Type a message..."
          className="w-full outline-none border focus:ring-0 bg-background-primary px-3 py-2 text-sm resize-none text-text-primary placeholder:text-text-secondary rounded"
          style={{ borderColor: color }}
        />
        
        <div className="space-y-3">
          <p className="text-text-primary text-xs">Section Above</p>
          <div className="h-px" style={{ backgroundColor: color }} />
          <p className="text-text-primary text-xs">Section Below</p>
        </div>
      </>
    );
  }
  
  if (varName === 'color-border-secondary') {
    return (
      <div className="p-4 rounded-lg bg-background-secondary border transition-colors hover:border-border-secondary" style={{ borderColor: color }}>
        <p className="text-text-primary text-sm">Hovered card border</p>
      </div>
    );
  }
  
  if (varName === 'color-border-danger') {
    return (
      <>
        <div className="p-3 rounded-lg bg-background-danger" style={{ borderWidth: '1px', borderStyle: 'solid', borderColor: color }}>
          <p className="text-text-danger text-sm font-semibold">Error State</p>
        </div>
        <input
          type="text"
          placeholder="Invalid input..."
          className="w-full px-3 py-2 rounded bg-background-primary text-text-primary text-sm"
          style={{ borderWidth: '2px', borderStyle: 'solid', borderColor: color }}
        />
      </>
    );
  }
  
  if (varName === 'color-border-info') {
    return (
      <div className="p-3 rounded-lg bg-background-info" style={{ borderWidth: '1px', borderStyle: 'solid', borderColor: color }}>
        <p className="text-text-info text-sm font-semibold">Info State</p>
      </div>
    );
  }
  
  return (
    <div className="p-4 rounded-lg bg-background-secondary" style={{ borderWidth: '1px', borderStyle: 'solid', borderColor: color }}>
      <p className="text-text-primary text-sm">Element with this border color</p>
    </div>
  );
}

// Ring (focus) color previews - Using REAL button focus styles
function RingPreview({ color }: { variable: ColorVariable; color: string }) {
  return (
    <>
      <Button 
        variant="outline"
        className="w-full"
        style={{ 
          outline: `2px solid ${color}`,
          outlineOffset: '2px'
        }}
      >
        Focused Button
      </Button>
      
      <input
        type="text"
        placeholder="Type here..."
        className="w-full px-3 py-2 rounded border border-border-primary bg-background-primary text-text-primary text-sm"
        style={{ 
          outline: `2px solid ${color}`,
          outlineOffset: '2px'
        }}
      />
      
      <p className="text-xs text-text-secondary">
        The ring color appears when elements receive keyboard focus (Tab key navigation). 
        This is essential for accessibility and keyboard navigation.
      </p>
    </>
  );
}


