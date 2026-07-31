import { defineConfig } from "vite";

// The wasm-bindgen web target ships a .wasm asset the JS loads via fetch().
// Vite serves /src/wasm/*.wasm as-is in dev and copies it in build. No wasm
// plugin needed for the `--target web` output (it fetches, doesn't import).
export default defineConfig({
  root: ".",
  server: {
    port: 5178,
    fs: {
      // allow serving the generated wasm from src/wasm
      allow: [".."],
    },
  },
  build: {
    target: "esnext",
    outDir: "dist",
  },
  // Ensure the .wasm is treated as an asset, not parsed as JS.
  assetsInclude: ["**/*.wasm"],
});
