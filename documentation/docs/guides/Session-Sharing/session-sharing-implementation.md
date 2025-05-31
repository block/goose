---
sidebar_position: 20
title: Create a Session Sharing Service
sidebar_label: Create a Sharing Service
---

Goose session sharing enables real-time collaboration by allowing users to share their AI-assisted workflows, conversations, and project context with team members. When you implement session sharing, your users can:

- Collaborate on complex problem-solving with shared context and history
- Share AI-guided debugging sessions with team members
- Create persistent knowledge bases from successful problem-solving sessions
- Onboard new team members by sharing exemplar workflows
- Get help from colleagues who can join their session and see the full context

Setting up session sharing for your organization requires implementing a service that manages these shared sessions using the [Goose API](/docs/guides/Session-Sharing/SSE-api-reference.md). This guide will walk you through creating a secure, scalable sharing service that lets your users collaborate effectively while maintaining control over your organization's data.


## Set up a server to enable session sharing

Your session sharing service implements two main endpoints and handles proper authentication and security:

### API Requirements
```
POST /sessions/share
GET /sessions/share/:shareToken
```

### API Interface
```typescript
// POST /sessions/share Request Body
interface CreateSessionRequest {
  working_dir: string;
  messages: Message[];
  description: string;
  base_url: string;
  total_tokens: number | null;
}

// POST /sessions/share Response
interface CreateSessionResponse {
  share_token: string;
}

// GET /sessions/share/:shareToken Response
interface SharedSessionDetails {
  share_token: string;
  created_at: number;
  base_url: string;
  description: string;
  working_dir: string;
  messages: Message[];
  message_count: number;
  total_tokens: number | null;
}
```

### Security Requirements
- The server must verify a secret key for authentication
- The server should be configured with HTTPS
- CORS must be properly configured to allow requests from Goose desktop clients
- Consider implementing rate limiting for shared session creation

## How Session Sharing Works

When a user shares a session:
1. Their Goose Desktop client sends the session data to your sharing service
2. Your service generates a unique token and stores the session
3. The user gets a shareable link they can send to colleagues
4. Other users can open the link to join the session with full context
```
User A                     Server                      User B
  |                          |                           |
  |-- Create share --------->|                           |
  |<-- Token & URL ----------|                           |
  |                          |                           |
  |-- Share URL with User B->|                           |
  |                          |                           |
  |                          |<-- Open URL in browser ---|
  |                          |--- Serve HTML page ------>|
  |                          |                           |
  |                          |   [Click "Open in Goose"] |
  |                          |                           |
  |                          |<-- Request session -------|
  |                          |--- Return session ------->|
```


## Example Server Implementation
```javascript
const express = require('express');
const app = express();
const cors = require('cors');
const crypto = require('crypto');


app.use(express.json());
app.use(cors({
  origin: true,
  credentials: true
}));

// In-memory storage (replace with database in production)
const sessions = new Map();

// Middleware to verify secret key
const verifySecretKey = (req, res, next) => {
  const secretKey = req.headers['x-secret-key'];
  if (!secretKey || secretKey !== process.env.SECRET_KEY) {
    return res.status(401).json({ error: 'Unauthorized' });
  }
  next();
};

// Create shared session
app.post('/sessions/share', verifySecretKey, (req, res) => {
  const {
    working_dir,
    messages,
    description,
    base_url,
    total_tokens
  } = req.body;

  const shareToken = crypto.randomUUID();
  const session = {
    share_token: shareToken,
    created_at: Date.now(),
    base_url,
    description,
    working_dir,
    messages,
    message_count: messages.length,
    total_tokens
  };

  sessions.set(shareToken, session);
  res.json({ share_token: shareToken });
});

// Get shared session
app.get('/sessions/share/:shareToken', verifySecretKey, (req, res) => {
  const session = sessions.get(req.params.shareToken);
  if (!session) {
    return res.status(404).json({ error: 'Session not found' });
  }
  res.json(session);
});

app.listen(3000, () => {
  console.log('Goose sharing server running on port 3000');
});

```

## Example web UI implementation

The process involves creating a web interface that handles shared session links:

 **URL Format**
```
https://your-sharing-service.com/share/{share_token}
```

**Web Route Implementation**

Create a web page that lets a user open Goose with the shared session.

![website goose built](../../assets/guides/example-goose-share-session-page.png)

