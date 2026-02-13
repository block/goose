/**
 * ColorPreview Component
 * 
 * Shows 1:1 accurate previews of how a selected color variable is used in the actual Goose UI.
 * Uses real component structures and class names from the app.
 */

import { ColorVariable } from '../types';
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '../../../../ui/card';
import { Button } from '../../../../ui/button';
import { Home, MessageSquarePlus, FileText, AppWindow, Clock, Puzzle } from 'lucide-react';
import { Gear } from '../../../../icons';

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
    <div className="h-full flex items-center justify-center p-8">
      <div className="w-full max-w-2xl space-y-6">
        {/* Color Info Header */}
        <div className="space-y-3">
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
        </div>
        
        <div className="h-px bg-border-primary" />
        
        {/* Usage Examples */}
        <div className="space-y-4">
          <h4 className="text-sm font-semibold text-text-primary">
            Where This Color Appears:
          </h4>
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
      <div className="space-y-4">
        <ExampleSection title="Main App Background">
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
        </ExampleSection>
        
        <ExampleSection title="Goose AI Message (Exact Replica)">
          {/* Exact replica of GooseMessage component */}
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
        </ExampleSection>
      </div>
    );
  }
  
  if (varName === 'color-background-secondary') {
    return (
      <div className="space-y-4">
        <ExampleSection title="Settings Card (Exact Replica)">
          {/* Using actual Card component from ui/card.tsx */}
          <Card className="rounded-lg" style={{ backgroundColor: color }}>
            <CardHeader className="pb-0">
              <CardTitle>Appearance</CardTitle>
              <CardDescription>Configure how goose appears on your system</CardDescription>
            </CardHeader>
            <CardContent className="pt-4 space-y-4 px-4">
              <div className="flex items-center justify-between">
                <div>
                  <h3 className="text-text-primary text-xs">Menu bar icon</h3>
                  <p className="text-xs text-text-secondary max-w-md mt-[2px]">
                    Show goose in the menu bar
                  </p>
                </div>
              </div>
            </CardContent>
          </Card>
        </ExampleSection>
        
        <ExampleSection title="Sidebar Navigation (Exact Replica)">
          {/* Exact replica of AppSidebar menu structure */}
          <div className="w-64 rounded-lg border border-border-primary overflow-hidden" style={{ backgroundColor: color }}>
            <div className="p-3 space-y-1">
              {/* Home button - active state */}
              <div className="w-full justify-start px-3 py-2 rounded-lg h-fit bg-background-tertiary transition-all duration-200 flex items-center gap-2">
                <Home className="w-4 h-4 text-text-primary" />
                <span className="text-text-primary text-sm">Home</span>
              </div>
              
              {/* Chat button */}
              <div className="w-full justify-start px-3 py-2 rounded-lg h-fit hover:bg-background-tertiary/50 transition-all duration-200 flex items-center gap-2">
                <MessageSquarePlus className="w-4 h-4 text-text-secondary" />
                <span className="text-text-secondary text-sm">Chat</span>
              </div>
              
              {/* Divider */}
              <div className="h-px bg-border-primary my-2" />
              
              {/* Other menu items */}
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
              
              {/* Divider */}
              <div className="h-px bg-border-primary my-2" />
              
              {/* Settings */}
              <div className="w-full justify-start px-3 py-2 rounded-lg h-fit hover:bg-background-tertiary/50 transition-all duration-200 flex items-center gap-2">
                <Gear className="w-4 h-4 text-text-secondary" />
                <span className="text-text-secondary text-sm">Settings</span>
              </div>
            </div>
          </div>
        </ExampleSection>
      </div>
    );
  }
  
  if (varName === 'color-background-tertiary') {
    return (
      <ExampleSection title="Hover States & Nested Elements">
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
      </ExampleSection>
    );
  }
  
  if (varName === 'color-background-inverse') {
    return (
      <div className="space-y-4">
        <ExampleSection title="Primary Buttons (Exact Replica)">
          <Button variant="default" className="w-full" style={{ backgroundColor: color }}>
            Primary Action Button
          </Button>
        </ExampleSection>
        
        <ExampleSection title="Selected States">
          <div className="p-3 rounded-lg text-text-inverse text-sm" style={{ backgroundColor: color }}>
            Selected item or active state
          </div>
        </ExampleSection>
      </div>
    );
  }
  
  if (varName === 'color-background-danger') {
    return (
      <ExampleSection title="Error/Danger States">
        <div className="space-y-3">
          <div className="p-3 rounded-lg border border-border-danger" style={{ backgroundColor: color }}>
            <p className="text-text-danger text-sm font-semibold">⚠️ Error Message</p>
            <p className="text-text-primary text-xs mt-1">
              Failed to load configuration file
            </p>
          </div>
          <Button variant="destructive" className="w-full" style={{ backgroundColor: color }}>
            Delete Session
          </Button>
        </div>
      </ExampleSection>
    );
  }
  
  if (varName === 'color-background-info') {
    return (
      <ExampleSection title="Info/Help States">
        <div className="p-3 rounded-lg border border-border-info" style={{ backgroundColor: color }}>
          <p className="text-text-info text-sm font-semibold">ℹ️ Tip</p>
          <p className="text-text-primary text-xs mt-1">
            You can use keyboard shortcuts to navigate faster
          </p>
        </div>
      </ExampleSection>
    );
  }
  
  return (
    <ExampleSection title="Background Example">
      <div className="p-4 rounded-lg border border-border-primary" style={{ backgroundColor: color }}>
        <p className="text-text-primary text-sm">Background color preview</p>
      </div>
    </ExampleSection>
  );
}

