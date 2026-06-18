#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";

const EXPECTED_APPS = ["ioc-toolbox", "encode-hash-lab", "secret-credential-scanner", "jwt-inspector"];
const LEGACY_APPS = ["clock", "chat"];

function fail(message) {
  throw new Error(message);
}

function listHtmlAppNames(appsDir) {
  if (!fs.existsSync(appsDir)) {
    fail(`Apps runtime directory is missing: ${appsDir}`);
  }

  return fs
    .readdirSync(appsDir)
    .filter((entry) => entry.endsWith(".html"))
    .map((entry) => entry.replace(/\.html$/u, ""))
    .sort();
}

function listCachedAppNames(cacheDir) {
  if (!fs.existsSync(cacheDir)) {
    return null;
  }

  const names = [];
  for (const entry of fs.readdirSync(cacheDir)) {
    if (!entry.endsWith(".json")) {
      continue;
    }

    const filePath = path.join(cacheDir, entry);
    const parsed = JSON.parse(fs.readFileSync(filePath, "utf8"));
    const name = parsed?.name;
    if (typeof name === "string" && name.length > 0) {
      names.push(name);
    }
  }

  return [...new Set(names)].sort();
}

function assertExactAppSet(names, subject) {
  const missing = EXPECTED_APPS.filter((expected) => !names.includes(expected));
  const extras = names.filter((name) => !EXPECTED_APPS.includes(name));
  const legacy = LEGACY_APPS.filter((name) => names.includes(name));

  if (missing.length > 0) {
    fail(`${subject} is missing expected app(s): ${missing.join(", ")}`);
  }

  if (legacy.length > 0) {
    fail(`${subject} still contains legacy app(s): ${legacy.join(", ")}`);
  }

  if (extras.length > 0) {
    fail(`${subject} contains unexpected app(s): ${extras.join(", ")}`);
  }
}

const rootArg = process.argv[2]?.trim();
const goosePathRoot = rootArg || process.env.GOOSE_PATH_ROOT;

if (!goosePathRoot) {
  fail("Usage: node scripts/check-security-apps-runtime.mjs <GOOSE_PATH_ROOT>");
}

const runtimeAppsDir = path.join(goosePathRoot, "data", "apps");
const cacheDir = path.join(goosePathRoot, "config", "mcp-apps-cache");

const runtimeApps = listHtmlAppNames(runtimeAppsDir);
const cachedApps = listCachedAppNames(cacheDir);

assertExactAppSet(runtimeApps, "apps runtime");
if (cachedApps) {
  assertExactAppSet(cachedApps, "apps cache");
}

console.log("apps_runtime_check=ok");
console.log(`goose_path_root=${goosePathRoot}`);
console.log(`runtime_apps=${runtimeApps.join(",")}`);
console.log(`cached_apps=${cachedApps ? cachedApps.join(",") : "not_initialized"}`);
