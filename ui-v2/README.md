# codename goose ui v2

Your on-machine AI agent, automating tasks seamlessly.

## Development

### Getting Started

```bash
# Install dependencies
npm install

# Start both web and electron development servers
npm start

# Start web server only
npm run start:web

# Start electron only
npm run start:electron
```

### Building and Packaging

```bash
# Build web application
npm run build:web

# Build electron application
npm run build:electron

# Package electron app
npm run package

# Create distributable
npm run make

# Preview web build
npm run preview:web
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
npm run test:e2e:ui           # Run e2e tests with UI mode for both web and electron

npm run test:e2e:web          # Run web e2e tests with browser visible
npm run test:e2e:web:headless # Run web e2e tests headlessly
npm run test:e2e:web:ui       # Run web e2e tests with Playwright UI mode

npm run test:e2e:electron          # Run electron e2e tests with window visible
npm run test:e2e:electron:headless # Run electron e2e tests headlessly
npm run test:e2e:electron:ui       # Run electron e2e tests with Playwright UI mode

# Type checking
npm run typecheck      # Check all TypeScript files
npm run tsc:web       # Check web TypeScript files
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
│   ├── main.ts                # Main process entry
│   └── preload.ts             # Preload script
├── src/
│   ├── components/            # React components
│   ├── services/
│   │   └── platform/         # Platform abstraction layer
│   │       ├── web/          # Web implementation
│   │       ├── electron/     # Electron implementation
│   │       ├── IPlatformService.ts
│   │       └── index.ts
│   ├── test/                 # Test setup and configurations
│   │   ├── e2e/             # End-to-end test files
│   │   │   ├── electron/    # Electron-specific e2e tests
│   │   │   │   └── electron.spec.ts
│   │   │   └── web/        # Web-specific e2e tests
│   │   │       └── web.spec.ts
│   │   ├── setup.ts
│   │   └── types.d.ts
│   ├── App.tsx
│   ├── electron.tsx          # Electron renderer entry
│   └── web.tsx               # Web entry
├── electron.html             # Electron HTML template
├── index.html               # Web HTML template
├── playwright.config.ts     # Playwright e2e test configuration
├── vite.config.ts           # Vite config for web
├── vite.main.config.ts      # Vite config for electron main
├── vite.preload.config.ts   # Vite config for preload script
├── vite.renderer.config.ts  # Vite config for electron renderer
├── tsconfig.json           # TypeScript config for web
├── tsconfig.electron.json  # TypeScript config for electron
└── forge.config.ts         # Electron Forge config
```

## Architecture

The application follows a platform-agnostic architecture that allows it to run seamlessly in both web browsers and Electron environments. Here's a detailed breakdown of the key architectural components:

### Platform Abstraction Layer

The core of the architecture is built around a platform abstraction layer that provides a consistent interface for platform-specific functionality:

```typescript
// Platform Service Interface
export interface IPlatformService {
  copyToClipboard(text: string): Promise<void>;
  // Additional platform-specific operations can be added here
}
```

This is implemented through two concrete classes:

- `WebPlatformService`: Implements functionality for web browsers using Web APIs
- `ElectronPlatformService`: Implements functionality for Electron using IPC

### Platform Service Pattern

The application uses a dependency injection pattern for platform services:

1. **Service Interface**: `IPlatformService` defines the contract for platform-specific operations
2. **Platform Detection**: The app automatically detects the running environment and initializes the appropriate service
3. **Unified Access**: Components access platform features through a single `platformService` instance

Example usage in components:

```typescript
import { platformService } from '@platform';

// Platform-agnostic code
await platformService.copyToClipboard(text);
```

### Electron Integration

For Electron-specific functionality, the architecture includes:

1. **Preload Script**: Safely exposes Electron APIs to the renderer process

```typescript
// Type definitions for Electron APIs
declare global {
  interface Window {
    electronAPI: {
      copyToClipboard: (text: string) => Promise<void>;
    };
  }
}
```

2. **IPC Communication**: Typed handlers for main process communication

```typescript
// Electron implementation
export class ElectronPlatformService implements IPlatformService {
  async copyToClipboard(text: string): Promise<void> {
    return window.electronAPI.copyToClipboard(text);
  }
}
```

### Build System

The project uses a sophisticated build system with multiple configurations:

1. **Web Build**: Vite-based build for web deployment
2. **Electron Build**:
   - Main Process: Separate Vite config for Electron main process
   - Renderer Process: Specialized config for Electron renderer
   - Preload Scripts: Dedicated build configuration for preload scripts