The page only attempts to auto-open if the user arrived directly at the URL (no referrer). This prevents repeated open attempts if they return to the page. It shows a clear "Opening..." message when attempting to open
and falls back to manual button if auto-open fails.  Installation instructions are shown if the app doesn't open.


If user arrives directly at the URL:

* Automatically attempt to open Goose Desktop
* Show "Opening..." message
* After 3 seconds, show manual instructions if still on the page

If user arrives from a referring page:

* Shows the "Open in Goose Desktop" button
* Waits for user to click
* Then follows the same sequence as auto-open


In both cases:

If Goose Desktop opens successfully, the user leaves this page
If it fails to open, they see the installation instructions

### Web page code example
```javascript
app.get('/share/:token', (req, res) => {
  const token = req.params.token;
  const baseUrl = `${req.protocol}://${req.get('host')}`;
  const gooseUrl = `goose://share/${token}?base_url=${encodeURIComponent(baseUrl)}`;
  
  // Send HTML page
  res.send(`
    <!DOCTYPE html>
    <html>
    <head>
      <title>Goose Shared Session</title>
      <meta name="viewport" content="width=device-width, initial-scale=1">
      <style>
        body {
          font-family: system-ui, -apple-system, sans-serif;
          max-width: 800px;
          margin: 0 auto;
          padding: 20px;
          line-height: 1.6;
        }
        .button {
          display: inline-block;
          background: #4F46E5;
          color: white;
          padding: 12px 24px;
          border-radius: 6px;
          text-decoration: none;
          margin: 10px 0;
        }
        .info {
          background: #F3F4F6;
          padding: 16px;
          border-radius: 8px;
          margin: 20px 0;
        }
        #openingApp {
          display: none;
        }
        #openButton {
          display: block;
        }
        #manualInstructions {
          display: none;
        }
      </style>
    </head>
    <body>
      <h1>Goose Shared Session</h1>
      
      <div class="info">
        <p>Someone has shared a Goose session with you!</p>
      </div>

      <div id="openingApp" class="info">
        <p>Opening Goose Desktop...</p>
        <p>If nothing happens, click the button below:</p>
      </div>

      <div id="openButton">
        <a href="${gooseUrl}" class="button" id="openGooseBtn">
          Open in Goose Desktop
        </a>
      </div>

      <div id="manualInstructions" class="info">
        <h3>Don't have Goose Desktop?</h3>
        <p>To view this shared session:</p>
        <ol>
          <li>Install Goose Desktop from <a href="https://block.github.io/goose/docs/getting-started/installation">Install Goose</a></li>
          <li>Configure session sharing in Settings:</li>
          <ul>
            <li>Enable session sharing</li>
            <li>Set the base URL to: <code>${baseUrl}</code></li>
          </ul>
          <li>Return to this page and click the "Open in Goose Desktop" button</li>
        </ol>
      </div>

      <script>
        let hasAttemptedOpen = false;
        const openingMessage = document.getElementById('openingApp');
        const manualInstructions = document.getElementById('manualInstructions');
        const openButton = document.getElementById('openButton');

        // Function to attempt opening Goose
        function openGooseDesktop() {
          if (!hasAttemptedOpen) {
            hasAttemptedOpen = true;
            openingMessage.style.display = 'block';
            
            // Try to open Goose Desktop
            window.location.href = '${gooseUrl}';
            
            // After a delay, show manual instructions if we're still here
            setTimeout(() => {
              openingMessage.style.display = 'none';
              manualInstructions.style.display = 'block';
            }, 3000);
          }
        }

        // Listen for button click
        document.getElementById('openGooseBtn').addEventListener('click', (e) => {
          e.preventDefault();
          openGooseDesktop();
        });

        // Try to open automatically only if this looks like a direct navigation
        if (!document.referrer) {
          openGooseDesktop();
        }
      </script>
    </body>
    </html>
  `);
});
```




## Additional Considerations

1. **Security**
- Use unguessable share tokens (UUIDs recommended)
- Implement token expiration
- Validate base URL matches between clients
- Add rate limiting
- Use HTTPS
- Add appropriate security headers

2. **Error Handling**
- Invalid or expired tokens
- Base URL mismatches
- Sharing not enabled
- Network errors

3. **Client Configuration**
- Both users need session sharing enabled
- Both users must use the same base URL
- Base URL can be set via:
  - Environment variable (`GOOSE_BASE_URL_SHARE`)
  - User configuration in settings