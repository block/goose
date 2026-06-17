#!/usr/bin/env node

const baseUrl = process.env.SECURITY_CHAT_BASE_URL;
const secret = process.env.SECURITY_CHAT_SECRET;
const workingDir = process.env.SECURITY_CHAT_WORKDIR;
const prompt = process.env.SECURITY_CHAT_PROMPT || 'Reply with exactly: pong';
const maxAttempts = Number(process.env.SECURITY_CHAT_MAX_ATTEMPTS || '2');

if (!baseUrl || !secret || !workingDir) {
  throw new Error('Missing SECURITY_CHAT_BASE_URL, SECURITY_CHAT_SECRET, or SECURITY_CHAT_WORKDIR');
}

const headers = {
  'Content-Type': 'application/json',
  'X-Secret-Key': secret,
};

const decoder = new TextDecoder();
const eventSummaries = [];

async function readConfigValue(key) {
  const response = await fetch(`${baseUrl}/config/read`, {
    method: 'POST',
    headers,
    body: JSON.stringify({ key, is_secret: false }),
  });

  if (!response.ok) {
    throw new Error(`Failed to read config ${key}: ${response.status}`);
  }

  const parsed = await response.json();
  if (parsed == null) {
    return '';
  }
  if (typeof parsed === 'object' && Object.prototype.hasOwnProperty.call(parsed, 'masked_value')) {
    return String(parsed.masked_value ?? '');
  }
  return String(parsed);
}

function extractText(message) {
  if (!message || !Array.isArray(message.content)) {
    return '';
  }

  return message.content
    .filter((part) => part?.type === 'text' && typeof part.text === 'string')
    .map((part) => part.text)
    .join('');
}

async function createSession() {
  const response = await fetch(`${baseUrl}/agent/start`, {
    method: 'POST',
    headers,
    body: JSON.stringify({ working_dir: workingDir }),
  });

  if (!response.ok) {
    throw new Error(`Failed to create session: ${response.status} ${await response.text()}`);
  }

  return response.json();
}

async function postReply(sessionId, requestId, text) {
  const response = await fetch(`${baseUrl}/sessions/${sessionId}/reply`, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      request_id: requestId,
      user_message: {
        id: `msg-${requestId}`,
        role: 'user',
        created: Math.floor(Date.now() / 1000),
        content: [{ type: 'text', text }],
        metadata: { userVisible: true, agentVisible: true },
      },
    }),
  });

  if (!response.ok) {
    throw new Error(`Failed to submit reply: ${response.status} ${await response.text()}`);
  }
}

async function readReplyEvents(sessionId, requestId) {
  const controller = new AbortController();
  const timeout = setTimeout(
    () => controller.abort(new Error('Timed out waiting for chat reply')),
    120000
  );
  const response = await fetch(`${baseUrl}/sessions/${sessionId}/events`, {
    headers: { 'X-Secret-Key': secret },
    signal: controller.signal,
  });

  if (!response.ok || !response.body) {
    clearTimeout(timeout);
    throw new Error(`Failed to open session events: ${response.status}`);
  }

  const reader = response.body.getReader();
  let buffer = '';
  let assistantText = '';
  let inferenceModel = '';
  let finishReason = '';

  try {
    while (true) {
      const { value, done } = await reader.read();
      if (done) {
        break;
      }

      buffer += decoder.decode(value, { stream: true });
      let separatorIndex = buffer.indexOf('\n\n');

      while (separatorIndex !== -1) {
        const chunk = buffer.slice(0, separatorIndex);
        buffer = buffer.slice(separatorIndex + 2);
        separatorIndex = buffer.indexOf('\n\n');

        const data = chunk
          .split('\n')
          .filter((line) => line.startsWith('data: '))
          .map((line) => line.slice(6))
          .join('\n')
          .trim();

        if (!data) {
          continue;
        }

        const event = JSON.parse(data);
        if (event.chat_request_id && event.chat_request_id !== requestId) {
          continue;
        }

        if (eventSummaries.length < 50) {
          eventSummaries.push({
            type: event.type,
            role: event.message?.role,
            contentTypes: Array.isArray(event.message?.content)
              ? event.message.content.map((part) => part?.type ?? 'unknown')
              : [],
            inferenceModel: event.message?.metadata?.inference?.model,
            finishReason: event.reason,
            error: event.error,
          });
        }

        if (event.type === 'Error') {
          throw new Error(event.error || 'Unknown chat error');
        }

        if (event.type === 'Message') {
          assistantText += extractText(event.message);
          inferenceModel ||= event.message?.metadata?.inference?.model ?? '';
        }

        if (event.type === 'Finish') {
          finishReason = event.reason || '';
          return {
            assistantText: assistantText.trim(),
            inferenceModel,
            finishReason,
          };
        }
      }
    }
  } finally {
    clearTimeout(timeout);
    controller.abort();
  }

  throw new Error('Session events ended before a finish event was received');
}

const configuredProvider = await readConfigValue('GOOSE_PROVIDER');
const configuredModel = await readConfigValue('GOOSE_MODEL');
const configuredBaseUrl = await readConfigValue('OPENAI_BASE_URL');

let session;
let requestId;
let result;
let lastError;

for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
  eventSummaries.length = 0;

  try {
    session = await createSession();
    requestId = crypto.randomUUID();
    const replyPromise = readReplyEvents(session.id, requestId);
    await postReply(session.id, requestId, prompt);
    result = await replyPromise;

    if (/pong/i.test(result.assistantText)) {
      console.log(`attempt=${attempt}`);
      break;
    }

    lastError = new Error(
      `Unexpected assistant reply: ${result.assistantText || '[empty]'}; provider=${configuredProvider}; configured_model=${configuredModel}; configured_base_url=${configuredBaseUrl}; finish_reason=${result.finishReason || '[missing]'}; events=${JSON.stringify(eventSummaries)}`
    );
  } catch (error) {
    lastError = error;
  }
}

if (!result || !/pong/i.test(result.assistantText)) {
  throw lastError ?? new Error('Chat request failed without a result');
}

console.log('chat=ok');
console.log(`session_id=${session.id}`);
console.log(`request_id=${requestId}`);
console.log(`provider=${configuredProvider}`);
console.log(`configured_model=${configuredModel}`);
console.log(`configured_base_url=${configuredBaseUrl}`);
if (result.inferenceModel) {
  console.log(`inference_model=${result.inferenceModel}`);
}
console.log(`assistant_text=${JSON.stringify(result.assistantText)}`);
