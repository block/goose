// LIVE end-to-end test: a real browser connecting to a real `goose roam share`
// over the managed iroh relays, then driving an ACP prompt.
//
// Orchestration (all real, no mocks):
//   1. spawn `goose roam share` (background) on a temp cwd -> capture its card
//      once it registers with a relay and prints "roaming agent is live"
//   2. launch headless system Chrome, load the web app (fresh roam identity)
//   3. read the browser's own card (#my-card) out of the page
//   4. `goose roam peers accept '<browser card>'` on the host (share re-reads
//      the allowlist per connection, so this takes effect live)
//   5. paste the host card into the app, click connect
//   6. assert the app reaches "session ready" (browser dialed the host THROUGH
//      the relay, completed the roam handshake, and opened an ACP session)
//   7. send a prompt, assert the agent streams a response back
//   8. clean up: revoke the test key, kill the share process
//
// This proves the whole point: iroh in the browser, relay-only, driving a goose
// roam agent. No Tauri, no local bridge.
import { chromium } from "playwright";
import { spawn, execFileSync } from "node:child_process";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const BASE = process.env.SMOKE_URL ?? "http://localhost:5178";
const GOOSE = process.env.GOOSE_BIN ?? "/Users/micn/Development/goose/target/debug/goose";
const hostCwd = mkdtempSync(join(tmpdir(), "roam-host-"));

const log = (...a) => console.log("  ", ...a);
const failures = [];
let share = null;
let browserKey = null;

function goose(args, opts = {}) {
  return execFileSync(GOOSE, args, { encoding: "utf8", timeout: 30000, ...opts });
}

// --- 1. spawn `goose roam share`, wait for it to go live + grab its card ----
function startShare() {
  return new Promise((resolve, reject) => {
    console.log(`\n[1] spawning: goose roam share --cwd ${hostCwd}`);
    share = spawn(GOOSE, ["roam", "share", "--cwd", hostCwd], {
      env: process.env,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let hostCard = null;
    let live = false;
    const t = setTimeout(
      () => reject(new Error("share did not go live within 40s")),
      40000,
    );
    share.stdout.on("data", (d) => {
      const s = d.toString();
      for (const line of s.split("\n")) {
        if (line.startsWith("goose+roam://")) {
          hostCard = line.trim();
          if (live && hostCard) {
            clearTimeout(t);
            resolve(hostCard);
          }
        }
      }
    });
    share.stderr.on("data", (d) => {
      const s = d.toString();
      if (/roaming agent is live/.test(s)) live = true;
      if (live && hostCard) {
        clearTimeout(t);
        resolve(hostCard);
      }
      for (const line of s.split("\n")) if (line.trim()) log(`share| ${line.trim()}`);
    });
    share.on("exit", (code) => {
      if (!hostCard) reject(new Error(`share exited early (code ${code})`));
    });
  });
}

const browser = await chromium.launch({ headless: true, channel: "chrome" });
const page = await browser.newPage();
const consoleErrors = [];
page.on("console", (m) => {
  const t = m.text();
  if (m.type() === "error") consoleErrors.push(t);
  if (/roam:/.test(t)) log(`page| ${t}`);
});
page.on("pageerror", (e) => consoleErrors.push(`pageerror: ${e.message}`));

try {
  const hostCard = await startShare();
  log(`✓ share live; host card ${hostCard.slice(0, 32)}…`);

  // --- 2 + 3. load app, read the browser's own card ------------------------
  console.log(`\n[2] loading web app ${BASE}`);
  await page.goto(BASE, { waitUntil: "networkidle", timeout: 30000 });
  browserKey = (await page.locator("#my-endpoint-id").textContent())?.trim();
  const browserCard = (await page.locator("#my-card").textContent())?.trim();
  if (!browserCard?.startsWith("goose+roam://")) {
    throw new Error(`bad browser card: ${browserCard}`);
  }
  log(`✓ browser identity ${browserKey.slice(0, 16)}…`);

  // --- 4. accept the browser's key on the host ----------------------------
  console.log(`\n[3] goose roam peers accept '<browser card>'`);
  const acceptOut = goose(["roam", "peers", "accept", browserCard, "web-e2e"]);
  log(`✓ ${acceptOut.trim().split("\n").pop()}`);

  // --- 5 + 6. paste host card, connect, wait for session ready ------------
  console.log(`\n[4] connecting browser -> host through the relay`);
  await page.locator("#card-input").fill(hostCard);
  await page.locator("#connect-btn").click();

  // #chat unhides and a "ready" system line appears once initialize +
  // newSession succeed over the roaming ACP stream.
  await page.locator("#chat").waitFor({ state: "visible", timeout: 45000 });
  await page
    .locator(".line.system", { hasText: "ready" })
    .first()
    .waitFor({ timeout: 45000 });
  const status = (await page.locator("#status").textContent())?.trim();
  log(`✓ CONNECTED THROUGH RELAY — status: "${status}"`);
  log("  (browser dialed host over iroh relay + completed roam handshake + opened ACP session)");

  // --- 7. drive a prompt, assert the agent responds -----------------------
  console.log(`\n[5] sending a prompt over the roaming ACP session`);
  await page.locator("#prompt-input").fill("Reply with exactly: roam works");
  await page.locator("#prompt-form button[type=submit]").click();

  const agentLine = page.locator(".line.agent .body").first();
  await agentLine.waitFor({ timeout: 90000 });
  // wait for the streamed text to settle
  let last = "";
  for (let i = 0; i < 30; i++) {
    await page.waitForTimeout(1000);
    const now = (await agentLine.textContent())?.trim() ?? "";
    if (now && now === last) break;
    last = now;
  }
  if (last) {
    log(`✓ AGENT RESPONDED: "${last}"`);
  } else {
    failures.push("no agent response text");
  }

  const realErrors = consoleErrors.filter((e) => !/favicon|404/i.test(e));
  if (realErrors.length) failures.push(`console errors:\n    ${realErrors.join("\n    ")}`);
} catch (err) {
  failures.push(`exception: ${err?.stack ?? err}`);
} finally {
  await page.screenshot({ path: "/tmp/roam-e2e.png" }).catch(() => {});
  await browser.close();
  if (browserKey) {
    try {
      goose(["roam", "peers", "revoke", browserKey]);
      log("cleaned up: revoked test key");
    } catch {}
  }
  if (share) share.kill("SIGINT");
}

if (failures.length) {
  console.error(`\n✗ E2E FAILED (${failures.length}):`);
  for (const f of failures) console.error(`  - ${f}`);
  console.error("  screenshot: /tmp/roam-e2e.png");
  process.exit(1);
}
console.log("\n✓✓ E2E PASSED — browser connected to goose roam over iroh and drove the agent\n");
