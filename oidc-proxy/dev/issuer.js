#!/usr/bin/env node
//
// Local OIDC issuer — serves discovery + JWKS so the proxy can validate
// locally-minted tokens. Run this alongside `wrangler dev`.
//
// Usage:
//   node dev/issuer.js
//   # → Serving OIDC issuer on http://localhost:8788
//
// The keypair is written to dev/.keys/ on first run and reused after that,
// so tokens minted by dev/mint-token.js will validate across restarts.

import { createServer } from "node:http";
import { readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { generateKeyPairSync, createPublicKey } from "node:crypto";

const __dirname = dirname(fileURLToPath(import.meta.url));
const KEYS_DIR = join(__dirname, ".keys");
const PRIVATE_KEY_PATH = join(KEYS_DIR, "private.pem");
const PUBLIC_KEY_PATH = join(KEYS_DIR, "public.pem");
const PORT = parseInt(process.env.ISSUER_PORT || "8788", 10);
const ISSUER = `http://localhost:${PORT}`;
const KID = "local-dev-key-001";

function ensureKeypair() {
  mkdirSync(KEYS_DIR, { recursive: true });

  if (existsSync(PRIVATE_KEY_PATH) && existsSync(PUBLIC_KEY_PATH)) {
    console.log("Using existing keypair from dev/.keys/");
    return;
  }

  console.log("Generating new RSA keypair in dev/.keys/");
  const { publicKey, privateKey } = generateKeyPairSync("rsa", {
    modulusLength: 2048,
    publicKeyEncoding: { type: "spki", format: "pem" },
    privateKeyEncoding: { type: "pkcs8", format: "pem" },
  });
  writeFileSync(PRIVATE_KEY_PATH, privateKey);
  writeFileSync(PUBLIC_KEY_PATH, publicKey);
}

function getPublicJwk() {
  const pem = readFileSync(PUBLIC_KEY_PATH, "utf-8");
  const key = createPublicKey(pem);
  const jwk = key.export({ format: "jwk" });
  return {
    ...jwk,
    kid: KID,
    alg: "RS256",
    use: "sig",
  };
}

ensureKeypair();
const publicJwk = getPublicJwk();

const routes = {
  "/.well-known/openid-configuration": () => ({
    issuer: ISSUER,
    jwks_uri: `${ISSUER}/.well-known/jwks`,
    response_types_supported: ["id_token"],
    subject_types_supported: ["public"],
    id_token_signing_alg_values_supported: ["RS256"],
  }),
  "/.well-known/jwks": () => ({
    keys: [publicJwk],
  }),
};

const server = createServer((req, res) => {
  const handler = routes[req.url];
  if (handler) {
    const body = JSON.stringify(handler());
    res.writeHead(200, {
      "Content-Type": "application/json",
      "Access-Control-Allow-Origin": "*",
    });
    res.end(body);
    console.log(`  ${req.method} ${req.url} → 200`);
  } else {
    res.writeHead(404);
    res.end("Not found");
    console.log(`  ${req.method} ${req.url} → 404`);
  }
});

server.listen(PORT, () => {
  console.log(`\nOIDC issuer running on ${ISSUER}`);
  console.log(`  Discovery: ${ISSUER}/.well-known/openid-configuration`);
  console.log(`  JWKS:      ${ISSUER}/.well-known/jwks`);
  console.log(`\nSet in .dev.vars:`);
  console.log(`  OIDC_ISSUER=${ISSUER}`);
  console.log();
});
