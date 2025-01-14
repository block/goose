/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ["class"],
  content: ["./src/**/*.{js,jsx,ts,tsx}", "./index.html"],
  plugins: [require("tailwindcss-animate"), require("@tailwindcss/typography")],
  theme: {
    extend: {
      keyframes: {
        shimmer: {
          "0%": { backgroundPosition: "200% 0" },
          "100%": { backgroundPosition: "-200% 0" },
        },
      },
      animation: {
        "shimmer-pulse": "shimmer 4s ease-in-out infinite",
      },
      typography: {
        xxs: {
          css: {
            fontSize: "10px",
          },
        },
        xs: {
          css: {
            fontSize: "12px",
            h1: {
              fontSize: "1.5em",
            },
            h2: {
              fontSize: "1.25em",
            },
            h3: {
              fontSize: "1.125em",
            },
            h4: {
              fontSize: "1em",
            },
          },
        },
      },
      spacing: {
        "8": "8px",
        "10": "10px",
        "16": "16px",
      },
      margin: {
        "10": "10px",
        "50": "50px",
      },
      backgroundImage: {
        "prev-goose-gradient":
          "linear-gradient(89deg, rgba(85, 95, 231, 0.04) 0.16%, rgba(85, 95, 231, 0.20) 99.77%)",
        "dark-prev-goose-gradient":
          "linear-gradient(89deg, rgb(147 149 151 / 4%) 0.16%, rgb(55 65 81 / 35%) 99.77%)",
        "card-gradient":
          "linear-gradient(359deg, rgba(255, 255, 255, 0.90) 9.96%, rgba(226, 245, 251, 0.90) 95.35%)",
        "dark-card-gradient":
          "linear-gradient(359deg, rgba(31, 41, 55, 0.90) 9.96%, rgba(17, 24, 39, 0.90) 95.35%)",
        "window-gradient":
          "linear-gradient(90deg, rgba(255, 255, 255, 0.55) 0%, rgba(246, 247, 255, 0.55) 100%)",
        "dark-window-gradient":
          "linear-gradient(90deg, rgba(50, 50, 50, 0.55) 0%, rgba(89, 89, 89, 0.55) 100%)",
      },
      fontSize: {
        14: "14px",
      },
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

        background: "var(--background)",
        foreground: "var(--foreground)",

        "splash-pills": {
          DEFAULT: "rgba(255, 255, 255, 0.60)",
          dark: "rgba(31, 41, 55, 0.60)",
        },
        "splash-pills-text": {
          DEFAULT: "rgba(0, 0, 0, 0.60)",
          dark: "rgba(255, 255, 255, 0.60)",
        },

        "prev-goose-text": {
          DEFAULT: "#4E52C5",
          dark: "#9CA3AF",
        },

        "inline-code": {
          DEFAULT: "rgba(255, 255, 255, 0.5)",
          dark: "rgba(57, 33, 0, 0.49)",
        },

        "more-menu": {
          DEFAULT: "rgba(255, 255, 255, 0.95)",
          dark: "rgba(31, 41, 55, 0.95)",
        },

        "bottom-menu": {
          DEFAULT: "rgba(0, 0, 0, 0.35)",
          dark: "rgba(255, 255, 255, 0.35)",
        },
        "bottom-menu-background": {
          DEFAULT: "rgba(255, 255, 255, 0.90)",
          dark: "rgba(31, 41, 55, 0.35)",
        },
        "tool-bold": {
          DEFAULT: "rgba(0, 0, 0, 0.85)",
          dark: "rgba(255, 255, 255, 0.65)",
        },
        tool: {
          DEFAULT: "rgba(0, 0, 0, 0.75)",
          dark: "rgba(255, 255, 255, 0.50)",
        },

        "tool-dim": {
          DEFAULT: "rgba(0, 0, 0, 0.6)",
          dark: "rgba(255, 255, 255, 0.50)",
        },

        "tool-result-green": {
          DEFAULT: "#028E00",
          dark: "#10B981",
        },
        "tool-card": {
          DEFAULT: "rgba(255, 255, 255, 0.80)",
          dark: "rgba(31, 41, 55, 0.80)",
        },
        "link-preview": {
          DEFAULT: "rgba(255, 255, 255, 0.80)",
          dark: "rgba(31, 41, 55, 0.80)",
        },
        "user-bubble": {
          DEFAULT: "rgba(85, 95, 231, 0.90)",
          dark: "rgba(129, 140, 248, 0.90)",
        },
        "goose-bubble": {
          DEFAULT: "rgba(0, 0, 0, 0.12)",
          dark: "rgba(255, 255, 255, 0.12)",
        },
        "goose-bubble-text": {
          DEFAULT: "rgba(255, 255, 255, 1.00)",
          dark: "rgba(0, 0, 0, 1.00)",
        },
        "logo-primary": {
          DEFAULT: "#000000",
          dark: "#9d9d9d",
        },
        "goose-text": {
          DEFAULT: "#000000",
          dark: "#9d9d9d",
        },
        "goose-text-light": {
          DEFAULT: "#FFFFFF",
          dark: "#2F2F2F",
        },
      },
      borderRadius: {
        lg: "var(--radius)",
        md: "calc(var(--radius) - 2px)",
        sm: "calc(var(--radius) - 4px)",
      },
      // will enable cash sans along with style updates
      // fontFamily: {
      //   sans: ["Cash Sans"],
      //   mono: ["Cash Sans Mono"],
      // },
    },
  },
};
