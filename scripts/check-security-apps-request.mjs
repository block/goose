#!/usr/bin/env node

const baseUrl = process.env.SECURITY_CHAT_BASE_URL;
const secret = process.env.SECURITY_CHAT_SECRET;
const workingDir = process.env.SECURITY_CHAT_WORKDIR;
let sessionId = process.env.SECURITY_APPS_SESSION_ID;

if (!baseUrl || !secret) {
  throw new Error("Missing SECURITY_CHAT_BASE_URL or SECURITY_CHAT_SECRET");
}

const EXPECTED_APPS = ["ioc-toolbox", "encode-hash-lab", "secret-credential-scanner", "jwt-inspector"];
const LEGACY_APPS = ["clock", "chat"];

async function createSession() {
  if (!workingDir) {
    throw new Error(
      "Missing SECURITY_CHAT_WORKDIR when SECURITY_APPS_SESSION_ID is not provided"
    );
  }

  const response = await fetch(`${baseUrl}/agent/start`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-Secret-Key": secret,
    },
    body: JSON.stringify({ working_dir: workingDir }),
  });

  if (!response.ok) {
    throw new Error(`Failed to create apps session: ${response.status} ${await response.text()}`);
  }

  const session = await response.json();
  if (!session?.id || typeof session.id !== "string") {
    throw new Error("Apps session response did not include a valid id");
  }

  return session.id;
}

if (!sessionId) {
  sessionId = await createSession();
}

const response = await fetch(`${baseUrl}/agent/list_apps?session_id=${encodeURIComponent(sessionId)}`, {
  headers: {
    "X-Secret-Key": secret,
  },
});

if (!response.ok) {
  throw new Error(`list_apps failed with status ${response.status}`);
}

const payload = await response.json();
const apps = Array.isArray(payload?.apps) ? payload.apps : [];
const names = [...new Set(apps.map((app) => app?.name).filter((name) => typeof name === "string"))].sort();

const missing = EXPECTED_APPS.filter((expected) => !names.includes(expected));
const extras = names.filter((name) => !EXPECTED_APPS.includes(name));
const legacy = LEGACY_APPS.filter((name) => names.includes(name));

if (missing.length > 0) {
  throw new Error(`Missing expected built-in app(s): ${missing.join(", ")}`);
}

if (legacy.length > 0) {
  throw new Error(`Legacy default app still exposed through list_apps: ${legacy.join(", ")}`);
}

if (extras.length > 0) {
  throw new Error(`Unexpected built-in app(s) exposed through list_apps: ${extras.join(", ")}`);
}

console.log("apps_api=ok");
console.log(`session_id=${sessionId}`);
console.log(`listed_apps=${names.join(",")}`);
