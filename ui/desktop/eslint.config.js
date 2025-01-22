const js = require('@eslint/js');
const tsParser = require('@typescript-eslint/parser');
const tsPlugin = require('@typescript-eslint/eslint-plugin');
const reactPlugin = require('eslint-plugin-react');
const reactHooksPlugin = require('eslint-plugin-react-hooks');

module.exports = [
  js.configs.recommended,
  {
    files: ['**/*.{ts,tsx}'],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
        ecmaFeatures: {
          jsx: true,
        },
      },
      globals: {
        // Electron/Node.js globals
        process: 'readonly',
        __dirname: 'readonly',
        require: 'readonly',
        module: 'readonly',
        // Browser globals
        window: 'readonly',
        document: 'readonly',
        navigator: 'readonly',
        localStorage: 'readonly',
        fetch: 'readonly',
        console: 'readonly',
        setTimeout: 'readonly',
        clearInterval: 'readonly',
        setInterval: 'readonly',
        CustomEvent: 'readonly',
        HTMLElement: 'readonly',
        HTMLInputElement: 'readonly',
        HTMLTextAreaElement: 'readonly',
        HTMLButtonElement: 'readonly',
        HTMLDivElement: 'readonly',
        FileList: 'readonly',
        FileReader: 'readonly',
        DOMParser: 'readonly',
        URL: 'readonly',
        URLSearchParams: 'readonly',
        Response: 'readonly',
        ReadableStream: 'readonly',
        AbortController: 'readonly',
        RequestCredentials: 'readonly',
        HeadersInit: 'readonly',
        KeyboardEvent: 'readonly',
        MouseEvent: 'readonly', // Add MouseEvent
        Node: 'readonly', // Add Node
        React: 'readonly',
        handleAction: 'readonly',
        requestAnimationFrame: 'readonly',
      },
    },
    plugins: {
      '@typescript-eslint': tsPlugin,
      'react': reactPlugin,
      'react-hooks': reactHooksPlugin,
    },
    rules: {
      ...tsPlugin.configs.recommended.rules,
      ...reactPlugin.configs.recommended.rules,
      ...reactHooksPlugin.configs.recommended.rules,
      // Customize rules
      'react/react-in-jsx-scope': 'off',
      'react/prop-types': 'off', // We use TypeScript for prop validation
      'react/no-unknown-property': ['error', {
        ignore: ['dark:fill'] // Allow Tailwind dark mode syntax
      }],
      'react/no-unescaped-entities': 'off', // Allow quotes in JSX
      '@typescript-eslint/explicit-module-boundary-types': 'off',
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/no-unused-vars': ['warn', {
        argsIgnorePattern: '^_',
        varsIgnorePattern: '^_'
      }],
      'react-hooks/rules-of-hooks': 'error',
      'react-hooks/exhaustive-deps': 'warn',
      '@typescript-eslint/ban-types': ['error', {
        types: {
          Object: {
            message: 'Use object instead',
            fixWith: 'object',
          },
        },
      }],
      '@typescript-eslint/no-var-requires': 'warn', // Downgrade to warning for Electron main process
      'no-undef': 'error',
      'no-useless-catch': 'warn',
    },
    settings: {
      react: {
        version: 'detect',
      },
    },
  },
];
