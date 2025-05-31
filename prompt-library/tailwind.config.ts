import type { Config } from "tailwindcss";

export default {
  content: ["./app/**/{**,.client,.server}/**/*.{js,jsx,ts,tsx}"],
  darkMode: "class",
  safelist: ["dark"],
  theme: {
    extend: {
      colors: {
        // start arcade colors
        bgApp: "var(--background-app)",
        bgSubtle: "var(--background-subtle)",
        bgStandard: "var(--background-standard)",
        bgProminent: "var(--background-prominent)",

        borderSubtle: "var(--border-subtle)",
        borderStandard: "var(--border-standard)",

        textProminent: "var(--text-prominent)",
        textStandard: "var(--text-standard)",
        textSubtle: "var(--text-subtle)",
        textPlaceholder: "var(--text-placeholder)",

        iconProminent: "var(--icon-prominent)",
        iconStandard: "var(--icon-standard)",
        iconSubtle: "var(--icon-subtle)",
        // end arcade colors
      },
      fontFamily: {
        sans: ['"Cash Sans"'],
      },
    },
  },
  plugins: [],
} satisfies Config;
