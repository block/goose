// Mobile-viewport overflow hunt: connect to a throwaway share, generate wide
// content, then list every element wider than the viewport.
import { spawn, execFileSync } from "node:child_process";
import { chromium } from "playwright";

const GOOSE = process.env.GOOSE_BIN ?? "goose";
const URL = "http://localhost:5178/";

const share = spawn(GOOSE, ["roam", "share", "--cwd", "/tmp"], {
  // GOOSE_DISABLE_KEYRING: never trigger a macOS keychain popup from tests
  env: { ...process.env, GOOSE_DISABLE_KEYRING: "1" },
  stdio: ["ignore", "pipe", "pipe"],
});
let hostCard = "";
share.stderr.on("data", (d) => process.stderr.write(`   share| ${d}`.slice(0, 200)));
const cardPromise = new Promise((res) => {
  share.stdout.on("data", (d) => {
    const m = String(d).match(/goose\+roam:\/\/\S+/);
    if (m) { hostCard = m[0]; res(); }
  });
});
await Promise.race([cardPromise, new Promise((_, rej) => setTimeout(() => rej(new Error("no card")), 30000))]);

const browser = await chromium.launch({ headless: true, channel: "chrome" });
const page = await browser.newPage({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true });
// clean state: no saved host/session
await page.addInitScript(() => localStorage.clear());
// auto-allow permission cards
await page.addInitScript(() => {
  const obs = new MutationObserver(() => {
    const btn = document.querySelector(".perm-actions button, .perm button");
    if (btn) btn.click();
  });
  obs.observe(document.documentElement, { childList: true, subtree: true });
});
await page.goto(URL, { waitUntil: "networkidle" });
await page.waitForTimeout(2000);
const myCard = (await page.locator("#my-card").textContent()).trim();
execFileSync(GOOSE, ["roam", "peers", "accept", myCard, "overflow-test"]);
await page.fill("#card-input", hostCard);
await page.click("#connect-btn");
  // front page after connect is the session matrix — start a fresh session
  await page.click("#matrix-new-session", { timeout: 90000 });
await page.waitForSelector("#workspace", { state: "visible", timeout: 60000 });
await page.waitForTimeout(2500);
await page.fill("#prompt-input", "Run this exact shell command: echo 'a-very-long-unbroken-token-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA'; ls -la /usr/lib | head -20. Then reply with: a markdown table with 4 columns (name, description, path, notes) and 3 rows, and a fenced bash code block containing one very long line: export SOME_VAR=abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnopqrstuvwxyz0123456789");
await page.click("#prompt-form button[type=submit]");
// wait for agent message + idle
await page.waitForSelector(".msg.agent .body", { timeout: 120000 });
let last = "", stable = 0;
for (let i = 0; i < 60 && stable < 4; i++) {
  await page.waitForTimeout(1500);
  const txt = await page.evaluate(() => document.querySelector("#log")?.textContent?.length ?? 0);
  if (String(txt) === last) stable++; else { last = String(txt); stable = 0; }
}
// enumerate overflowing elements
const report = await page.evaluate(() => {
  const vw = document.documentElement.clientWidth;
  const bad = [];
  document.querySelectorAll("*").forEach((el) => {
    const r = el.getBoundingClientRect();
    if (r.width > vw + 1 || r.right > vw + 1) {
      const cls = (el.className?.baseVal ?? el.className ?? "").toString().slice(0, 90);
      bad.push(`${el.tagName.toLowerCase()}.${cls} w=${Math.round(r.width)} right=${Math.round(r.right)}`);
    }
  });
  return { vw, docScrollW: document.documentElement.scrollWidth, logScrollW: document.querySelector("#log")?.scrollWidth, bad: bad.slice(0, 25) };
});
console.log(JSON.stringify(report, null, 1));
await page.screenshot({ path: "/tmp/roam-mobile-overflow.png", fullPage: false });
console.log("shot: /tmp/roam-mobile-overflow.png");
await browser.close();
try { execFileSync(GOOSE, ["roam", "peers", "revoke", "overflow-test"]); } catch {}
share.kill("SIGINT");
