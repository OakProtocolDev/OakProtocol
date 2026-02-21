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
        // Dark theme palette inspired by GMX / Aave
        oak: {
          bg: {
            DEFAULT: "#0a0a0b",
            elevated: "#121214",
            card: "#161618",
            hover: "#1c1c1f",
          },
          border: {
            DEFAULT: "#2a2a2e",
            muted: "#1e1e21",
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
        "oak": "12px",
        "oak-lg": "16px",
      },
      boxShadow: {
        "oak": "0 4px 24px rgba(0, 0, 0, 0.4)",
        "oak-glow": "0 0 40px rgba(34, 197, 94, 0.08)",
      },
      transitionDuration: {
        "oak": "150ms",
      },
      animation: {
        "fade-in": "fadeIn 0.2s ease-out",
      },
      keyframes: {
        fadeIn: {
          "0%": { opacity: "0" },
          "100%": { opacity: "1" },
        },
      },
    },
  },
  plugins: [],
};

export default config;
