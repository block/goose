# MCP Gateway Frontend

This is a Next.js 15 application with TypeScript, built for managing MCP (Model Context Protocol) gateway functionality.

## Table of Contents

- [Core Stack](#core-stack)
- [Directory Structure](#directory-structure)
- [Development Setup](#development-setup)
- [Development Scripts](#development-scripts)
- [Linting & Testing](#linting--testing)
- [Conventions](#conventions)
- [Deployment](#deployment)

## Core Stack

- **Framework**: Next.js 15 with App Router
- **Language**: TypeScript
- **Styling**: Tailwind CSS v4
- **UI Components**: Radix UI primitives + custom shadcn/ui components
- **State Management**: Zustand + React Query (TanStack Query)
- **Forms**: React Hook Form + Zod validation
- **Testing**: Vitest with MSW for API mocking

## Directory Structure

**`/src/app/`** - Next.js App Router pages

- `(auth)/` - Authentication routes (login)
- `(dashboard)/` - Main app routes (agents, apps, settings, members)
- `(landing)/` - Public landing page

**`/src/features/`** - Feature-based modules

- Each feature (agents, apps, auth, etc.) contains:
  - `api/` - API client functions
  - `components/` - Feature-specific components
  - `hooks/` - Custom React hooks
  - `types/` - TypeScript type definitions
  - `store/` - Zustand stores (when needed)

**`/src/components/`** - Shared components

- `ui/` - Base UI components (button, dialog, form, etc.)
- `ui-extensions/` - Enhanced components (data tables, date pickers)
- `layout/` - App layout components (sidebar, header, footer)
- `providers/` - Context providers

**`/src/lib/`** - Utilities and shared API logic

**`/public/`** - Static assets (icons, logos, manifests)

### Key Configuration Files

- `next.config.ts` - Next.js configuration
- `tailwind.config.ts` - Tailwind CSS setup
- `components.json` - shadcn/ui component registry
- `vitest.config.mts` - Test configuration
- `tsconfig.json` - TypeScript configuration

## Development Setup

1. **Install dependencies:**

   ```bash
   pnpm install
   ```

2. **Configure Environment Variables:** Copy `.env.example` to `.env`

   ```bash
   cp .env.example .env
   ```

3. **Start the application:**
   ```bash
   pnpm dev
   ```

## Development Scripts

- `pnpm dev` - Start development server
- `pnpm build` - Build for production
- `pnpm lint` - Run ESLint
- `pnpm format` - Format code with Prettier

## Linting & Testing

- **Format code:** `pnpm format`
- **Run linters:** `pnpm lint`

## Conventions

- API functions should be placed in feature-specific `api/` folders or `src/lib/api/`
- Use TypeScript strict mode
- Follow existing component patterns and naming conventions
- Prefer composition over inheritance

## Deployment

The application can be deployed on Vercel or any Node.js hosting platform that supports Next.js 15.

### Environment Variables

Configure the following environment variables for your deployment:

```sh
NEXT_PUBLIC_API_URL=<your-api-endpoint>
NEXT_PUBLIC_APP_URL=<your-app-url>
NEXT_PUBLIC_ENVIRONMENT=production
```
