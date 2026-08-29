/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        // Deep precision graphite / charcoal surfaces
        app: {
          bg: '#0c0e12',
          panel: '#13171f',
          surface: '#181d26',
          inset: '#090b0e',
          elevated: '#1f2532',
          border: '#262d3d',
          'border-subtle': '#1c212c',
          'border-focus': '#00d2ff',
        },
        // Legacy background aliases for compatibility
        background: {
          DEFAULT: '#0c0e12',
          subtle: '#13171f',
          card: '#181d26',
          hover: '#1f2532',
        },
        // Precision signal status colors
        signal: {
          cyan: '#00d2ff',
          'cyan-muted': '#0284c7',
          'cyan-dim': 'rgba(0, 210, 255, 0.12)',
          green: '#10b981',
          'green-muted': '#059669',
          'green-dim': 'rgba(16, 185, 129, 0.12)',
          amber: '#f59e0b',
          'amber-muted': '#b45309',
          'amber-dim': 'rgba(245, 158, 11, 0.12)',
          red: '#ef4444',
          'red-muted': '#b91c1c',
          'red-dim': 'rgba(239, 68, 68, 0.12)',
          violet: '#8b5cf6',
        },
        // Brand accents
        brand: {
          50: '#eef2ff',
          100: '#e0e7ff',
          400: '#38bdf8',
          500: '#0ea5e9',
          600: '#0284c7',
          700: '#0369a1',
          glow: 'rgba(14, 165, 233, 0.2)',
        },
        // Typography tokens
        ink: {
          100: '#f8fafc',
          200: '#e2e8f0',
          300: '#94a3b8',
          400: '#64748b',
          500: '#475569',
          600: '#334155',
        }
      },
      fontFamily: {
        sans: ['Segoe UI Variable Text', 'Segoe UI', 'Inter', '-apple-system', 'BlinkMacSystemFont', 'sans-serif'],
        mono: ['JetBrains Mono', 'Consolas', 'Cascadia Code', 'Fira Code', 'monospace'],
      },
      borderRadius: {
        'xs': '4px',
        'sm': '6px',
        'md': '8px',
        'lg': '10px',
        'xl': '12px',
      },
      animation: {
        'signal-pulse': 'pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        'spin-slow': 'spin 6s linear infinite',
      }
    },
  },
  plugins: [],
}
