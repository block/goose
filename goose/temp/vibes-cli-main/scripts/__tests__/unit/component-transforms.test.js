/**
 * Tests for component transformation utilities
 *
 * These tests ensure the build-components.js logic correctly:
 * - Removes import statements
 * - Prefixes React hooks
 * - Namespaces conflicting function names
 */

import { describe, it, expect } from 'vitest';
import {
  removeImports,
  prefixReactHooks,
  namespaceVibesButtonFunctions,
  namespaceCollidingFunctions,
  transformComponent
} from '../../lib/component-transforms.js';

describe('removeImports', () => {
  describe('named imports', () => {
    it('removes named imports from react', () => {
      const input = `import { useState, useEffect } from "react";
const [count, setCount] = useState(0);`;
      const result = removeImports(input);
      expect(result).not.toContain('import');
      expect(result).toContain('useState(0)');
    });

    it('removes named imports with single quotes', () => {
      const input = `import { useState } from 'react';
useState(0);`;
      const result = removeImports(input);
      expect(result).not.toContain('import');
    });
  });

  describe('default imports', () => {
    it('removes default imports', () => {
      const input = `import React from "react";
React.createElement('div');`;
      const result = removeImports(input);
      expect(result).not.toContain('import');
      expect(result).toContain("React.createElement");
    });
  });

  describe('mixed imports', () => {
    it('removes default + named imports', () => {
      const input = `import React, { useState } from "react";
const [x] = useState();`;
      const result = removeImports(input);
      expect(result).not.toContain('import');
    });
  });

  describe('type imports', () => {
    it('removes type imports', () => {
      const input = `import type { FC } from "react";
const Component: FC = () => null;`;
      const result = removeImports(input);
      expect(result).not.toContain('import type');
    });
  });

  describe('export removal', () => {
    it('removes export keyword from declarations', () => {
      const input = `export function MyComponent() {}
export const value = 42;`;
      const result = removeImports(input);
      expect(result).not.toMatch(/^export\s/m);
      expect(result).toContain('function MyComponent');
      expect(result).toContain('const value');
    });
  });

  describe('preserves content', () => {
    it('preserves non-import code', () => {
      const input = `import { useState } from "react";

function Component() {
  const [state, setState] = useState(0);
  return state;
}`;
      const result = removeImports(input);
      expect(result).toContain('function Component()');
      expect(result).toContain('return state');
    });
  });
});

describe('prefixReactHooks', () => {
  describe('useState', () => {
    it('prefixes useState with React.', () => {
      const input = 'const [x, setX] = useState(0);';
      const result = prefixReactHooks(input);
      expect(result).toBe('const [x, setX] = React.useState(0);');
    });

    it('does not double-prefix React.useState', () => {
      const input = 'const [x] = React.useState(0);';
      const result = prefixReactHooks(input);
      expect(result).toBe('const [x] = React.useState(0);');
    });
  });

  describe('useEffect', () => {
    it('prefixes useEffect with React.', () => {
      const input = 'useEffect(() => {}, []);';
      const result = prefixReactHooks(input);
      expect(result).toBe('React.useEffect(() => {}, []);');
    });
  });

  describe('useRef', () => {
    it('prefixes useRef with React.', () => {
      const input = 'const ref = useRef(null);';
      const result = prefixReactHooks(input);
      expect(result).toBe('const ref = React.useRef(null);');
    });
  });

  describe('useCallback', () => {
    it('prefixes useCallback with React.', () => {
      const input = 'const fn = useCallback(() => {}, []);';
      const result = prefixReactHooks(input);
      expect(result).toBe('const fn = React.useCallback(() => {}, []);');
    });
  });

  describe('useMemo', () => {
    it('prefixes useMemo with React.', () => {
      const input = 'const val = useMemo(() => x * 2, [x]);';
      const result = prefixReactHooks(input);
      expect(result).toBe('const val = React.useMemo(() => x * 2, [x]);');
    });
  });

  describe('useId', () => {
    it('prefixes useId with React.', () => {
      const input = 'const id = useId();';
      const result = prefixReactHooks(input);
      expect(result).toBe('const id = React.useId();');
    });
  });

  describe('useLayoutEffect', () => {
    it('prefixes useLayoutEffect with React.', () => {
      const input = 'useLayoutEffect(() => {});';
      const result = prefixReactHooks(input);
      expect(result).toBe('React.useLayoutEffect(() => {});');
    });
  });

  describe('forwardRef', () => {
    it('prefixes forwardRef with React.', () => {
      const input = 'const Comp = forwardRef((props, ref) => null);';
      const result = prefixReactHooks(input);
      expect(result).toBe('const Comp = React.forwardRef((props, ref) => null);');
    });
  });

  describe('multiple hooks', () => {
    it('prefixes multiple hooks in same code', () => {
      const input = `const [x] = useState(0);
useEffect(() => {}, [x]);
const ref = useRef(null);`;
      const result = prefixReactHooks(input);
      expect(result).toContain('React.useState');
      expect(result).toContain('React.useEffect');
      expect(result).toContain('React.useRef');
    });
  });
});

