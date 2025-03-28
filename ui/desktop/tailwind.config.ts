/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ['class'],
  content: ['./src/**/*.{js,jsx,ts,tsx}', './index.html'],
  plugins: [require('tailwindcss-animate'), require('@tailwindcss/typography')],
  theme: {
    extend: {
      fontFamily: {
        sans: ['Cash Sans', 'sans-serif'],
      },
      keyframes: {
        shimmer: {
          '0%': { backgroundPosition: '200% 0' },
          '100%': { backgroundPosition: '-200% 0' },
        },
        loader: {
          '0%': { left: 0, width: 0 },
          '50%': { left: 0, width: '100%' },
          '100%': { left: '100%', width: 0 },
        },
        popin: {
          from: { opacity: 0, transform: 'scale(0.95)' },
          to: { opacity: 1, transform: 'scale(1)' },
        },
        fadein: {
          '0%': { opacity: 0 },
          '100%': { opacity: 1 },
        },
        appear: {
          '0%': { opacity: 0, transform: 'translateY(12px)' },
          '100%': { opacity: 1, transform: 'translateY(0)' },
        },
        flyin: {
          '0%': { opacity: 0, transform: 'translate(-300%, 300%)' },
          '100%': { opacity: 1, transform: 'translate(0, 0)' },
        },
        wind: {
          '0%': { transform: 'translate(0, 0)' },
          '99.99%': { transform: 'translate(-100%, 100%)' },
          '100%': { transform: 'translate(0, 0)' },
        },
        rotate: {
          '0%': { transform: 'rotate(0deg)' },
          '100%': { transform: 'rotate(360deg)' },
        },
        'slide-in-right': {
          '0%': { transform: 'translateX(100%)' },
          '100%': { transform: 'translateX(0)' },
        },
        'slide-out-right': {
          '0%': { transform: 'translateX(0)' },
          '100%': { transform: 'translateX(100%)' },
        },
        'slide-in-left': {
          '0%': { transform: 'translateX(-100%)' },
          '100%': { transform: 'translateX(0)' },
        },
        'slide-out-left': {
          '0%': { transform: 'translateX(0)' },
          '100%': { transform: 'translateX(-100%)' },
        },
      },
      animation: {
        'shimmer-pulse': 'shimmer 4s ease-in-out infinite',
        'gradient-loader': 'loader 750ms ease-in-out infinite',
        'slide-in-right': 'slide-in-right 0.3s ease-out forwards',
        'slide-out-right': 'slide-out-right 0.3s ease-out forwards',
        'slide-in-left': 'slide-in-left 0.3s ease-out forwards',
        'slide-out-left': 'slide-out-left 0.3s ease-out forwards',
      },
      colors: {
        bgApp: 'var(--background-app)',
        bgSubtle: 'var(--background-subtle)',
        bgStandard: 'var(--background-standard)',
        bgProminent: 'var(--background-prominent)',
        bgAppInverse: 'var(--background-app-inverse)',
        bgStandardInverse: 'var(--background-standard-inverse)',

        borderSubtle: 'var(--border-subtle)',
        borderStandard: 'var(--border-standard)',

        textProminent: 'var(--text-prominent)',
        textStandard: 'var(--text-standard)',
        textSubtle: 'var(--text-subtle)',
        textPlaceholder: 'var(--text-placeholder)',
        textProminentInverse: 'var(--text-prominent-inverse)',

        iconProminent: 'var(--icon-prominent)',
        iconStandard: 'var(--icon-standard)',
        iconSubtle: 'var(--icon-subtle)',
        iconExtraSubtle: 'var(--icon-extra-subtle)',
        slate: 'var(--slate)',
        blockTeal: 'var(--block-teal)',
        blockOrange: 'var(--block-orange)',
      },
    },
  },
};
