import type { Config } from "@react-router/dev/config";

const basename = process.env.REACT_APP_BASENAME || "";

export default {
  basename,
  ssr: false,
} satisfies Config;