describe('namespaceVibesButtonFunctions', () => {
  describe('VibesButton.styles', () => {
    it('renames getContentWrapperStyle for VibesButton.styles', () => {
      const input = 'function getContentWrapperStyle(a, b) { return {}; }';
      const result = namespaceVibesButtonFunctions(input, 'VibesButton.styles');
      expect(result).toContain('getVibesButtonContentWrapperStyle');
      expect(result).not.toMatch(/\bgetContentWrapperStyle\b/);
    });
  });

  describe('VibesButton', () => {
    it('renames getContentWrapperStyle for VibesButton', () => {
      const input = 'const style = getContentWrapperStyle(isMobile, hasIcon);';
      const result = namespaceVibesButtonFunctions(input, 'VibesButton');
      expect(result).toContain('getVibesButtonContentWrapperStyle');
    });
  });

  describe('other components', () => {
    it('does not rename for HiddenMenuWrapper', () => {
      const input = 'function getContentWrapperStyle(height) { return {}; }';
      const result = namespaceVibesButtonFunctions(input, 'HiddenMenuWrapper');
      expect(result).toContain('getContentWrapperStyle');
      expect(result).not.toContain('getVibesButtonContentWrapperStyle');
    });

    it('does not rename for arbitrary components', () => {
      const input = 'getContentWrapperStyle()';
      const result = namespaceVibesButtonFunctions(input, 'SomeOtherComponent');
      expect(result).toBe('getContentWrapperStyle()');
    });
  });
});

describe('namespaceCollidingFunctions', () => {
  const collidingFunctions = [
    'getContainerStyle',
    'getLabelStyle',
    'getButtonWrapperStyle',
    'getResponsiveLabelStyle',
    'getResponsiveButtonWrapperStyle',
    'getResponsiveContainerStyle'
  ];

  describe('LabelContainer.styles', () => {
    it('namespaces all colliding functions', () => {
      const input = collidingFunctions.map(fn => `function ${fn}() {}`).join('\n');
      const result = namespaceCollidingFunctions(input, 'LabelContainer.styles');

      for (const fn of collidingFunctions) {
        const prefixed = fn.replace(/^get/, 'getLabelContainer');
        expect(result).toContain(prefixed);
        expect(result).not.toMatch(new RegExp(`\\b${fn}\\b`));
      }
    });

    it('namespaces function calls too', () => {
      const input = 'const style = getContainerStyle();';
      const result = namespaceCollidingFunctions(input, 'LabelContainer.styles');
      expect(result).toBe('const style = getLabelContainerContainerStyle();');
    });
  });

  describe('LabelContainer', () => {
    it('namespaces colliding functions', () => {
      const input = 'return getContainerStyle();';
      const result = namespaceCollidingFunctions(input, 'LabelContainer');
      expect(result).toBe('return getLabelContainerContainerStyle();');
    });
  });

  describe('VibesPanel.styles', () => {
    it('namespaces all colliding functions', () => {
      const input = collidingFunctions.map(fn => `function ${fn}() {}`).join('\n');
      const result = namespaceCollidingFunctions(input, 'VibesPanel.styles');

      for (const fn of collidingFunctions) {
        const prefixed = fn.replace(/^get/, 'getVibesPanel');
        expect(result).toContain(prefixed);
        expect(result).not.toMatch(new RegExp(`\\b${fn}\\b`));
      }
    });

    it('namespaces function calls too', () => {
      const input = 'const style = getContainerStyle();';
      const result = namespaceCollidingFunctions(input, 'VibesPanel.styles');
      expect(result).toBe('const style = getVibesPanelContainerStyle();');
    });
  });

  describe('VibesPanel', () => {
    it('namespaces colliding functions', () => {
      const input = 'return getContainerStyle();';
      const result = namespaceCollidingFunctions(input, 'VibesPanel');
      expect(result).toBe('return getVibesPanelContainerStyle();');
    });
  });

  describe('other components', () => {
    it('does not namespace for unrelated components', () => {
      const input = 'function getContainerStyle() {}';
      const result = namespaceCollidingFunctions(input, 'SomeOther');
      expect(result).toBe('function getContainerStyle() {}');
    });
  });
});

describe('transformComponent', () => {
  it('applies all transformations', () => {
    const input = `import { useState } from "react";
export function VibesButton() {
  const [open, setOpen] = useState(false);
  const style = getContentWrapperStyle();
  return null;
}`;
    const result = transformComponent(input, 'VibesButton');

    // Import removed
    expect(result).not.toContain('import');
    // Export removed
    expect(result).not.toMatch(/^export\s/m);
    // Hook prefixed
    expect(result).toContain('React.useState');
    // Function namespaced
    expect(result).toContain('getVibesButtonContentWrapperStyle');
  });

  it('preserves code structure', () => {
    const input = `import React from "react";
export function MyIcon() {
  return React.createElement("svg", null);
}`;
    const result = transformComponent(input, 'MyIcon');

    expect(result).toContain('function MyIcon()');
    expect(result).toContain('React.createElement');
  });

  it('applies LabelContainer namespacing', () => {
    const input = `function LabelContainer() {
  return getContainerStyle();
}`;
    const result = transformComponent(input, 'LabelContainer');
    expect(result).toContain('getLabelContainerContainerStyle');
    expect(result).not.toMatch(/\bgetContainerStyle\b/);
  });

  it('applies VibesPanel namespacing', () => {
    const input = `function VibesPanel() {
  return getContainerStyle();
}`;
    const result = transformComponent(input, 'VibesPanel');
    expect(result).toContain('getVibesPanelContainerStyle');
    expect(result).not.toMatch(/\bgetContainerStyle\b/);
  });
});
