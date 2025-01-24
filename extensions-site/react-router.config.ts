import type { Config } from "@react-router/dev/config";

const basename = process.env.VITE_BASENAME || "";

export default {
  basename,
  ssr: false,
} satisfies Config;
