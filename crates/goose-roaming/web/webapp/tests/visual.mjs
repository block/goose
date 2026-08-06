// Visual capture: connect, send a prompt that exercises markdown + a tool call,
// and screenshot the rendered GUI so we can eyeball that it looks like a real
// chat app (not just that the plumbing works).
import { chromium } from "playwright";
import { spawn, execFileSync } from "node:child_process";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const BASE = "http://localhost:5178";
const GOOSE = process.env.GOOSE_BIN ?? "/Users/micn/Development/goose/target/debug/goose";
const hostCwd = mkdtempSync(join(tmpdir(), "roam-vis-"));
// seed a couple of files so a "list files" tool call has something to show
writeFileSync(join(hostCwd, "hello.txt"), "hi from roam\n");
writeFileSync(join(hostCwd, "notes.md"), "# notes\n- one\n- two\n");

const log = (...a) => console.log("  ", ...a);
let share = null, browserKey = null;

function startShare() {
  return new Promise((resolve, reject) => {
    share = spawn(GOOSE, ["roam", "share", "--cwd", hostCwd], { env: process.env, stdio: ["ignore", "pipe", "pipe"] });
    let card = null, live = false;
    const done = () => { if (live && card) resolve(card); };
    const t = setTimeout(() => reject(new Error("share timeout")), 40000);
    share.stdout.on("data", d => { for (const l of d.toString().split("\n")) if (l.startsWith("goose+roam://")) { card = l.trim(); clearTimeout(t); done(); } });
    share.stderr.on("data", d => { if (/roaming agent is live/.test(d.toString())) { live = true; clearTimeout(t); done(); } });
  });
}

const browser = await chromium.launch({ headless: true, channel: "chrome" });
const page = await browser.newPage({ viewport: { width: 1100, height: 820 }, deviceScaleFactor: 2 });
const consoleLines = [];
page.on("console", (m) => consoleLines.push(`console.${m.type()}: ${m.text()}`));
page.on("pageerror", (e) => consoleLines.push(`PAGEERROR: ${e.message}`));
// The permission UX is now an inline card (not window.confirm). Auto-click its
// allow (primary) button as soon as one appears, via a MutationObserver.
await page.addInitScript(() => {
  new MutationObserver(() => {
    const btn = document.querySelector(".perm .perm-actions button.primary");
    if (btn) btn.click();
  }).observe(document.documentElement, { childList: true, subtree: true });
});
try {
  const hostCard = await startShare();
  log("share live");
  await page.goto(BASE, { waitUntil: "networkidle" });
  const browserCard = (await page.locator("#my-card").textContent())?.trim();
  browserKey = (await page.locator("#my-endpoint-id").textContent())?.trim();
  execFileSync(GOOSE, ["roam", "peers", "accept", browserCard, "vis"], { encoding: "utf8" });
  log("accepted browser");

  await page.locator("#card-input").fill(hostCard);
  await page.locator("#connect-btn").click();
  // front page after connect is the session matrix — start a fresh session
  await page.click("#matrix-new-session", { timeout: 90000 });
  await page.locator("#workspace").waitFor({ state: "visible", timeout: 45000 });
  // Wait until the new session is actually ready (sessionId set, not busy),
  // signalled by the "say hello" system line — otherwise a send is dropped.
  await page.locator(".msg.system", { hasText: "say hello" }).first().waitFor({ timeout: 45000 });
  log("connected + session ready");

  // A prompt that exercises a tool call (→ inline permission card + tool widget)
  // AND markdown in the summary.
  await page.locator("#prompt-input").fill(
    "Run a shell command to list the files in the current directory, then reply " +
      "in markdown: a `##` heading, a bullet list of the files, and a fenced " +
      "```bash code block with the command you ran. Keep it brief.",
  );
  await page.locator("#send-btn").click();
  log("prompt sent; waiting for an agent message to render…");

  // Wait for an actual agent message to appear, then for the turn to settle.
  try {
    await page.locator(".msg.agent .body").first().waitFor({ timeout: 90000 });
  } catch {
    log("no agent message within 90s");
  }
  for (let i = 0; i < 60; i++) {
    await page.waitForTimeout(1000);
    const s = (await page.locator("#status").textContent())?.trim();
    if (s === "connected" && i > 2) break;
  }
  await page.waitForTimeout(1500);

  const shot = "/tmp/roam-gui.png";
  await page.screenshot({ path: shot, fullPage: true });
  log(`screenshot: ${shot}`);

  // dump what got rendered, for a text sanity check
  const counts = await page.evaluate(() => ({
    agentMsgs: document.querySelectorAll(".msg.agent").length,
    toolWidgets: document.querySelectorAll(".tool").length,
    codeBlocks: document.querySelectorAll(".msg.agent pre").length,
    lists: document.querySelectorAll(".msg.agent ul, .msg.agent ol").length,
    headings: document.querySelectorAll(".msg.agent h1,.msg.agent h2,.msg.agent h3").length,
    sessions: document.querySelectorAll(".session-item").length,
  }));
  log("rendered:", JSON.stringify(counts));
  if (counts.agentMsgs === 0) {
    log("--- console tail (agentMsgs=0, diagnosing) ---");
    for (const l of consoleLines.slice(-25)) log("   " + l);
    const logHtml = await page.evaluate(() => document.getElementById("log")?.innerHTML ?? "(no #log)");
    log("--- #log innerHTML (first 600 chars) ---");
    log("   " + logHtml.slice(0, 600));
  }
} catch (e) {
  log("EXCEPTION:", e.message);
  await page.screenshot({ path: "/tmp/roam-gui-err.png" }).catch(() => {});
} finally {
  await browser.close();
  if (browserKey) try { execFileSync(GOOSE, ["roam", "peers", "revoke", browserKey]); } catch {}
  if (share) share.kill("SIGINT");
}
