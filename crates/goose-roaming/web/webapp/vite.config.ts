import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath } from "node:url";

// This app reuses goose's reference clients directly from the repo:
//  - @aaif/goose-sdk (ui/sdk): GooseClient — protocol/transport layer
//  - @desktop (ui/desktop/src): real desktop components (MarkdownContent,
//    ToolCallStatusIndicator, …) imported as source; @vitejs/plugin-react
//    compiles their JSX, tailwind scans them for classes (via theme.css),
//    and a tiny window.electron shim covers the desktop-only APIs.
const repoRoot = fileURLToPath(new URL("../../../../", import.meta.url));
const gooseSdk = fileURLToPath(
  new URL("../../../../ui/sdk/dist/index.js", import.meta.url),
);
const desktopSrc = fileURLToPath(
  new URL("../../../../ui/desktop/src", import.meta.url),
);

const buildStamp = `${new Date().toISOString().slice(0, 16).replace("T", " ")}Z`;

export default defineConfig({
  root: ".",
  define: {
    __BUILD_STAMP__: JSON.stringify(buildStamp),
  },
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@aaif/goose-sdk": gooseSdk,
      "@desktop": desktopSrc,
    },
    // One shared copy across app + SDK + desktop sources.
    dedupe: ["react", "react-dom", "@agentclientprotocol/sdk", "zod", "react-intl"],
  },
  server: {
    port: 5178,
    fs: { allow: [repoRoot] },
  },
  build: {
    target: "esnext",
    outDir: "dist",
  },
  assetsInclude: ["**/*.wasm"],
});
