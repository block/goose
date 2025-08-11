# Technical Stack & Architecture

**Last Updated:** [Date]  
**Template:** React Router SPA

## Core Technologies

### Frontend Framework
```json
{
  "react": "^18.2.0",
  "react-dom": "^18.2.0",
  "react-router-dom": "^6.22.0"
}
```

### Build Tool
```json
{
  "vite": "^5.1.0",
  "@vitejs/plugin-react": "^4.2.1"
}
```

### Development Dependencies
```json
{
  "@types/react": "^18.2.55",
  "@types/react-dom": "^18.2.19",
  "@typescript-eslint/eslint-plugin": "^6.21.0",
  "@typescript-eslint/parser": "^6.21.0",
  "eslint": "^8.56.0",
  "eslint-plugin-react-hooks": "^4.6.0",
  "eslint-plugin-react-refresh": "^0.4.5",
  "typescript": "^5.2.2"
}
```

## Project Structure
```
src/
├── components/          # Reusable UI components
│   ├── common/         # Shared components (Button, Input, etc.)
│   └── layout/         # Layout components (Header, Footer, etc.)
├── pages/              # Route-based page components
├── hooks/              # Custom React hooks
├── utils/              # Utility functions
├── services/           # API and external service integrations
├── styles/             # Global styles and theme
├── assets/             # Images, fonts, and static files
├── App.tsx             # Main app component with routing
└── main.tsx            # Application entry point
```

## Routing Architecture

### Route Configuration
```typescript
// Current routes structure
const routes = [
  { path: '/', element: <Home /> },
]
```

### Route Guards
- Public routes: Accessible without authentication
- Protected routes: Require authentication
- Role-based routes: Require specific permissions

## State Management

### Current Approach
- React Context API for global state
- Local component state for UI state
- Custom hooks for shared logic

### State Structure
```typescript
interface AppState {
  user: User | null;
  theme: 'light' | 'dark';
  // Add more global state here
}
```

## Styling Approach

### CSS Modules
- Component-scoped styles
- File naming: `Component.module.css`

### Global Styles
- Reset/normalize CSS
- Typography scale
- Color variables
- Spacing system

## API Integration

### HTTP Client
```typescript
// Using native fetch with wrapper
const apiClient = {
  get: (url: string) => fetch(url).then(r => r.json()),
  post: (url: string, data: any) => fetch(url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data)
  })
}
```

### API Endpoints
```typescript
const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:3000';

const endpoints = {
  auth: {
    login: `${API_BASE}/auth/login`,
    logout: `${API_BASE}/auth/logout`,
    refresh: `${API_BASE}/auth/refresh`
  },
  users: {
    profile: `${API_BASE}/users/profile`,
    update: `${API_BASE}/users/update`
  }
}
```

## Build Configuration

### Vite Config
```typescript
// vite.config.ts
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      '/api': 'http://localhost:3000'
    }
  },
  build: {
    outDir: 'dist',
    sourcemap: true
  }
})
```

## Testing Setup

### Test Framework
- Vitest for unit tests
- React Testing Library for component tests
- Playwright for E2E tests

### Test Structure
```
src/
├── __tests__/          # Test files
├── setupTests.ts       # Test configuration
└── test-utils.tsx      # Test utilities
```

### Output Structure
```
dist/
├── index.html
├── assets/
│   ├── index-[hash].js
│   └── index-[hash].css
└── favicon.ico
```

### Hosting Requirements
- Static file hosting (Netlify, Vercel, S3, etc.)
- SPA routing support (fallback to index.html)
- HTTPS certificate

---

*This document contains technical implementation details. For product features and design, see PRODUCT_SPEC.md*