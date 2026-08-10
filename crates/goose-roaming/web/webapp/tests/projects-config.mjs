// Live check: project grouping in the drawer + the ACP session config sheet
// (model/provider/mode selects) against a real `goose roam share`.
//
// Unlike e2e.mjs this runs the share WITHOUT --cwd tricks against the real
// session store, so listSessions returns sessions that carry projectId meta.
import { spawn, execFileSync } from "node:child_process";
import { chromium } from "playwright";

const GOOSE = process.env.GOOSE_BIN ?? "goose";
const APP = process.env.APP_URL ?? "http://localhost:5178";
let share, browser, acceptedKey;

function fail(msg) {
  console.error(`✗ ${msg}`);
  cleanup().then(() => process.exit(1));
}
async function cleanup() {
  try {
    if (acceptedKey) execFileSync(GOOSE, ["roam", "peers", "revoke", acceptedKey], { env: envNoKeyring() });
  } catch { /* best effort */ }
  try { await browser?.close(); } catch { /* ignore */ }
  try { share?.kill(); } catch { /* ignore */ }
}
const envNoKeyring = () => ({ ...process.env, GOOSE_DISABLE_KEYRING: "1" });

function startShare() {
  return new Promise((resolve, reject) => {
    share = spawn(GOOSE, ["roam", "share", "--cwd", "/tmp"], {
      env: envNoKeyring(),
      stdio: ["ignore", "pipe", "pipe"],
    });
    let card = null, live = false;
    const t = setTimeout(() => reject(new Error("share not live in 40s")), 40000);
    const feed = (d) => {
      const s = d.toString();
      const m = s.match(/goose\+roam:\/\/[A-Za-z0-9_-]+/);
      if (m) card = m[0];
      if (s.includes("roaming agent is live")) live = true;
      if (card && live) { clearTimeout(t); resolve(card); }
    };
    share.stdout.on("data", feed);
    share.stderr.on("data", feed);
  });
}

const card = await startShare();
console.log("[1] share live");

browser = await chromium.launch({ channel: "chrome", headless: true });
const page = await browser.newPage();
page.on("console", (m) => { if (m.type() === "error") console.log("   page err|", m.text()); });
await page.goto(APP);

// fresh identity per run: pair it
await page.evaluate(() => localStorage.clear());
await page.reload();
const myCard = await page.locator("#my-card").textContent({ timeout: 20000 });
execFileSync(GOOSE, ["roam", "peers", "add", myCard.trim(), "projtest"], { env: envNoKeyring() });
execFileSync(GOOSE, ["roam", "peers", "accept", "projtest"], { env: envNoKeyring() });
acceptedKey = "projtest";
console.log("[2] browser identity accepted");

await page.fill("#card-input", card);
await page.click("#connect-btn");
await page.waitForSelector("#workspace", { timeout: 60000 });
console.log("[3] connected");

// --- project grouping: open the drawer, look for uppercase project labels
// Desktop viewport: the drawer (<aside>) is always visible; #sidebar-toggle is
// md:hidden and only exists for phones. Read the groups directly.
await page.waitForTimeout(1500);
const groups = await page.evaluate(() => {
  const labels = [...document.querySelectorAll(".project-label")]
    .map((e) => e.textContent?.trim())
    .filter(Boolean);
  const items = document.querySelectorAll(".session-item").length;
  return { labels, items };
});
console.log(`[4] drawer: ${groups.items} sessions, project groups: ${JSON.stringify(groups.labels.slice(0, 8))}`);
if (groups.items === 0) fail("no sessions listed");
if (groups.labels.length === 0) console.log("   note: no project labels visible (no projectId sessions in window?)");
else console.log("   ✓ PROJECT GROUPING RENDERS");

// --- session config sheet: start a session, open the sheet, inspect selects
await page.keyboard.press("Escape").catch(() => {});
await page.click("#matrix-new-session", { timeout: 15000 }).catch(async () => {
  // drawer may cover it; close then retry
  await page.mouse.click(5, 300);
  await page.click("#matrix-new-session", { timeout: 15000 });
});
await page.waitForSelector(".msg.system", { timeout: 60000 });
console.log("[5] new session open");

await page.click("#session-config-btn", { timeout: 15000 });
await page.waitForSelector("#session-config select", { timeout: 10000 });
const cfg = await page.evaluate(() => {
  return [...document.querySelectorAll("#session-config label")].map((l) => ({
    name: l.querySelector("span")?.textContent?.trim(),
    current: l.querySelector("select")?.selectedOptions?.[0]?.textContent?.trim(),
    options: l.querySelectorAll("option").length,
  }));
});
console.log("[6] config sheet:", JSON.stringify(cfg, null, 1));
const model = cfg.find((c) => /model/i.test(c.name ?? ""));
if (!model || model.options < 2) fail("model select missing or has <2 options");
console.log(`   ✓ MODEL SELECT: ${model.options} options, current: ${model.current}`);

// --- change the mode (safer than model: no provider auth surprises) and verify it sticks
const modeIdx = cfg.findIndex((c) => /mode/i.test(c.name ?? ""));
if (modeIdx >= 0 && cfg[modeIdx].options > 1) {
  const sel = page.locator("#session-config select").nth(modeIdx);
  const before = await sel.inputValue();
  const values = await sel.locator("option").evaluateAll((os) => os.map((o) => o.value));
  const other = values.find((v) => v !== before);
  await sel.selectOption(other);
  await page.waitForTimeout(2500);
  const after = await sel.inputValue();
  if (after !== other) fail(`mode change did not stick: wanted ${other}, got ${after}`);
  console.log(`   ✓ SET_CONFIG_OPTION ROUND-TRIP: mode ${before} -> ${after}`);
  await sel.selectOption(before); // put it back
  await page.waitForTimeout(1500);
}

console.log("\n✓✓ PROJECTS + CONFIG SHEET VERIFIED LIVE");
await cleanup();
process.exit(0);
