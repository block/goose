import type { Config } from "tailwindcss";
import { COLORS } from "./styles/colors";

const config = {
	darkMode: ["class"],
	content: [
		"./pages/**/*.{ts,tsx}",
		"./components/**/*.{ts,tsx}",
		"./app/**/*.{ts,tsx}",
		"./src/**/*.{ts,tsx}",
	],
	prefix: "",
	theme: {
		container: {
			center: true,
			padding: "2rem",
			screens: {
				"2xl": "1400px",
			},
		},
		extend: {
			colors: {
				...COLORS,
			},
			boxShadow: {
        'inset-card': 'oklch(92.3% 0.003 48.717) 0px 3px 12px inset',
        'inset-card-dark': 'oklch(26.8% 0.007 34.298) 0px 3px 12px inset',
      },
			keyframes: {
				"accordion-down": {
					from: {
						height: "0",
					},
					to: {
						height: "var(--radix-accordion-content-height)",
					},
				},
				"accordion-up": {
					from: {
						height: "var(--radix-accordion-content-height)",
					},
					to: {
						height: "0",
					},
				},
				"collapsible-down": {
					from: {
						height: "0",
					},
					to: {
						height: "var(--radix-collapsible-content-height)",
					},
				},
				"collapsible-up": {
					from: {
						height: "var(--radix-collapsible-content-height)",
					},
					to: {
						height: "0",
					},
				},
			},
			animation: {
				"collapsible-down": "collapsible-down 0.2s ease-out",
				"collapsible-up": "collapsible-up 0.2s ease-out",
				"accordion-down": "accordion-down 0.2s ease-out",
				"accordion-up": "accordion-up 0.2s ease-out",
			},
		},
	},
	plugins: [require("tailwindcss-animate")],
} satisfies Config;

export default config;
