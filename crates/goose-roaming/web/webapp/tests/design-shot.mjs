// Screenshot helper for the design pass: connects to the local share and
// captures the main surfaces at phone + desktop sizes.
import { chromium } from "playwright";
import { execFileSync } from "node:child_process";
import fs from "node:fs";

const BASE = "http://localhost:5178";
const GOOSE = "/Users/micn/Development/goose/target/debug/goose";
const OUT = process.argv[2] ?? "/tmp/design-before";
fs.mkdirSync(OUT, { recursive: true });

// host card from the running share's identity via serve.json fallback: use roam id
const card = execFileSync(GOOSE, ["roam", "id"], { encoding: "utf8", env: { ...process.env, GOOSE_DISABLE_KEYRING: "1" } })
  .split("\n").find(l => l.startsWith("goose+roam://"))?.trim();

const browser = await chromium.launch({ headless: true, channel: "chrome" });
for (const [label, vp] of [["phone", { width: 390, height: 844 }], ["desktop", { width: 1280, height: 800 }]]) {
  const ctx = await browser.newContext({ viewport: vp, deviceScaleFactor: 2 });
  const page = await ctx.newPage();
  await page.goto(BASE);
  await page.waitForTimeout(1500);
  await page.screenshot({ path: `${OUT}/${label}-onboarding.png` });
  // connect to real host
  if (card) {
    const input = page.locator("#card-input");
    if (await input.count()) {
      await input.fill(card);
      await page.locator("#connect-btn").click();
      // First dial is rejected (fresh browser identity): accept the key, retry.
      const keyEl = page.locator("#my-endpoint-id");
      try {
        await keyEl.waitFor({ state: "visible", timeout: 30000 });
        const key = (await keyEl.innerText()).trim();
        const myCard = (await page.locator("#my-card").innerText()).trim();
        execFileSync(GOOSE, ["roam", "peers", "accept", myCard, `design-${label}`], { env: { ...process.env, GOOSE_DISABLE_KEYRING: "1" } });
        await page.locator("#connect-btn").click();
      } catch { /* already paired */ }
      await page.locator("#workspace").waitFor({ state: "visible", timeout: 60000 }).catch(() => {});
      await page.waitForTimeout(1200);
      await page.screenshot({ path: `${OUT}/${label}-workspace.png` });
      // open sidebar on phone
      if (label === "phone") {
        const t = page.locator("#sidebar-toggle");
        if (await t.isVisible().catch(() => false)) { await t.click(); await page.waitForTimeout(500); await page.screenshot({ path: `${OUT}/${label}-sidebar.png` }); await t.click(); }
      }
      // open a session
      const row = page.locator("aside .session-row, aside button").filter({ hasText: /./ }).nth(2);
      const rows = page.locator("aside button");
      if (await rows.count() > 3) {
        await rows.nth(3).click();
        await page.waitForTimeout(2500);
        await page.screenshot({ path: `${OUT}/${label}-chat.png` });
      }
    }
  }
  await ctx.close();
}
await browser.close();
console.log("shots in", OUT);
