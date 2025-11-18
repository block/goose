# User Context System for Goose

This document describes the comprehensive user context system implemented for Goose, which enables the AI to understand and remember users when they're introduced in collaborative sessions.

## Overview

The user context system allows Goose to:
- **Automatically detect user introductions** in messages like "hey goose meet James"
- **Extract user information** such as names, roles, expertise, and relationships
- **Maintain persistent user profiles** with preferences and collaboration history
- **Provide contextual information to the AI** for personalized interactions
- **Track user activity** and collaboration patterns across sessions

## Architecture

### Core Components

1. **UserContextService** (`src/services/UserContextService.ts`)
   - Central service for managing user profiles and introductions
   - Handles data persistence using localStorage
   - Provides methods for searching, updating, and managing user data

2. **useUserContext Hook** (`src/hooks/useUserContext.ts`)
   - React hook that provides a clean interface to the UserContextService
   - Handles error states and loading states
   - Provides typed methods for all user context operations

3. **Chat Engine Integration** (`src/hooks/useChatEngine.ts`)
   - Automatically processes introductions in incoming messages
   - Injects user context into AI messages for better personalization
   - Updates user activity timestamps for collaborators

4. **UserContextPanel Component** (`src/components/UserContextPanel.tsx`)
   - Optional UI component to display user information in the chat interface
   - Shows user profiles, expertise, and collaboration history

## Features

### 1. Automatic Introduction Detection

The system automatically detects various introduction patterns:

```typescript
// Examples of detected patterns:
"meet John"
"this is Sarah, she's a developer"
"let me introduce Mike - he's our designer"
"say hello to Lisa! She handles marketing"
"@goose meet Alex. Alex specializes in data science"
```

### 2. Information Extraction

From introduction messages, the system extracts:
- **Name**: The person's name or preferred name
- **Role**: Job title or function (developer, designer, manager, etc.)
- **Expertise**: Skills, technologies, or areas of specialization
- **Relationship**: How they relate to the introducer (colleague, client, etc.)

### 3. User Profile Management

Each user profile includes:
```typescript
interface UserProfile {
  userId: string;
  displayName?: string;
  avatarUrl?: string;
  preferredName?: string;
  role?: string;
  expertise?: string[];
  timezone?: string;
  workingHours?: {
    start: string;
    end: string;
    days: string[];
  };
  preferences?: {
    communicationStyle?: 'formal' | 'casual' | 'technical';
    responseLength?: 'brief' | 'detailed' | 'comprehensive';
    notificationLevel?: 'all' | 'mentions' | 'minimal';
  };
  introducedBy?: string;
  introducedAt?: Date;
  lastSeen?: Date;
  commonTopics?: string[];
  collaborationHistory?: CollaborationHistory[];
  notes?: string;
}
```

### 4. AI Context Integration

When users send messages to Goose, the system:
1. **Processes introductions** in the message content
2. **Generates context summaries** for the current session
3. **Injects user context** into AI messages for personalized responses
4. **Updates activity timestamps** for all participants

Example of generated context:
```markdown
## User Context

The following users have been introduced in this session:

- **James** (Developer) - Expertise: React, TypeScript - Relationship: colleague
- **Sarah** (Designer) - Expertise: UI/UX, Figma - Relationship: teammate
- **Mike** (Senior Engineer) - Expertise: Backend, Databases

Use this context to personalize your interactions with these users.
```

## Usage

### Basic Usage

The system works automatically once integrated. When users introduce others:

```
User: "Hey goose, meet James. He's a React developer who specializes in frontend architecture."
```

Goose will:
1. Detect the introduction of "James"
2. Extract role: "React developer"  
3. Extract expertise: ["React", "frontend architecture"]
4. Store this information for future reference
5. Include this context in subsequent AI responses

### Programmatic Usage

```typescript
import { useUserContext } from './hooks/useUserContext';

function MyComponent() {
  const userContext = useUserContext();
  
  // Process an introduction
  const introductions = await userContext.processIntroduction(
    "Meet Sarah, our UX designer",
    "primary-user",
    "session-123"
  );
  
  // Search for users
  const developers = await userContext.searchUsers("developer");
  
  // Get user profile
  const user = await userContext.getUserProfile("user-123");
  
  // Update user information
  await userContext.updateUserProfile("user-123", {
    expertise: ["React", "Node.js"],
    preferences: {
      communicationStyle: 'technical',
      responseLength: 'detailed'
    }
  });
}
```