// Text color previews - Using REAL Goose UI patterns
function TextPreview({ variable, color }: { variable: ColorVariable; color: string }) {
  const varName = variable.name;
  
  if (varName === 'color-text-primary') {
    return (
      <div className="space-y-4">
        <ExampleSection title="Chat Message Text (Exact Replica)">
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
        </ExampleSection>
        
        <ExampleSection title="Card Title Text">
          <Card className="rounded-lg">
            <CardHeader className="pb-0">
              <CardTitle style={{ color }}>Settings Section</CardTitle>
              <CardDescription>Primary text appears in headings and main content</CardDescription>
            </CardHeader>
          </Card>
        </ExampleSection>
      </div>
    );
  }
  
  if (varName === 'color-text-secondary') {
    return (
      <ExampleSection title="Labels & Descriptions (Exact Replica)">
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
      </ExampleSection>
    );
  }
  
  if (varName === 'color-text-inverse') {
    return (
      <ExampleSection title="Inverse Text (On Dark Backgrounds)">
        <div className="space-y-3">
          <Button variant="default" className="w-full">
            <span style={{ color }}>Button Text</span>
          </Button>
          <div className="p-3 rounded-lg bg-background-inverse">
            <p className="text-sm" style={{ color }}>
              Text on dark/inverse backgrounds
            </p>
          </div>
        </div>
      </ExampleSection>
    );
  }
  
  if (varName === 'color-text-danger') {
    return (
      <ExampleSection title="Error Text (Exact Replica)">
        <div className="space-y-3">
          <div className="p-3 rounded-lg border border-border-danger bg-background-danger">
            <p className="text-sm font-semibold" style={{ color }}>⚠️ Error</p>
            <p className="text-xs mt-1" style={{ color }}>
              Failed to load configuration file
            </p>
          </div>
          <Button variant="destructive">
            <span style={{ color }}>Delete Session</span>
          </Button>
        </div>
      </ExampleSection>
    );
  }
  
  if (varName === 'color-text-success') {
    return (
      <ExampleSection title="Success Messages">
        <div className="p-3 rounded-lg border border-border-primary bg-background-primary">
          <p className="text-sm font-semibold" style={{ color }}>✓ Success</p>
          <p className="text-xs mt-1" style={{ color }}>
            Theme saved successfully!
          </p>
        </div>
      </ExampleSection>
    );
  }
  
  if (varName === 'color-text-warning') {
    return (
      <ExampleSection title="Warning Messages">
        <div className="p-3 rounded-lg border border-border-primary bg-background-primary">
          <p className="text-sm font-semibold" style={{ color }}>⚡ Warning</p>
          <p className="text-xs mt-1" style={{ color }}>
            This action cannot be undone
          </p>
        </div>
      </ExampleSection>
    );
  }
  
  if (varName === 'color-text-info') {
    return (
      <ExampleSection title="Info Messages">
        <div className="p-3 rounded-lg border border-border-info bg-background-info">
          <p className="text-sm font-semibold" style={{ color }}>ℹ️ Tip</p>
          <p className="text-xs mt-1" style={{ color }}>
            You can use keyboard shortcuts to navigate faster
          </p>
        </div>
      </ExampleSection>
    );
  }
  
  return (
    <ExampleSection title="Text Example">
      <p className="text-sm" style={{ color }}>Sample text in this color</p>
    </ExampleSection>
  );
}

