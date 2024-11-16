/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ["class"],
  content: [
    "./src/**/*.{js,jsx,ts,tsx}",
    "./index.html",
  ],
  theme: {
    extend: {
      margin: {
        '10': '10px',
        '50': '50px',
      },
      backgroundImage: {
        'card-gradient': 'linear-gradient(28.59deg, rgba(255, 255, 255, 0.9) 5.64%, rgba(226, 245, 251, 0.9) 94.49%);',
        'window-gradient': 'linear-gradient(90deg, rgba(255, 255, 255, 0.55) 0%, rgba(246, 247, 255, 0.55) 100%);'
      },
      colors: {
        background: 'var(--background)',
        foreground: 'var(--foreground)',
        tab: '#E9F7FA',

        'tool-bold': 'rgba(0, 0, 0, 0.65);',
        'tool': 'rgba(0, 0, 0, 0.50);',

        'tool-result-green': '#028E00;',
        'tool-card': 'rgba(255, 255, 255, 0.80);',
        'user-bubble': 'rgba(85, 95, 231, 0.90);',
        'goose-bubble': 'rgba(0, 0, 0, 0.12);'
      },
      borderRadius: {
        lg: 'var(--radius)',
        md: 'calc(var(--radius) - 2px)',
        sm: 'calc(var(--radius) - 4px)'
      }
    }
  },
  plugins: [require("tailwindcss-animate")],
}