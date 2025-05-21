/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  darkMode: 'media', // Enable dark mode and use the system preference
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