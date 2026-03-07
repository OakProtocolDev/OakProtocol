/** @type {import('tailwindcss').Config} */
module.exports = {
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        oak: {
          bg: { DEFAULT: "#050807", elevated: "#0c1210", card: "#0f1613", hover: "#142118" },
          border: { DEFAULT: "#1a2520", muted: "#0f1613" },
          text: { primary: "#fafafa", secondary: "#a1a1aa", muted: "#71717a" },
          accent: { DEFAULT: "#22c55e", hover: "#16a34a", muted: "rgba(34, 197, 94, 0.12)" },
          error: "#ef4444",
          warning: "#f59e0b",
        },
      },
      fontFamily: {
        sans: ["var(--font-inter)", "Inter", "system-ui", "sans-serif"],
        mono: ["var(--font-mono)", "ui-monospace", "monospace"],
      },
      borderRadius: { oak: "12px", "oak-lg": "16px" },
      boxShadow: {
        oak: "0 4px 24px rgba(0, 0, 0, 0.4)",
        "oak-glow": "0 0 40px rgba(34, 197, 94, 0.08)",
      },
    },
  },
};
