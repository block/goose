// main.tsx imports @desktop/i18n/compiled/en.json, which is generated (and
// gitignored) by the desktop's i18n:compile. On a clean checkout it doesn't
// exist and Vite fails to resolve it — compile it here if missing.
import { existsSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const desktopDir = join(here, "..", "..", "..", "..", "..", "ui", "desktop");
const compiledEn = join(desktopDir, "src", "i18n", "compiled", "en.json");

if (!existsSync(compiledEn)) {
  console.log("compiling desktop i18n catalog (missing compiled/en.json)...");
  execFileSync("node", [join(desktopDir, "scripts", "i18n-compile.js")], {
    stdio: "inherit",
    cwd: desktopDir,
  });
}
