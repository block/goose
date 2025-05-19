# codename goose ui v2

Your on-machine AI agent, automating tasks seamlessly.

## Development

### Getting Started

```bash
# Install dependencies
npm install

# Start electron development server
npm start
```

### Building and Packaging

```bash
# Build electron application
npm run build

# Package electron app
npm run package

# Create distributable
npm run make
```

### Quality and Testing

```bash
# Run tests
npm test

# Run tests with UI
npm run test:ui

# Generate test coverage
npm run test:coverage

# End-to-End Testing
npm run test:e2e              # Run all e2e tests headlessly
npm run test:e2e:ui           # Run e2e tests with UI mode

# Type checking
npm run typecheck      # Check all TypeScript files
npm run tsc:electron  # Check electron TypeScript files

# Linting
npm run lint          # Run all linting
npm run lint:fix      # Fix linting issues
npm run lint:style    # Check CSS
npm run lint:style:fix # Fix CSS issues

# Code formatting
npm run prettier      # Check formatting
npm run prettier:fix  # Fix formatting
npm run format        # Fix all formatting (prettier + style)

# Run all checks (types, lint, format)
npm run check-all
```

## Project Structure

```
├── electron/                  # Electron main process files
│   ├── main.ts               # Main process entry
│   └── preload.ts            # Preload script
├── src/
│   ├── components/           # React components
│   ├── services/             # Application services
│   │   └── platform/        # Platform services
│   ├── test/                # Test setup and configurations
│   │   ├── e2e/            # End-to-end test files
│   │   ├── setup.ts
│   │   └── types.d.ts
│   └── App.tsx              # Main application component
├── index.html               # HTML template
├── playwright.config.ts     # Playwright e2e test configuration
├── vite.config.ts          # Base Vite configuration
├── vite.main.config.ts     # Vite config for electron main
├── vite.preload.config.ts  # Vite config for preload script
├── vite.renderer.config.ts # Vite config for electron renderer
├── tsconfig.json          # Base TypeScript configuration
├── tsconfig.electron.json # TypeScript config for electron
├── tsconfig.node.json    # TypeScript config for Node.js
└── forge.config.ts       # Electron Forge configuration
```

## Architecture

The application is built as an Electron desktop application with a modern React frontend. Here's a detailed breakdown of the key architectural components:

### Platform Services

The application uses a service-based architecture to handle platform-specific functionality:

```typescript
// Platform Service Interface
export interface IPlatformService {
  copyToClipboard(text: string): Promise<void>;
  // Additional platform-specific operations
}
```

### Electron Integration

The architecture includes several key Electron-specific components:

1. **Preload Script**: Safely exposes Electron APIs to the renderer process

```typescript
// Type definitions for Electron APIs
declare global {
  interface Window {
    electronAPI: {
      copyToClipboard: (text: string) => Promise<void>;
      // Other API methods
    };
  }
}
```

2. **IPC Communication**: Typed handlers for main process communication

```typescript
// Electron platform service implementation
export class PlatformService implements IPlatformService {
  async copyToClipboard(text: string): Promise<void> {
    return window.electronAPI.copyToClipboard(text);
  }
}
```

### Build System

The project uses a multi-configuration build system:

1. **Main Process**: Built using `vite.main.config.ts`
   - Handles core Electron functionality
   - Manages window creation and system integration

2. **Renderer Process**: Built using `vite.renderer.config.ts`
   - React application
   - UI components and application logic

3. **Preload Scripts**: Built using `vite.preload.config.ts`
   - Secure bridge between main and renderer processes
   - Exposes limited API surface to frontend

The build process is managed by Electron Forge, which handles:
- Development environment setup
- Application packaging
- Distribution creation for various platforms