// Border color previews - Using REAL Goose UI patterns
function BorderPreview({ variable, color }: { variable: ColorVariable; color: string }) {
  const varName = variable.name;
  
  if (varName === 'color-border-primary') {
    return (
      <div className="space-y-4">
        <ExampleSection title="Settings Card Border (Exact Replica)">
          <Card className="rounded-lg" style={{ borderColor: color }}>
            <CardHeader className="pb-0">
              <CardTitle>Card Title</CardTitle>
              <CardDescription>This card uses the primary border color</CardDescription>
            </CardHeader>
            <CardContent className="pt-4 px-4">
              <p className="text-text-primary text-xs">Card content goes here</p>
            </CardContent>
          </Card>
        </ExampleSection>
        
        <ExampleSection title="Input Border (Exact Replica)">
          <input
            type="text"
            placeholder="Type a message..."
            className="w-full outline-none border focus:ring-0 bg-background-primary px-3 py-2 text-sm resize-none text-text-primary placeholder:text-text-secondary rounded"
            style={{ borderColor: color }}
          />
        </ExampleSection>
        
        <ExampleSection title="Divider Lines">
          <div className="space-y-3">
            <p className="text-text-primary text-xs">Section Above</p>
            <div className="h-px" style={{ backgroundColor: color }} />
            <p className="text-text-primary text-xs">Section Below</p>
          </div>
        </ExampleSection>
      </div>
    );
  }
  
  if (varName === 'color-border-secondary') {
    return (
      <ExampleSection title="Hover & Focus Borders">
        <div className="space-y-3">
          <div className="p-4 rounded-lg bg-background-secondary border transition-colors hover:border-border-secondary" style={{ borderColor: color }}>
            <p className="text-text-primary text-sm">Hovered card border</p>
          </div>
        </div>
      </ExampleSection>
    );
  }
  
  if (varName === 'color-border-danger') {
    return (
      <ExampleSection title="Error Borders (Exact Replica)">
        <div className="space-y-3">
          <div className="p-3 rounded-lg bg-background-danger" style={{ borderWidth: '1px', borderStyle: 'solid', borderColor: color }}>
            <p className="text-text-danger text-sm font-semibold">Error State</p>
          </div>
          <input
            type="text"
            placeholder="Invalid input..."
            className="w-full px-3 py-2 rounded bg-background-primary text-text-primary text-sm"
            style={{ borderWidth: '2px', borderStyle: 'solid', borderColor: color }}
          />
        </div>
      </ExampleSection>
    );
  }
  
  if (varName === 'color-border-info') {
    return (
      <ExampleSection title="Info Borders">
        <div className="p-3 rounded-lg bg-background-info" style={{ borderWidth: '1px', borderStyle: 'solid', borderColor: color }}>
          <p className="text-text-info text-sm font-semibold">Info State</p>
        </div>
      </ExampleSection>
    );
  }
  
  return (
    <ExampleSection title="Border Example">
      <div className="p-4 rounded-lg bg-background-secondary" style={{ borderWidth: '1px', borderStyle: 'solid', borderColor: color }}>
        <p className="text-text-primary text-sm">Element with this border color</p>
      </div>
    </ExampleSection>
  );
}

// Ring (focus) color previews - Using REAL button focus styles
function RingPreview({ variable, color }: { variable: ColorVariable; color: string }) {
  return (
    <div className="space-y-4">
      <ExampleSection title="Button Focus Ring (Exact Replica)">
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
      </ExampleSection>
      
      <ExampleSection title="Input Focus Ring (Exact Replica)">
        <input
          type="text"
          placeholder="Type here..."
          className="w-full px-3 py-2 rounded border border-border-primary bg-background-primary text-text-primary text-sm"
          style={{ 
            outline: `2px solid ${color}`,
            outlineOffset: '2px'
          }}
        />
      </ExampleSection>
      
      <ExampleSection title="Accessibility Note">
        <p className="text-xs text-text-secondary">
          The ring color appears when elements receive keyboard focus (Tab key navigation). 
          This is essential for accessibility and keyboard navigation.
        </p>
      </ExampleSection>
    </div>
  );
}

// Helper components
function ExampleSection({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="space-y-3">
      <div className="flex items-center gap-2">
        <div className="text-sm font-semibold text-text-primary">{title}</div>
        <div className="text-xs text-text-secondary bg-background-secondary px-2 py-0.5 rounded">
          1:1 Replica
        </div>
      </div>
      <div className="rounded-lg bg-background-secondary/50 border border-border-primary p-4">
        {children}
      </div>
    </div>
  );
}

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
