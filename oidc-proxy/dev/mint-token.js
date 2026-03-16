#!/usr/bin/env node
//
// Mint a signed OIDC token using the local dev keypair.
// The token is valid against the local issuer (dev/issuer.js).
//
// Usage:
//   node dev/mint-token.js                    # prints token to stdout
//   node dev/mint-token.js --ttl 3600         # custom TTL in seconds (default: 1200)
//   node dev/mint-token.js --repo other/repo  # override repository claim
//
// Example with goose:
//   ANTHROPIC_BASE_URL=http://localhost:8787 \
//   ANTHROPIC_API_KEY=$(node dev/mint-token.js) \
//   goose session

import { readFileSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { createSign } from "node:crypto";
import { parseArgs } from "node:util";

const __dirname = dirname(fileURLToPath(import.meta.url));
const KEYS_DIR = join(__dirname, ".keys");
const PRIVATE_KEY_PATH = join(KEYS_DIR, "private.pem");
const KID = "local-dev-key-001";

const { values } = parseArgs({
  options: {
    ttl: { type: "string", default: "1200" },
    repo: { type: "string", default: "block/goose" },
    ref: { type: "string", default: "refs/heads/main" },
    audience: { type: "string", default: "goose-oidc-proxy" },
    port: { type: "string", default: "8788" },
  },
});

if (!existsSync(PRIVATE_KEY_PATH)) {
  console.error(
    "No keypair found. Run the issuer first to generate one:\n  node dev/issuer.js",
  );
  process.exit(1);
}

const privateKey = readFileSync(PRIVATE_KEY_PATH, "utf-8");
const issuer = `http://localhost:${values.port}`;
const now = Math.floor(Date.now() / 1000);
const ttl = parseInt(values.ttl, 10);

function base64UrlEncode(data) {
  const str = typeof data === "string" ? data : JSON.stringify(data);
  return Buffer.from(str).toString("base64url");
}

const header = {
  alg: "RS256",
  typ: "JWT",
  kid: KID,
};

const payload = {
  iss: issuer,
  aud: values.audience,
  iat: now,
  exp: now + ttl,
  nbf: now,
  sub: "repo:block/goose:ref:refs/heads/main",
  repository: values.repo,
  repository_owner: values.repo.split("/")[0],
  ref: values.ref,
  sha: "0000000000000000000000000000000000000000",
  workflow: "local-dev",
  actor: "local-developer",
  event_name: "workflow_dispatch",
  run_id: "0",
  run_number: "0",
  run_attempt: "1",
  job_workflow_ref: `${values.repo}/.github/workflows/local-dev.yml@${values.ref}`,
};

const headerB64 = base64UrlEncode(header);
const payloadB64 = base64UrlEncode(payload);

const signer = createSign("RSA-SHA256");
signer.update(`${headerB64}.${payloadB64}`);
const signature = signer.sign(privateKey, "base64url");

const token = `${headerB64}.${payloadB64}.${signature}`;

// Print just the token — easy to use in $() substitution
process.stdout.write(token);
