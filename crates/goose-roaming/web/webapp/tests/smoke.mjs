// Runtime smoke test: proves the roaming wasm actually RUNS in a real browser
// (not just compiles). Drives headless Chromium via Playwright against the Vite
// dev server and exercises the wasm module directly:
//
//   1. wasm loads + instantiates in-browser
//   2. RoamClient() generates an ed25519 keypair  -> proves getrandom wasm_js
//      backend + ed25519-dalek run in wasm at runtime
//   3. a goose+roam:// card round-trips through decodeCardEndpointId()
//      -> proves base64 + serde_json + iroh EndpointId parsing in wasm
//
// It deliberately does NOT require a live `goose roam share` (no accepted key,
// no host). That end-to-end round trip is the next step; this isolates "does
// the browser wasm run".
import { chromium } from "playwright";

const BASE = process.env.SMOKE_URL ?? "http://localhost:5178";
const failures = [];
const log = (...a) => console.log("  ", ...a);

// Use the system-installed Google Chrome (channel) rather than Playwright's
// bundled chromium, which may be a version mismatch on this machine.
const browser = await chromium.launch({ headless: true, channel: "chrome" });
const page = await browser.newPage();

const consoleErrors = [];
page.on("console", (m) => {
  if (m.type() === "error") consoleErrors.push(m.text());
});
page.on("pageerror", (e) => consoleErrors.push(`pageerror: ${e.message}`));

try {
  console.log(`\nnavigating to ${BASE}`);
  await page.goto(BASE, { waitUntil: "networkidle", timeout: 30000 });

  // 1 + 2: the app itself instantiates wasm and shows this browser's key.
  const shownId = await page
    .locator("#my-endpoint-id")
    .textContent({ timeout: 20000 });
  const hexRe = /^[0-9a-f]{64}$/;
  if (shownId && hexRe.test(shownId.trim())) {
    log(`✓ wasm ran; RoamClient generated key ${shownId.slice(0, 16)}…`);
  } else {
    failures.push(`endpoint id not a 64-char hex key: ${JSON.stringify(shownId)}`);
  }

  // 3: round-trip a card built from that (valid) public key through the wasm
  // decoder, entirely in the page.
  const result = await page.evaluate(async (relayUrl) => {
    const m = await import("/src/wasm/goose_roaming_web.js");
    await m.default();
    const client = new m.RoamClient(undefined);
    const id = client.endpointId();

    // Build a goose+roam:// card the same way goose-roaming encodes: URL-safe
    // base64 (no pad) of JSON { version, endpoint_id, relay_urls }.
    const json = JSON.stringify({ version: 1, endpoint_id: id, relay_urls: [relayUrl] });
    const b64 = btoa(String.fromCharCode(...new TextEncoder().encode(json)))
      .replace(/\+/g, "-")
      .replace(/\//g, "_")
      .replace(/=+$/, "");
    const card = `goose+roam://${b64}`;

    const decodedId = m.decodeCardEndpointId(card);
    // secretHex round-trip: restoring from hex yields the same public key.
    const restored = new m.RoamClient(client.secretHex());
    return { id, decodedId, restoredId: restored.endpointId() };
  }, "https://usw1-2.relay.michaelneale.mesh-llm.iroh.link./");

  if (result.decodedId === result.id) {
    log(`✓ card decode round-trip: ${result.decodedId.slice(0, 16)}…`);
  } else {
    failures.push(
      `card round-trip mismatch: made ${result.id} decoded ${result.decodedId}`,
    );
  }
  if (result.restoredId === result.id) {
    log(`✓ identity persistence round-trip (secretHex → same key)`);
  } else {
    failures.push(
      `secretHex round-trip mismatch: ${result.id} vs ${result.restoredId}`,
    );
  }

  const realErrors = consoleErrors.filter(
    (e) => !/favicon|Failed to load resource.*404/i.test(e),
  );
  if (realErrors.length) {
    failures.push(`console errors:\n    ${realErrors.join("\n    ")}`);
  } else {
    log("✓ no console/page errors");
  }
} catch (err) {
  failures.push(`exception: ${err?.stack ?? err}`);
} finally {
  await browser.close();
}

if (failures.length) {
  console.error(`\n✗ SMOKE FAILED (${failures.length}):`);
  for (const f of failures) console.error(`  - ${f}`);
  process.exit(1);
}
console.log("\n✓ SMOKE PASSED — roaming wasm runs in a real browser\n");
