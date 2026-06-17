#!/usr/bin/env node

const baseUrl = process.env.SECURITY_CHAT_BASE_URL;
const secret = process.env.SECURITY_CHAT_SECRET;
const sessionId = process.env.SECURITY_APPS_SESSION_ID;

if (!baseUrl || !secret || !sessionId) {
  throw new Error("Missing SECURITY_CHAT_BASE_URL, SECURITY_CHAT_SECRET, or SECURITY_APPS_SESSION_ID");
}

const EXPECTED_APPS = ["ioc-toolbox", "encode-hash-lab", "secret-credential-scanner", "jwt-inspector"];
const LEGACY_APPS = ["clock", "chat"];

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
