PKCE instructions: 

PKCE Guide
Step 1: Send your user to OpenRouter
To start the PKCE flow, send your user to OpenRouter’s /auth URL with a callback_url parameter pointing back to your site:


With S256 Code Challenge (Recommended)

With Plain Code Challenge

Without Code Challenge

https://openrouter.ai/auth?callback_url=<YOUR_SITE_URL>&code_challenge=<CODE_CHALLENGE>&code_challenge_method=S256
The code_challenge parameter is optional but recommended.

Your user will be prompted to log in to OpenRouter and authorize your app. After authorization, they will be redirected back to your site with a code parameter in the URL:

Alt text

Use SHA-256 for Maximum Security
For maximum security, set code_challenge_method to S256, and set code_challenge to the base64 encoding of the sha256 hash of code_verifier.

For more info, visit Auth0’s docs.

How to Generate a Code Challenge
The following example leverages the Web Crypto API and the Buffer API to generate a code challenge for the S256 method. You will need a bundler to use the Buffer API in the web browser:

Generate Code Challenge

import { Buffer } from 'buffer';
async function createSHA256CodeChallenge(input: string) {
  const encoder = new TextEncoder();
  const data = encoder.encode(input);
  const hash = await crypto.subtle.digest('SHA-256', data);
  return Buffer.from(hash).toString('base64url');
}
const codeVerifier = 'your-random-string';
const generatedCodeChallenge = await createSHA256CodeChallenge(codeVerifier);
Localhost Apps
If your app is a local-first app or otherwise doesn’t have a public URL, it is recommended to test with http://localhost:3000 as the callback and referrer URLs.

When moving to production, replace the localhost/private referrer URL with a public GitHub repo or a link to your project website.

Step 2: Exchange the code for a user-controlled API key
After the user logs in with OpenRouter, they are redirected back to your site with a code parameter in the URL:

Alt text

Extract this code using the browser API:

Extract Code

const urlParams = new URLSearchParams(window.location.search);
const code = urlParams.get('code');
Then use it to make an API call to https://openrouter.ai/api/v1/auth/keys to exchange the code for a user-controlled API key:

Exchange Code

const response = await fetch('https://openrouter.ai/api/v1/auth/keys', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    code: '<CODE_FROM_QUERY_PARAM>',
    code_verifier: '<CODE_VERIFIER>', // If code_challenge was used
    code_challenge_method: '<CODE_CHALLENGE_METHOD>', // If code_challenge was used
  }),
});
const { key } = await response.json();
And that’s it for the PKCE flow!

Step 3: Use the API key
Store the API key securely within the user’s browser or in your own database, and use it to make OpenRouter requests.

Make an OpenRouter request

fetch('https://openrouter.ai/api/v1/chat/completions', {
  method: 'POST',
  headers: {
    Authorization: 'Bearer <API_KEY>',
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    model: 'openai/gpt-4o',
    messages: [
      {
        role: 'user',
        content: 'Hello!',
      },
    ],
  }),
});
Error Codes
400 Invalid code_challenge_method: Make sure you’re using the same code challenge method in step 1 as in step 2.
403 Invalid code or code_verifier: Make sure your user is logged in to OpenRouter, and that code_verifier and code_challenge_method are correct.
405 Method Not Allowed: Make sure you’re using POST and HTTPS for your request.
