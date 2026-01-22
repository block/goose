# Goose Web Client 🪿

Goose Web Client is a modern, React-based interface for the Goose AI agent. It provides a rich chat experience, session management, and visual tool outputs, designed to help developers build and debug with ease.

## Demo

<video src="./screenshot-c.mov" controls="controls" style="max-width: 100%;">
  Your browser does not support the video tag.
</video>

> [Download Demo Video](./screenshot-c.mov)

## Features

- **🤖 AI Chat Interface**: Seamless conversation with the Goose AI agent.
- **🛠️ Tool Visualization**: Rich, collapsible displays for tool calls and outputs.
- **📊 Session Management**: Create, rename, delete, and search through chat sessions.
- **🔒 Secure & Stable**: Built-in Error Boundaries and sanitized runtime environments.
- **🎨 Modern UI**: Clean, responsive design with comprehensive CSS variable theming.

## Tech Stack

- **Framework**: React 18 + TypeScript
- **Build Tool**: Vite
- **Routing**: React Router v6
- **Testing**: Vitest + React Testing Library
- **Styling**: Vanilla CSS (Variables & Design Tokens)

## Getting Started

### Prerequisites

- Node.js (v18 or higher)
- Access to a running `goosed` backend service

### Installation

```bash
# Navigate to the web directory
cd ui/web

# Install dependencies
npm install
```

### Running Locally

```bash
# Start the development server
npm run dev
```

The app will be available at `http://localhost:5173`.

### Running Tests

We use **Vitest** for unit and component testing.

```bash
# Run all tests
npm test run

# Watch mode
npm test
```

### Building for Production

```bash
npm run build
```

## Project Structure

```
src/
├── components/   # Reusable UI components (ChatInput, Message, etc.)
├── contexts/     # Global state (GoosedContext)
├── hooks/        # Custom hooks (useChat)
├── pages/        # Page views (Home, Chat, History)
├── __tests__/    # Unit and integration tests
└── main.tsx      # Application entry point
```

## Contributing

1. Ensure all tests pass before submitting a PR.
2. Follow the existing code style and structure.
3. New features should include appropriate test coverage.
