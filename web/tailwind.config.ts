import type { Config } from "tailwindcss";

const config: Config = {
  content: [
    "./app/**/*.{js,ts,jsx,tsx,mdx}",
    "./components/**/*.{js,ts,jsx,tsx,mdx}",
  ],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        oak: {
          bg: {
            DEFAULT: "#050807",
            elevated: "#0c1210",
            card: "#0f1613",
            hover: "#142118",
          },
          border: {
            DEFAULT: "#1a2520",
            muted: "#0f1613",
          },
          text: {
            primary: "#fafafa",
            secondary: "#a1a1aa",
            muted: "#71717a",
          },
          accent: {
            DEFAULT: "#22c55e",
            hover: "#16a34a",
            muted: "rgba(34, 197, 94, 0.12)",
          },
          error: "#ef4444",
          warning: "#f59e0b",
        },
      },
      fontFamily: {
        sans: ["system-ui", "ui-sans-serif", "sans-serif"],
        mono: ["ui-monospace", "monospace"],
      },
      borderRadius: {
        oak: "12px",
        "oak-lg": "16px",
      },
      boxShadow: {
        oak: "0 4px 24px rgba(0, 0, 0, 0.4)",
        "oak-glow": "0 0 40px rgba(34, 197, 94, 0.08)",
        "oak-glow-strong": "0 0 24px rgba(34, 197, 94, 0.2)",
        "oak-glass": "0 8px 32px rgba(0, 0, 0, 0.4), 0 2px 8px rgba(0, 0, 0, 0.2)",
      },
      transitionDuration: {
        oak: "150ms",
      },
      animation: {
        "fade-in": "fadeIn 0.3s ease-out",
        shimmer: "shimmer 3s ease-in-out infinite",
        "pulse-glow": "pulseGlow 2s ease-in-out infinite",
      },
      keyframes: {
        fadeIn: {
          "0%": { opacity: "0" },
          "100%": { opacity: "1" },
        },
        shimmer: {
          "0%": { transform: "translateX(-100%)" },
          "100%": { transform: "translateX(200%)" },
        },
        pulseGlow: {
          "0%, 100%": { opacity: "1", boxShadow: "0 0 8px rgba(34, 197, 94, 0.6), 0 0 16px rgba(34, 197, 94, 0.3)" },
          "50%": { opacity: "0.8", boxShadow: "0 0 12px rgba(34, 197, 94, 0.8), 0 0 24px rgba(34, 197, 94, 0.4)" },
        },
      },
      backdropBlur: {
        xs: "2px",
      },
    },
  },
  plugins: [],
};

export default config;
