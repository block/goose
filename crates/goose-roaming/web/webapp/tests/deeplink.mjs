// LIVE test of the desktop-QR deep link: the desktop Settings → Sharing QR
// encodes `<web-app-url>#card=goose+roam://…`. Scanning it opens the web app
// with the host card in the URL fragment; the app must connect without the
// user pasting anything.
//
// Two cases, both against a real share over the managed relays:
//   A. UNPAIRED browser opens the deep link -> the app tries to connect, the
//      host rejects (not_allowlisted), and the pairing panel is on screen
//      with this browser's own card ready to accept — the "first scan" UX.
//   B. The host accepts the browser key; a fresh page opens the same deep
//      link -> lands at "session ready" with zero manual input — the
//      "already paired" UX.
// Also asserts the fragment is scrubbed from the address bar (the card should
// never linger in history / be shoulder-surfable).
import { chromium } from "playwright";
import { spawn, execFileSync } from "node:child_process";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const BASE = process.env.SMOKE_URL ?? "http://localhost:5178";
const GOOSE = process.env.GOOSE_BIN ?? "/Users/micn/Development/goose/target/debug/goose";
const hostCwd = mkdtempSync(join(tmpdir(), "roam-deeplink-"));

const log = (...a) => console.log("  ", ...a);
const failures = [];
let share = null;
const testKeys = [];

function goose(args, opts = {}) {
  return execFileSync(GOOSE, args, { encoding: "utf8", timeout: 30000, ...opts });
}

function startShare() {
  return new Promise((resolve, reject) => {
    console.log(`\n[1] spawning: goose roam share --cwd ${hostCwd}`);
    share = spawn(GOOSE, ["roam", "share", "--cwd", hostCwd], {
      env: { ...process.env, GOOSE_DISABLE_KEYRING: "1" },
      stdio: ["ignore", "pipe", "pipe"],
    });
    let hostCard = null;
    let live = false;
    const t = setTimeout(() => reject(new Error("share did not go live within 40s")), 40000);
    const maybe = () => {
      if (live && hostCard) {
        clearTimeout(t);
        resolve(hostCard);
      }
    };
    share.stdout.on("data", (d) => {
      for (const line of d.toString().split("\n")) {
        if (line.startsWith("goose+roam://")) hostCard = line.trim();
      }
      maybe();
    });
    share.stderr.on("data", (d) => {
      if (/roaming agent is live/.test(d.toString())) live = true;
      maybe();
    });
    share.on("exit", (code) => {
      if (!hostCard) reject(new Error(`share exited early (code ${code})`));
    });
  });
}

const browser = await chromium.launch({ headless: true, channel: "chrome" });

try {
  const hostCard = await startShare();
  log(`host card: ${hostCard.slice(0, 40)}…`);
  const deepLink = `${BASE}/#card=${hostCard}`;

  // --- A. unpaired browser opens the deep link --------------------------
  console.log("\n[2] UNPAIRED: open deep link, expect pairing panel");
  const ctxA = await browser.newContext();
  const pageA = await ctxA.newPage();
  await pageA.goto(deepLink);

  // The fragment must be scrubbed immediately.
  await pageA.waitForFunction(() => !window.location.hash.includes("goose+roam"), null, {
    timeout: 10000,
  });
  log("✓ FRAGMENT SCRUBBED from address bar");

  // The card field is pre-filled from the link (no paste needed)…
  const prefilled = await pageA.locator("#card-input").inputValue();
  if (prefilled !== hostCard) {
    failures.push(`card input not prefilled from deep link (got ${prefilled.slice(0, 30)})`);
  } else {
    log("✓ CARD PREFILLED from deep link");
  }

  // …the auto-dial is rejected (browser not accepted yet), and the pairing
  // card ("first time?") is visible so the user can complete pairing.
  pageA.on("console", (m) => log(`pageA| ${m.text().slice(0, 120)}`));
  try {
    await pageA.locator("#my-endpoint-id").waitFor({ state: "visible", timeout: 60000 });
  } catch (e) {
    const st = (await pageA.locator("#status").textContent().catch(() => "?"))?.trim();
    throw new Error(`pairing panel never opened; status="${st}": ${e}`);
  }
  const keyA = (await pageA.locator("#my-endpoint-id").innerText()).trim();
  const cardA = (await pageA.locator("#my-card").innerText()).trim();
  testKeys.push(keyA);
  log(`✓ PAIRING PANEL VISIBLE — browser key ${keyA.slice(0, 12)}…`);
  await ctxA.close();

  // --- B. accept the browser, fresh page opens the same deep link -------
  console.log("\n[3] accepting browser key on the host");
  goose(["roam", "peers", "accept", cardA, "deeplink-test"], {
    env: { ...process.env, GOOSE_DISABLE_KEYRING: "1" },
  });

  console.log("[4] PAIRED: fresh context opens deep link, expect session ready");
  const ctxB = await browser.newContext();
  const pageB = await ctxB.newPage();
  // Same roam identity as context A? No — fresh context = fresh localStorage
  // = fresh key. Accept THIS page's key instead: read it from the pairing
  // panel after the first (rejected) auto-dial, accept, then re-open.
  await pageB.goto(deepLink);
  await pageB.locator("#my-endpoint-id").waitFor({ state: "visible", timeout: 60000 });
  const keyB = (await pageB.locator("#my-endpoint-id").innerText()).trim();
  const cardB = (await pageB.locator("#my-card").innerText()).trim();
  testKeys.push(keyB);
  goose(["roam", "peers", "accept", cardB, "deeplink-test-b"], {
    env: { ...process.env, GOOSE_DISABLE_KEYRING: "1" },
  });
  // The app scrubbed the fragment, so this goto differs only by hash — the
  // browser would NOT reload and the boot effect wouldn't re-run. Bounce
  // through about:blank to force a real navigation (a phone re-scanning the
  // QR is always a fresh load).
  await pageB.goto("about:blank");
  await pageB.goto(deepLink);
  // Success = the workspace (session matrix + sidebar) is on screen: the app
  // dialed the host from the fragment alone and initialized ACP.
  await pageB.locator("#workspace").waitFor({ state: "visible", timeout: 90000 });
  const status = (await pageB.locator("#status").textContent())?.trim();
  log(`✓ CONNECTED via deep link alone — status: "${status}" — zero manual input`);
  await ctxB.close();
} catch (err) {
  failures.push(String(err));
} finally {
  for (const key of testKeys) {
    try {
      goose(["roam", "peers", "revoke", key], {
        env: { ...process.env, GOOSE_DISABLE_KEYRING: "1" },
      });
    } catch {
      /* best effort */
    }
  }
  share?.kill("SIGTERM");
  await browser.close();
}

if (failures.length) {
  console.error("\n✗ DEEPLINK FAILURES:");
  for (const f of failures) console.error("  -", f);
  process.exit(1);
}
console.log("\n✓✓ DEEP LINK VERIFIED — QR scan → browser → bound to this goose");
