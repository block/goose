/* eslint-env node */

/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        bgApp: 'var(--bg-app)',
        textProminent: 'var(--text-prominent)',
      },
    },
  },
  plugins: [],
};
