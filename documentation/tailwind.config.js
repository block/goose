// tailwind.config.js
module.exports = {
  content: ["./src/**/*.{js,jsx,ts,tsx}"],
  darkMode: ["class", '[data-theme="dark"]'],
  corePlugins: {
    preflight: false,
  },
  theme: {
    extend: {
      colors: {
        // Arcade colors
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
      },
      fontFamily: {
        sans: ['"Cash Sans"', "sans-serif"],
      },
    },
  },
  plugins: [],
};
