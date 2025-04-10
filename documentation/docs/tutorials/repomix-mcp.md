---
title: Repomix Extension
description: Add Repomix MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';

<!-- <YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/VIDEO_ID" /> -->


This tutorial covers how to add the [Repomix MCP Server](https://github.com/yamadashy/repomix) as a Goose extension  to enable automated repository packing, codebase analysis, architecture summaries, and code exploration.


:::tip TLDR

**Command**
```sh
npx -y repomix --mcp
```
:::

## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
:::


<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
  1. Run the `configure` command:
  ```sh
  goose configure
  ```

  2. Choose to add a `Command-line Extension`
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension 
    │
    ◆  What type of extension would you like to add?
    │  ○ Built-in Extension 
    // highlight-start    
    │  ● Command-line Extension (Run a local command or script)
    // highlight-end    
    │  ○ Remote Extension 
    └ 
  ```

  3. Give your extension a name
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    // highlight-start
    ◆  What would you like to call this extension?
    │  repomix
    // highlight-end
    └ 
  ```

  4. Enter the command
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  repomix
    │
    // highlight-start
    ◆  What command should be run?
    │  npx -y repomix --mcp
    // highlight-end
    └ 
  ```  

  5. Enter the number of seconds Goose should wait for actions to complete before timing out. Default is 300s
    ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  repomix
    │
    ◇  What command should be run?
    │  npx -y repomix --mcp
    │
    // highlight-start
    ◆  Please set the timeout for this tool (in secs):
    │  300
    // highlight-end
    │
    └ 
  ``` 
  
  6. Choose No when asked to add environment variables

   ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  repomix
    │
    ◇  What command should be run?
    │  npx -y repomix --mcp
    │     
    ◇  Please set the timeout for this tool (in secs):
    │  300
    │    
    // highlight-start
    ◆  Would you like to add environment variables?
    │  No
    │
    // highlight-end
    └  Added repomix extension
  ```  

  </TabItem>
  <TabItem value="ui" label="Goose Desktop">
  1. [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=repomix&arg=--mcp&id=repomix&name=Repomix&description=Pack%20repositories%20into%20AI-friendly%20formats%20for%20Goose)
  2. Press `Yes` to confirm the installation
  3. Click `Save Configuration`
  4. Scroll to the top and click `Exit` from the upper left corner
  </TabItem>
</Tabs>

## Example Usage

In this example, I’ll show you how to use Goose with the Repomix Extension to automate developer onboarding for the Axios repository. Goose will unpack the repo, highlight key files, skip unnecessary files, and give you a clear understanding of the architecture, improvement areas, and even draft a better README, all from a single prompt.

### Goose Prompt

> I’m trying to get familiar with the Axios repo, but I haven’t worked with it before. Can you explore the project on GitHub and give me a good overview of how it’s structured? Focus on the important parts like source code and config files, skip over logs or build artifacts. Help me understand how the pieces fit together, point out any areas for improvement, and if possible, write me a better README with the project description, features, setup instructions, and contribution guidelines.


### Goose Output

:::note CLI

I'll help you explore and analyze the Axios repository. Let me fetch it using the repomix tool, focusing on the essential parts of the codebase.

--- Pack Remote Repository ---

Let me read the packed codebase to analyze its structure and provide you with a comprehensive overview.

--- Read Repomix Output ---

Based on my analysis of the Axios codebase, I'll provide you with a comprehensive overview of the project structure and create an improved README. Let me break this down into sections:

**Axios Project Analysis**

**Project Overview**
Axios is a popular Promise-based HTTP client that works in both browser and Node.js environments. The project is well-structured and follows modern JavaScript practices.

**Core Architecture**
The project is organized into several key directories:

1. `/lib` - Main source code

- `/adapters` - HTTP request adapters (http, xhr, fetch)
- `/cancel` - Cancellation functionality
- `/core` - Core functionality (Axios, AxiosHeaders, etc.)
- `/defaults` - Default configurations
- `/env` - Environment-specific code
- `/helpers` - Utility functions
- `/platform` - Platform-specific implementations

2. Key Components:

Axios class - Main client implementation
AxiosHeaders - Headers management
InterceptorManager - Request/response interceptors
Multiple adapters supporting different environments

**Key Features**

1. Adapters System

- HTTP adapter for Node.js
- XHR adapter for browsers
- New Fetch adapter
- Pluggable adapter architecture

2. Request/Response Interceptors

- Pre/post request processing
- Error handling
- Request/response transformation

3. Advanced Features

- Progress tracking
- Request cancellation
- Rate limiting
- Automatic request/response transformations
- Form data handling
- File uploads

**Areas for Improvement**

1. Documentation

- More detailed API documentation
- Better examples for advanced features
- Clearer migration guides

2. Code Organization

- Consider splitting into smaller modules
- Better separation of browser/node specific code

3. Testing

- Increase test coverage
- Add more integration tests
- Better browser testing

Here's an improved README.md for the project:

**Axios**

![Axios](https://axios-http.com/assets/logo.svg)

A modern, promise-based HTTP client for browser and Node.js.

[![npm version](https://img.shields.io/npm/v/axios.svg)](https://www.npmjs.org/package/axios)
[![build status](https://img.shields.io/github/actions/workflow/status/axios/axios/ci.yml?branch=v1.x)](https://github.com/axios/axios/actions)
[![coverage](https://img.shields.io/coveralls/mzabriskie/axios.svg)](https://coveralls.io/r/mzabriskie/axios)
[![install size](https://img.shields.io/bundlephobia/minzip/axios)](https://bundlephobia.com/package/axios)

**Features**

- 🌐 Works in browser & Node.js
- 🔄 Automatic transforms for JSON data
- 🛡️ Client-side XSRF protection
- 🎯 Request & response interceptors
- ⚡️ Transform request & response data
- 🚫 Request cancellation
- 📊 Progress capturing
- 🔍 Automatic error handling
- 📝 TypeScript support
- 🔄 Automatic serialization of objects to `multipart/form-data` and `x-www-form-urlencoded`

**Installation**

```bash
# Using npm
npm install axios

# Using yarn
yarn add axios

# Using pnpm
pnpm add axios

# Using bun
bun add axios
```

**Quick Start**

```js
import axios from 'axios';

// Make a GET request
const response = await axios.get('/api/data');

// Make a POST request
const response = await axios.post('/api/data', {
  firstName: 'Fred',
  lastName: 'Flintstone'
});

// Request with config
const response = await axios({
  method: 'post',
  url: '/api/data',
  data: {
    firstName: 'Fred',
    lastName: 'Flintstone'
  }
});
```

**Core Features**

**Creating an Instance**

```js
const api = axios.create({
  baseURL: 'https://api.example.com',
  timeout: 5000,
  headers: {'X-Custom-Header': 'value'}
});
```
**Interceptors**

```js
// Request interceptor
axios.interceptors.request.use(config => {
  // Modify request config
  return config;
});

// Response interceptor
axios.interceptors.response.use(response => {
  // Handle response
  return response;
}, error => {
  // Handle error
  return Promise.reject(error);
});
```

**Request Cancellation**
```js
const controller = new AbortController();

axios.get('/api/data', {
  signal: controller.signal
});

// Cancel the request
controller.abort();
```

**Progress Tracking**

```js
axios.post('/upload', data, {
  onUploadProgress: (progressEvent) => {
    const percentCompleted = Math.round(
      (progressEvent.loaded * 100) / progressEvent.total
    );
    console.log(`Upload Progress: ${percentCompleted}%`);
  }
});
```

**Advanced Usage**

**Form Data**
```js
// Automatic serialization
await axios.post('/api/submit', {
  name: 'John',
  files: document.querySelector('#fileInput').files
}, {
  headers: {
    'Content-Type': 'multipart/form-data'
  }
});
```

**Rate Limiting**

```js
const response = await axios.post('/api/upload', data, {
  maxRate: [100 * 1024], // 100KB/s limit
  onUploadProgress: ({progress, rate}) => {
    console.log(`Upload: ${progress}%, Rate: ${rate/1024}KB/s`);
  }
});
```

**TypeScript Support**

```js
import axios, { AxiosResponse, AxiosError } from 'axios';

interface User {
  id: number;
  name: string;
}

try {
  const response: AxiosResponse<User> = await axios.get('/user/123');
  console.log(response.data.name);
} catch (error) {
  if (axios.isAxiosError(error)) {
    console.error('Axios Error:', error.response?.data);
  }
}
```

**Browser Support**

| Chrome | Firefox | Safari | Edge |
|--------|---------|--------|------|
| ![Chrome](https://raw.githubusercontent.com/alrra/browser-logos/main/src/chrome/chrome_64x64.png) <br /> Latest ✔️ | ![Firefox](https://raw.githubusercontent.com/alrra/browser-logos/main/src/firefox/firefox_64x64.png) <br /> Latest ✔️ | ![Safari](https://raw.githubusercontent.com/alrra/browser-logos/main/src/safari/safari_64x64.png) <br /> Latest ✔️ | ![Edge](https://raw.githubusercontent.com/alrra/browser-logos/main/src/edge/edge_64x64.png) <br /> Latest ✔️ |

**Contributing**
Please read our Contributing Guide before submitting a Pull Request.

**Security**
For security issues, please read our Security Policy.

**License**

```
This improved README provides better organization, more comprehensive examples, and clearer documentation of features. It maintains the essential information while making it more accessible and user-friendly.

Let me know if you'd like me to elaborate on any part of the analysis or README!
```
:::