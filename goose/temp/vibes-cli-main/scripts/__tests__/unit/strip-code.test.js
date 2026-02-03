/**
 * Tests for strip-code.js utilities
 *
 * These functions remove import/export statements and template-provided
 * constants from JSX code before injecting into templates.
 */

import { describe, it, expect } from 'vitest';
import {
  stripImports,
  stripExportDefault,
  stripConfig,
  stripConstants,
  stripForTemplate
} from '../../lib/strip-code.js';

describe('stripImports', () => {
  describe('single-line imports', () => {
    it('removes default imports', () => {
      const code = `import React from "react";
const App = () => <div>Hello</div>;`;
      const result = stripImports(code);
      expect(result).not.toContain('import React');
      expect(result).toContain('const App');
    });

    it('removes named imports', () => {
      const code = `import { useState, useEffect } from "react";
function App() {}`;
      const result = stripImports(code);
      expect(result).not.toContain('import {');
      expect(result).toContain('function App');
    });

    it('removes side-effect imports', () => {
      const code = `import "./styles.css";
const App = () => null;`;
      const result = stripImports(code);
      expect(result).not.toContain('import "./styles.css"');
      expect(result).toContain('const App');
    });

    it('removes imports with single quotes', () => {
      const code = `import React from 'react';
const x = 1;`;
      const result = stripImports(code);
      expect(result).not.toContain('import React');
      expect(result).toContain('const x = 1');
    });
  });

  describe('multi-line imports', () => {
    it('removes multi-line named imports', () => {
      const code = `import {
  useState,
  useEffect,
  useCallback
} from "react";
const App = () => null;`;
      const result = stripImports(code);
      expect(result).not.toContain('import {');
      expect(result).not.toContain('useState');
      expect(result).toContain('const App');
    });
  });

  describe('mixed imports', () => {
    it('removes all import types in sequence', () => {
      const code = `import React from "react";
import { useState } from "react";
import "./styles.css";
const App = () => <div>Hello</div>;`;
      const result = stripImports(code);
      expect(result).not.toContain('import');
      expect(result).toContain('const App');
    });
  });

  describe('edge cases', () => {
    it('preserves import-like strings in code', () => {
      const code = `const text = "import something from somewhere";
const App = () => null;`;
      const result = stripImports(code);
      expect(result).toContain('import something from somewhere');
    });

    it('handles empty string', () => {
      const result = stripImports('');
      expect(result).toBe('');
    });

    it('handles code without imports', () => {
      const code = 'const App = () => <div>Hello</div>;';
      const result = stripImports(code);
      expect(result).toBe(code);
    });
  });
});

describe('stripExportDefault', () => {
  it('removes export default from function declaration', () => {
    const code = 'export default function App() { return null; }';
    const result = stripExportDefault(code);
    expect(result).toBe('function App() { return null; }');
  });

  it('removes export default from const declaration', () => {
    const code = 'export default const App = () => null;';
    const result = stripExportDefault(code);
    expect(result).toBe('const App = () => null;');
  });

  it('only removes first export default', () => {
    const code = `export default function App() {}
export default function Other() {}`;
    const result = stripExportDefault(code);
    expect(result).toContain('function App');
    expect(result).toContain('export default function Other');
  });

  it('preserves code without export default', () => {
    const code = 'function App() { return null; }';
    const result = stripExportDefault(code);
    expect(result).toBe(code);
  });
});

describe('stripConfig', () => {
  it('removes CONFIG object declaration', () => {
    const code = `const CONFIG = {
  APP_NAME: "test",
  DOMAIN: "test.com"
};
function App() {}`;
    const result = stripConfig(code);
    expect(result).not.toContain('const CONFIG');
    expect(result).not.toContain('APP_NAME');
    expect(result).toContain('function App');
  });

  it('handles CONFIG with nested objects', () => {
    const code = `const CONFIG = {
  APP_NAME: "test",
  FEATURES: ["a", "b", "c"]
};
const App = () => null;`;
    const result = stripConfig(code);
    expect(result).not.toContain('const CONFIG');
    expect(result).toContain('const App');
  });

  it('preserves other const declarations', () => {
    const code = `const CONFIG = {
  APP_NAME: "test"
};
const otherConfig = { foo: "bar" };`;
    const result = stripConfig(code);
    expect(result).not.toContain('const CONFIG =');
    expect(result).toContain('const otherConfig');
  });
});

describe('stripConstants', () => {
  it('removes specified constants', () => {
    const code = `const APP_NAME = "test";
const DOMAIN = "example.com";
const App = () => null;`;
    const result = stripConstants(code, ['APP_NAME', 'DOMAIN']);
    expect(result).not.toContain('const APP_NAME');
    expect(result).not.toContain('const DOMAIN');
    expect(result).toContain('const App');
  });

  it('handles empty constants array', () => {
    const code = 'const APP_NAME = "test";';
    const result = stripConstants(code, []);
    expect(result).toBe(code);
  });

  it('preserves constants not in the list', () => {
    const code = `const KEEP_THIS = "yes";
const REMOVE_THIS = "no";`;
    const result = stripConstants(code, ['REMOVE_THIS']);
    expect(result).toContain('const KEEP_THIS');
    expect(result).not.toContain('const REMOVE_THIS');
  });
});

describe('stripForTemplate', () => {
  it('strips all template conflicts', () => {
    const code = `import React from "react";
import { useState } from "react";
const CONFIG = {
  APP_NAME: "test"
};
export default function App() {
  return <div>Hello</div>;
}`;
    const result = stripForTemplate(code);
    expect(result).not.toContain('import');
    expect(result).not.toContain('const CONFIG');
    expect(result).not.toContain('export default');
    expect(result).toContain('function App');
  });

  it('strips additional template constants when provided', () => {
    const code = `import React from "react";
const CLERK_KEY = "pk_test_123";
const APP_NAME = "myapp";
function App() {}`;
    const result = stripForTemplate(code, ['CLERK_KEY', 'APP_NAME']);
    expect(result).not.toContain('import');
    expect(result).not.toContain('const CLERK_KEY');
    expect(result).not.toContain('const APP_NAME');
    expect(result).toContain('function App');
  });

  it('trims whitespace', () => {
    const code = '  \n  const App = () => null;\n  ';
    const result = stripForTemplate(code);
    expect(result).toBe('const App = () => null;');
  });

  it('handles empty template constants array', () => {
    const code = 'import React from "react";\nfunction App() {}';
    const result = stripForTemplate(code, []);
    expect(result).not.toContain('import');
    expect(result).toContain('function App');
  });
});
