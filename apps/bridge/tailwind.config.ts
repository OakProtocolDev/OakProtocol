import type { Config } from "tailwindcss";
import uiPreset from "../../packages/ui/tailwind.preset.js";

const config: Config = {
  content: ["./app/**/*.{js,ts,jsx,tsx,mdx}", "./components/**/*.{js,ts,jsx,tsx,mdx}", "../../packages/ui/src/**/*.{js,ts,jsx,tsx,mdx}"],
  darkMode: "class",
  presets: [uiPreset],
  theme: { extend: {} },
  plugins: [],
};

export default config;
