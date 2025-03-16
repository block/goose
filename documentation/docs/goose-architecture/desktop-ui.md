---
title: Desktop UI Architecture
sidebar_position: 7
---

# Desktop UI Architecture

The Goose desktop application is built using Electron with React and TypeScript. This document explains the desktop UI architecture, implementation, and key components.

## Core Architecture

The desktop application consists of three main parts:

1. **Main Process**: Handles window management, IPC, and system integration
2. **Renderer Process**: Implements the UI using React
3. **Goose Server**: A separate process that runs the Goose server

## Main Process

The main process is implemented in `main.ts` and is responsible for:

1. Creating and managing windows
2. Setting up IPC communication
3. Managing the application lifecycle
4. Starting and monitoring the Goose server

```typescript
// Window management
function createWindow(sessionId?: string, workingDir?: string) {
  // Implementation details
}

// IPC setup
function setupIPC() {
  // Implementation details
}

// Goose server management
function startGooseServer() {
  // Implementation details
}
```

## Renderer Process

The renderer process is implemented using React and is responsible for:

1. Rendering the UI
2. Handling user interactions
3. Communicating with the main process
4. Managing application state

### Component Structure

The UI is organized into several key components:

1. **App**: The root component that manages views and application state
2. **ChatView**: Handles chat interactions with the Goose agent
3. **SettingsView**: Manages application settings
4. **WelcomeView**: Provides onboarding for new users
5. **SessionsView**: Manages saved sessions

### State Management

The application uses React hooks and context for state management:

```typescript
// Model context
const { switchModel } = useModel();
const { addRecentModel } = useRecentModels();

// Chat state
const { chat, setChat } = useChat({ setView, setIsLoadingSession });

// Configuration
const { getExtensions, addExtension } = useConfig();
```

## IPC Communication

The desktop application uses Electron's IPC for communication between the main and renderer processes:

```typescript
// From renderer to main
window.electron.createChatWindow(undefined, window.appConfig.get('GOOSE_WORKING_DIR'));

// From main to renderer
window.electron.on('add-extension', handleAddExtension);
```

## Goose Server Integration

The desktop application integrates with the Goose server through:

1. Starting and monitoring the server process
2. Communicating with the server via HTTP
3. Managing server configuration

```typescript
// Server initialization
async function initializeSystem(provider: string, model: string) {
  // Implementation details
}
```

## Extension Management

The desktop application supports extension management:

1. Installing extensions from deep links
2. Managing extension configuration
3. Synchronizing built-in extensions

```typescript
// Extension installation
async function addExtensionFromDeepLink(link: string, setView: Function) {
  // Implementation details
}

// Built-in extension management
async function initializeBuiltInExtensions(addExtension: Function) {
  // Implementation details
}
```

## Session Management

The application supports session management:

1. Creating new sessions
2. Loading existing sessions
3. Managing session state

```typescript
// Session loading
async function fetchSessionDetails(sessionId: string) {
  // Implementation details
}
```

## Error Handling

The application implements comprehensive error handling:

1. Catching and displaying fatal errors
2. Handling authentication errors
3. Managing network errors

```typescript
// Error handling
function handleFatalError(_: any, errorMessage: string) {
  setFatalError(errorMessage);
}
```

## Cross-Platform Compatibility

The desktop application is designed for cross-platform compatibility:

1. Using Electron for cross-platform support
2. Managing platform-specific paths
3. Handling platform-specific features

## Best Practices

1. **Component Separation**: Keep components focused on specific responsibilities
2. **State Management**: Use React hooks and context for state management
3. **Error Handling**: Implement comprehensive error handling
4. **Performance Optimization**: Optimize rendering and IPC communication