### Testing

Use the built-in test utilities:

```javascript
// In browser console
window.testUserContext.runAllTests();

// Test specific functionality
window.testUserContext.testUserIntroductions();
window.testUserContext.testUserSearch();
```

## Integration with Matrix

The user context system integrates seamlessly with Matrix collaborative sessions:

1. **Matrix User IDs**: When users are mentioned with @username, the system attempts to match them with Matrix user IDs
2. **Collaboration History**: Tracks when users join/leave Matrix rooms
3. **Activity Tracking**: Updates last seen timestamps based on Matrix messages
4. **Primary User Logic**: Respects the primary user response logic while maintaining context for all participants

## Data Persistence

User context data is stored in localStorage with the following structure:

```typescript
{
  profiles: {
    [userId: string]: UserProfile
  },
  introductions: UserIntroduction[],
  lastUpdated: number
}
```

### Data Management

```typescript
// Export user data
const data = await userContextService.exportData();

// Import user data
await userContextService.importData(data);

// Clear all data
await userContextService.clearAllData();
```

## Privacy and Security

- **Local Storage Only**: All user context data is stored locally in the browser
- **No External Transmission**: User profiles are not sent to external servers
- **Opt-in Context**: Context is only injected into AI messages when relevant
- **User Control**: Users can clear their data at any time

## Configuration

The system can be configured through the UserContextService:

```typescript
// Initialize with custom settings
await userContextService.initialize();

// Configure introduction detection patterns
userContextService.addCustomIntroductionPattern(/custom pattern/gi);

// Set context injection preferences
userContextService.setContextInjectionEnabled(true);
```

## Future Enhancements

Potential improvements for the user context system:

1. **Smart Notifications**: Notify when users with relevant expertise join sessions
2. **Skill Matching**: Suggest collaborators based on project requirements
3. **Integration with Calendar**: Respect user working hours and availability
4. **Advanced Search**: Full-text search across user profiles and conversation history
5. **Context Suggestions**: Suggest relevant context to include based on conversation topics
6. **User Preferences UI**: Interface for users to manage their own profiles
7. **Export/Import**: Backup and restore user context data
8. **Analytics**: Track collaboration patterns and user engagement

## Troubleshooting

### Common Issues

1. **Introductions not detected**
   - Check that the message contains clear introduction keywords
   - Verify the introduction patterns in the service
   - Test with `window.testUserContext.testUserIntroductions()`

2. **Context not injected**
   - Ensure the user context service is initialized
   - Check that there are users in the current session
   - Verify AI is enabled for the session

3. **Data not persisting**
   - Check localStorage permissions
   - Verify the storage key is not being cleared
   - Test export/import functionality

### Debug Tools

```javascript
// Check service status
console.log('User context initialized:', userContextService.isInitialized);

// View all users
window.debugUserContext();

// Test introduction processing
window.testUserIntroduction("meet John, he's a developer");

// Check storage
console.log(localStorage.getItem('goose-user-context'));
```

## API Reference

### UserContextService

#### Methods

- `initialize()`: Initialize the service and load stored data
- `processIntroduction(message, introducedBy, sessionId, mentionedUserIds?)`: Process introduction in message
- `createOrUpdateUserProfile(userId, updates)`: Create or update user profile
- `getUserProfile(userId)`: Get user profile by ID
- `searchUsersByName(name)`: Search users by name or expertise
- `getAllUserProfiles()`: Get all user profiles
- `generateUserContextSummary(sessionId)`: Generate context summary for session
- `containsIntroductions(message)`: Check if message contains introductions
- `updateLastSeen(userId)`: Update user's last seen timestamp
- `addCollaborationHistory(userId, sessionId, role)`: Add collaboration history entry
- `exportData()`: Export all user context data
- `importData(data)`: Import user context data
- `clearAllData()`: Clear all stored data

### useUserContext Hook

Returns an object with all UserContextService methods plus:
- `isInitialized`: Boolean indicating if service is ready
- `error`: Current error state (if any)

All methods are wrapped with error handling and return promises.

## Conclusion

The user context system provides Goose with the ability to understand and remember users in collaborative sessions, enabling more personalized and contextually aware interactions. The system is designed to be privacy-focused, performant, and easy to integrate into existing workflows.
