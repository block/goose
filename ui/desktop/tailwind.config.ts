/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ["class"],
  content: [
    "./src/**/*.{js,jsx,ts,tsx}",
    "./index.html",
  ],
  plugins: [
    require("tailwindcss-animate"),
    require('@tailwindcss/typography')
  ],
  theme: {
    extend: {
      keyframes: {
        shimmer: {
          '0%': { backgroundPosition: '200% 0' },
          '100%': { backgroundPosition: '-200% 0' }
        }
      },
      animation: {
        'shimmer-pulse': 'shimmer 4s ease-in-out infinite',
      },
      typography: {
        xxs: {
          css: {
            fontSize: '10px'
          }
        },
        xs: {
          css: {
            fontSize: '12px',
            h1: {
              fontSize: '1.5em'
            },
            h2: {
              fontSize: '1.25em'
            },
            h3: {
              fontSize: '1.125em'
            },
            h4: {
              fontSize: '1em'
            }
          }
        }
      },
      spacing: {
        '8': '8px',
        '10': '10px',
        '16': '16px',
      },
      margin: {
        '10': '10px',
        '50': '50px'
      },
      backgroundImage: {
        'prev-goose-gradient': 'linear-gradient(89deg, rgba(85, 95, 231, 0.04) 0.16%, rgba(85, 95, 231, 0.20) 99.77%)',
        'card-gradient': 'linear-gradient(359deg, rgba(255, 255, 255, 0.90) 9.96%, rgba(226, 245, 251, 0.90) 95.35%)',
        'window-gradient': 'linear-gradient(90deg, rgba(255, 255, 255, 0.55) 0%, rgba(246, 247, 255, 0.55) 100%)'
      },
      fontSize: {
        14: '14px'
      },
      colors: {
        background: 'var(--background)',
        foreground: 'var(--foreground)',

        'splash-pills': 'rgba(255, 255, 255, 0.60)',
        'splash-pills-text': 'rgba(0, 0, 0, 0.60)',

        'prev-goose-text': '#4E52C5',

        'more-menu': 'rgba(255, 255, 255, 0.95))',

        'bottom-menu': 'rgba(0, 0, 0, 0.35)',

        'tool-bold': 'rgba(0, 0, 0, 0.85)',
        'tool': 'rgba(0, 0, 0, 0.75)',
        'tool-dim': 'rgba(0, 0, 0, 0.6)',

        'tool-result-green': '#028E00',
        'tool-card': 'rgba(255, 255, 255, 0.80)',
        'link-preview': 'rgba(255, 255, 255, 0.80)',
        'user-bubble': 'rgba(85, 95, 231, 0.90)',
        'goose-bubble': 'rgba(0, 0, 0, 0.12)'
      },
      borderRadius: {
        lg: 'var(--radius)',
        md: 'calc(var(--radius) - 2px)',
        sm: 'calc(var(--radius) - 4px)'
      }
    }
  }
}
