import { defineConfig } from "vite";
import { resolve } from "path";

export default defineConfig({
  resolve: {
    alias: {
      "@vibes.diy/use-vibes-base": resolve(__dirname, "./mocks/use-vibes-base.ts"),
    },
  },
});
