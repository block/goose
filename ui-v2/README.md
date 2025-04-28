# Electron React App

A modern Electron and React application template with TypeScript support, featuring:
- 🔥 Hot reloading for development
- 🎯 TypeScript for type safety
- ⚡ Vite for fast builds
- 🧪 Vitest for testing
- 🎨 Platform-agnostic design

## Project Structure

```
├── electron/                   # Electron main process files
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
│   │   ├── setup.ts
│   │   └── types.d.ts
│   ├── App.tsx
│   ├── electron.tsx          # Electron renderer entry
│   └── web.tsx               # Web entry
├── electron.html             # Electron HTML template
├── index.html               # Web HTML template
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

### Component Architecture

The UI layer follows React best practices:

1. **Strict Mode**: Development builds use React.StrictMode for catching potential issues
2. **Suspense**: Lazy loading support with Suspense boundaries
3. **TypeScript**: Full type safety with React.FC and proper prop typing

Example component structure:
```typescript
interface Props {
  // Type definitions
}

const Component: React.FC<Props> = ({ ...props }) => {
  // Implementation
};
```

### Build System

The project uses a sophisticated build system with multiple configurations:

1. **Web Build**: Vite-based build for web deployment
2. **Electron Build**: 
   - Main Process: Separate Vite config for Electron main process
   - Renderer Process: Specialized config for Electron renderer
   - Preload Scripts: Dedicated build configuration for preload scripts

### Development Environment

The development setup supports:

1. **Hot Reloading**: Both web and Electron builds support HMR
2. **Concurrent Development**: Can run web and Electron development servers simultaneously
3. **Type Checking**: Real-time TypeScript type checking during development
4. **Testing**: Integrated Vitest setup with React Testing Library

## Scripts
